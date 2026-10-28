//! Binding the listeners, then serving until a shutdown signal: stop accepting, end event
//! streams, drain in-flight requests for up to the grace period.

use std::future::Future;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{Context, bail};
use hyper_util::server::graceful::GracefulShutdown;
use proof::TelemetryHello;
use rustls::ServerConfig;
use tokio::net::TcpListener;
use tokio::sync::watch;

use crate::app::{AppState, SOFTWARE, ServerInfo};
use crate::assets::Site;
use crate::config::Config;
use crate::conn::{self, Listener};
use crate::contact::Contact;
use crate::headers::SecurityHeaders;
use crate::pow::Pow;
use crate::routes::{self, RedirectTarget};
use crate::telemetry::{self, Metrics, StreamLimit};
use crate::tls::{self, SelfSigned};

/// A bound, ready-to-run server. Everything that can fail at startup (manifest, files, TLS
/// material, data directory, ports) has been checked by [`Server::bind`].
pub struct Server {
    app: Arc<AppState>,
    shutdown: watch::Sender<bool>,
    grace: Duration,
    http: Option<TcpListener>,
    https: Option<(TcpListener, Arc<ServerConfig>)>,
    redirect: Option<(TcpListener, RedirectTarget)>,
    self_signed: Option<SelfSigned>,
}

impl Server {
    /// Loads `dist/` into memory, prepares TLS and contact storage, binds every listener.
    pub async fn bind(config: Config) -> anyhow::Result<Server> {
        let site = Site::load(&config.dist)?;
        let security = SecurityHeaders::new(&site.csp).context("manifest `csp`")?;
        let cores = std::thread::available_parallelism().map_or(1, |n| u32::try_from(n.get()).unwrap_or(u32::MAX));
        let tls_setup = config.tls.as_ref().map(|t| tls::server_config(&t.source, t.allow_tls12)).transpose()?;
        let contact = Contact::new(&config.data_dir, config.contact_recipient.as_deref(), config.telegram.as_ref())?;
        let hello = telemetry::hello_frame(&TelemetryHello {
            build: site.build.clone(),
            cores,
            region: config.region.clone(),
        });
        let fallback_host = site
            .origin
            .parse::<http::Uri>()
            .ok()
            .and_then(|u| u.host().map(str::to_owned))
            .unwrap_or_else(|| "localhost".to_owned());
        tracing::info!(
            build = %site.build,
            routes = site.route_count(),
            bytes = site.memory_bytes,
            "loaded {} into memory",
            config.dist.display()
        );
        tracing::info!("{SOFTWARE}");
        tracing::info!(
            recipient = %contact.store.recipient(),
            dir = %config.data_dir.join("contact").display(),
            telegram = contact.notifier.is_some(),
            "contact briefs are age-encrypted at rest"
        );

        let (shutdown, shutdown_rx) = watch::channel(false);
        let app = Arc::new(AppState {
            site,
            security,
            info: ServerInfo {
                region: config.region.clone(),
                location: config.location.clone(),
                started: Instant::now(),
                cores,
            },
            metrics: Metrics::new(cores as usize)?,
            streams: StreamLimit::new(config.max_telemetry_streams),
            pow: Pow::new(config.pow_difficulty),
            contact,
            hello,
            shutdown: shutdown_rx,
        });

        let http = match config.listen {
            Some(addr) => Some(listen(addr, "--listen").await?),
            None => None,
        };
        let (https, self_signed) = match (&config.tls, tls_setup) {
            (Some(settings), Some(setup)) => {
                let listener = listen(settings.listen, "--tls-listen").await?;
                (Some((listener, setup.config)), setup.self_signed)
            }
            _ => (None, None),
        };
        let redirect = match config.redirect_http {
            Some(addr) => {
                let Some((tls_listener, _)) = &https else { bail!("--redirect-http needs a TLS listener") };
                let https_port = tls_listener.local_addr()?.port();
                Some((listen(addr, "--redirect-http").await?, RedirectTarget { https_port, fallback_host }))
            }
            None => None,
        };
        if http.is_none() && https.is_none() {
            bail!("nothing to listen on");
        }

        if let Some(l) = &http {
            tracing::info!("listening on http://{} (plain HTTP: development)", l.local_addr()?);
        }
        if let (Some((l, _)), Some(settings)) = (&https, &config.tls) {
            let versions = if settings.allow_tls12 { "TLS 1.3 + 1.2" } else { "TLS 1.3 only" };
            tracing::info!(
                "listening on https://{} ({versions}; X25519MLKEM768, X25519, secp256r1; ALPN h2, http/1.1)",
                l.local_addr()?
            );
        }
        if let Some(ss) = &self_signed {
            tracing::warn!(
                "self-signed certificate for localhost, 127.0.0.1, ::1 — SHA-256 fingerprint {}",
                ss.fingerprint
            );
        }
        if let Some((l, target)) = &redirect {
            tracing::info!("redirecting http://{} to https (port {})", l.local_addr()?, target.https_port);
        }
        Ok(Server { app, shutdown, grace: config.shutdown_grace, http, https, redirect, self_signed })
    }

