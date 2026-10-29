//! Binding the listeners, then serving until a shutdown signal: stop accepting, end event
//! streams, drain in-flight requests (TCP and QUIC) for up to the grace period.

use std::future::Future;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{Context, bail};
use hyper_util::server::graceful::GracefulShutdown;
use proof::TelemetryHello;
use tokio::net::TcpListener;
use tokio::sync::{mpsc, watch};

use crate::acme::Acme;
use crate::app::{AppState, SOFTWARE, ServerInfo};
use crate::assets::Site;
use crate::config::Config;
use crate::conn::{self, Listener};
use crate::contact::Contact;
use crate::ctf::Ctf;
use crate::headers::{SecurityHeaders, static_value};
use crate::pow::Pow;
use crate::quic::{self, QuicListener};
use crate::routes::{self, RedirectTarget};
use crate::rum::Rum;
use crate::telemetry::{self, Metrics, StreamLimit};
use crate::tls::{self, SelfSigned, TcpTls};

/// A bound, ready-to-run server. Everything that can fail at startup (manifest, files, TLS
/// material, data directory, ports) has been checked by [`Server::bind`].
pub struct Server {
    app: Arc<AppState>,
    shutdown: watch::Sender<bool>,
    grace: Duration,
    http: Option<TcpListener>,
    https: Option<(TcpListener, TcpTls)>,
    /// HTTP/3, on the TLS listener's address.
    quic: Option<quinn::Endpoint>,
    redirect: Option<(TcpListener, RedirectTarget)>,
    self_signed: Option<SelfSigned>,
    acme: Option<Arc<Acme>>,
}

impl Server {
    /// Loads `dist/` into memory, prepares TLS and contact storage, binds every listener.
    pub async fn bind(config: Config) -> anyhow::Result<Server> {
        let site = Site::load(&config.dist)?;
        let security = SecurityHeaders::new(&site.csp).context("manifest `csp`")?;
        let cores = std::thread::available_parallelism().map_or(1, |n| u32::try_from(n.get()).unwrap_or(u32::MAX));
        let tls_setup = config.tls.as_ref().map(tls::setup).transpose()?;
        let contact = Contact::new(&config.data_dir, config.contact_recipient.as_deref(), config.telegram.as_ref())?;
        let ctf = Ctf::open(&config.data_dir)?;
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

        let rum = Rum::new(site.page_paths())?;
        let http = match config.listen {
            Some(addr) => Some(listen(addr, "--listen").await?),
            None => None,
        };
        let (https, quic, self_signed, acme) = match (&config.tls, tls_setup) {
            (Some(settings), Some(setup)) => {
                let listener = listen(settings.listen, "--tls-listen").await?;
                // Same address and port as TCP (the kernel keeps the two protocols apart).
                let quic = match setup.quic {
                    Some(quic_tls) => Some(quic::endpoint(listener.local_addr()?, quic_tls)?),
                    None => None,
                };
                (Some((listener, setup.tcp)), quic, setup.self_signed, setup.acme)
            }
            _ => (None, None, None, None),
        };
        let alt_svc = match (&quic, config.tls.as_ref().and_then(|t| t.http3.as_ref())) {
            (Some(endpoint), Some(h3)) => {
                let port = h3.advertise_port.unwrap_or(endpoint.local_addr()?.port());
                Some(static_value(format!("h3=\":{port}\"; ma=86400"))?)
            }
            _ => None,
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
            ctf,
            rum,
            hello,
            alt_svc,
            shutdown: shutdown_rx,
        });