    /// Address of the plain-HTTP listener.
    pub fn http_addr(&self) -> Option<SocketAddr> {
        self.http.as_ref().and_then(|l| l.local_addr().ok())
    }

    /// Address of the TLS listener.
    pub fn https_addr(&self) -> Option<SocketAddr> {
        self.https.as_ref().and_then(|(l, _)| l.local_addr().ok())
    }

    /// Address of the HTTP → HTTPS redirect listener.
    pub fn redirect_addr(&self) -> Option<SocketAddr> {
        self.redirect.as_ref().and_then(|(l, _)| l.local_addr().ok())
    }

    /// The certificate generated by `--tls-self-signed`.
    pub fn self_signed(&self) -> Option<&SelfSigned> {
        self.self_signed.as_ref()
    }

    /// Serves until `signal` completes, then shuts down gracefully.
    pub async fn run(self, signal: impl Future<Output = ()>) -> anyhow::Result<()> {
        let Server { app, shutdown, grace, http, https, redirect, .. } = self;
        let sampler = tokio::spawn(telemetry::run_sampler(Arc::clone(&app), app.shutdown.clone()));
        let builder = Arc::new(conn::http_builder());
        let graceful = GracefulShutdown::new();
        let site = routes::site(Arc::clone(&app));
        let listener = |router: axum::Router, tls: Option<Arc<ServerConfig>>| Listener {
            app: Arc::clone(&app),
            router,
            tls,
            http: Arc::clone(&builder),
        };
        {
            let mut loops = Vec::new();
            if let Some(l) = http {
                loops.push(conn::accept_loop(l, listener(site.clone(), None), &graceful));
            }
            if let Some((l, config)) = https {
                loops.push(conn::accept_loop(l, listener(site.clone(), Some(config)), &graceful));
            }
            if let Some((l, target)) = redirect {
                loops.push(conn::accept_loop(l, listener(routes::redirect(target), None), &graceful));
            }
            tokio::select! {
                _ = futures_util::future::join_all(loops) => {}
                () = signal => {}
            }
            // Leaving this block drops the accept loops, and with them the listeners.
        }
        let _ = shutdown.send(true);
        tracing::info!(
            connections = graceful.count(),
            "shutting down: no longer accepting; draining in-flight requests for up to {grace:?}"
        );
        match tokio::time::timeout(grace, graceful.shutdown()).await {
            Ok(()) => tracing::info!("all connections closed"),
            Err(_) => tracing::warn!("grace period over; dropping the remaining connections"),
        }
        let _ = sampler.await;
        Ok(())
    }
}

async fn listen(addr: SocketAddr, flag: &str) -> anyhow::Result<TcpListener> {
    TcpListener::bind(addr).await.with_context(|| format!("cannot listen on {addr} ({flag})"))
}