        if let Some(l) = &http {
            tracing::info!("listening on http://{} (plain HTTP: development)", l.local_addr()?);
        }
        if let (Some((l, _)), Some(settings)) = (&https, &config.tls) {
            let versions = if settings.allow_tls12 { "TLS 1.3 + 1.2" } else { "TLS 1.3 only" };
            tracing::info!(
                "listening on https://{} ({versions}; X25519MLKEM768, X25519, secp256r1; ALPN h2, http/1.1)",
                l.local_addr()?
            );
            if let (Some(endpoint), Some(alt_svc)) = (&quic, &app.alt_svc) {
                tracing::info!(
                    "HTTP/3 on udp {} (TLS 1.3; ALPN h3), announced as Alt-Svc: {}",
                    endpoint.local_addr()?,
                    alt_svc.to_str().unwrap_or("?")
                );
            }
            if acme.is_some() && l.local_addr()?.port() != 443 {
                tracing::warn!(
                    target: "acme",
                    "Let's Encrypt validates TLS-ALPN-01 on port 443 only: make sure public port 443 reaches this listener"
                );
            }
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
        Ok(Server { app, shutdown, grace: config.shutdown_grace, http, https, quic, redirect, self_signed, acme })
    }

    /// Address of the plain-HTTP listener.
    pub fn http_addr(&self) -> Option<SocketAddr> {
        self.http.as_ref().and_then(|l| l.local_addr().ok())
    }

    /// Address of the TLS listener.
    pub fn https_addr(&self) -> Option<SocketAddr> {
        self.https.as_ref().and_then(|(l, _)| l.local_addr().ok())
    }

    /// Address of the HTTP/3 (UDP) endpoint.
    pub fn http3_addr(&self) -> Option<SocketAddr> {
        self.quic.as_ref().and_then(|e| e.local_addr().ok())
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
        let Server { app, shutdown, grace, http, https, quic, redirect, acme, .. } = self;
        let sampler = tokio::spawn(telemetry::run_sampler(Arc::clone(&app), app.shutdown.clone()));
        let renewal = acme.map(|acme| tokio::spawn(acme.run(app.shutdown.clone())));
        let builder = Arc::new(conn::http_builder());
        let graceful = GracefulShutdown::new();
        let site = routes::site(Arc::clone(&app));
        // Every QUIC connection task holds a clone of `drain`; `drained` sees the last one go.
        let (drain, mut drained) = mpsc::channel::<()>(1);
        let quic_listener = quic.clone().map(|endpoint| (endpoint, QuicListener { app: Arc::clone(&app), router: site.clone() }));
        let listener = |router: axum::Router, tls: Option<TcpTls>| Listener {
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
            let quic_loop = async move {
                if let Some((endpoint, ctx)) = quic_listener {
                    quic::accept_loop(endpoint, ctx, drain).await;
                }
                // HTTP/3 failing on its own leaves TCP serving; only the signal stops the server.
                std::future::pending::<()>().await
            };
            tokio::select! {
                _ = futures_util::future::join_all(loops) => {}
                () = quic_loop => {}
                () = signal => {}
            }
            // Leaving this block drops the accept loops, and with them the TCP listeners.
        }
        let _ = shutdown.send(true);
        if let Some(endpoint) = &quic {
            // New QUIC connections are refused from here on.
            endpoint.set_server_config(None);
        }
        let quic_open = quic.as_ref().map_or(0, quinn::Endpoint::open_connections);
        tracing::info!(
            connections = graceful.count() + quic_open,
            "shutting down: no longer accepting; draining in-flight requests for up to {grace:?}"
        );
        let deadline = tokio::time::Instant::now() + grace;
        let all_closed = async {
            graceful.shutdown().await;
            let _ = drained.recv().await;
        };
        match tokio::time::timeout_at(deadline, all_closed).await {
            Ok(()) => tracing::info!("all connections closed"),
            Err(_) => tracing::warn!("grace period over; dropping the remaining connections"),
        }
        if let Some(endpoint) = quic {
            // CONNECTION_CLOSE to anything still open, and a moment for those packets to leave.
            endpoint.close(quic::H3_NO_ERROR, b"server shutting down");
            let flush = deadline.max(tokio::time::Instant::now() + Duration::from_millis(50));
            let _ = tokio::time::timeout_at(flush, endpoint.wait_idle()).await;
        }
        if let Some(renewal) = renewal {
            let _ = renewal.await;
        }
        let _ = sampler.await;
        Ok(())
    }
}

async fn listen(addr: SocketAddr, flag: &str) -> anyhow::Result<TcpListener> {
    TcpListener::bind(addr).await.with_context(|| format!("cannot listen on {addr} ({flag})"))
}
