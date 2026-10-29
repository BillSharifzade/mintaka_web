//! `GET /_proof` (docs/CONTRACTS.md §5): the facts of this very request, as JSON.

use std::sync::Arc;
use std::time::Instant;

use axum::extract::{Request, State};
use axum::response::Response;
use http::{StatusCode, Version};
use proof::{ConnectionFacts, HttpFacts, PrivacyFacts, Proof, ServerFacts, TlsFacts};

use super::{GET_HEAD, RouteName, alt_svc, ctx, internal_error, is_read, json, mark, method_not_allowed};
use crate::app::{AppState, SOFTWARE};
use crate::conn::ReqCtx;
use crate::headers::micros;

pub(crate) async fn handler(State(app): State<Arc<AppState>>, req: Request) -> Response {
    let routed = Instant::now();
    let response = match ctx(&req) {
        _ if !is_read(req.method()) => method_not_allowed(GET_HEAD),
        Some(ctx) => {
            let mut facts = collect(&app, &ctx, req.version());
            facts.server.time_us = micros(ctx.t0.elapsed());
            json(StatusCode::OK, &facts)
        }
        None => internal_error(),
    };
    mark(response, RouteName::Named("/_proof"), routed)
}

/// Everything `/_proof` and curl's `/proof` report. `server.time_us` is left for the caller to
/// set as late as possible.
pub(crate) fn collect(app: &AppState, ctx: &ReqCtx, version: Version) -> Proof {
    let tls = ctx.conn.tls.as_ref();
    Proof {
        server: ServerFacts {
            region: app.info.region.clone(),
            location: app.info.location.clone(),
            build: app.site.build.clone(),
            software: SOFTWARE.to_owned(),
            time_us: micros(ctx.t0.elapsed()),
            uptime_s: app.info.started.elapsed().as_secs(),
            cores: app.info.cores,
        },
        tls: tls.map(|t| TlsFacts {
            version: t.version.to_owned(),
            cipher: t.cipher.map(str::to_owned),
            group: t.group.map(str::to_owned),
            post_quantum: t.post_quantum,
            alpn: t.alpn.map(str::to_owned),
            handshake_us: t.handshake.map(micros),
        }),
        http: HttpFacts { version: http_version(version).to_owned() },
        connection: ConnectionFacts {
            id: ctx.conn.id,
            requests: ctx.seq,
            kernel_rtt_us: ctx.rtt.map(|r| r.rtt_us),
            kernel_rttvar_us: ctx.rtt.map(|r| r.rttvar_us),
            quic_rtt_us: ctx.quic_rtt.map(|d| u32::try_from(micros(d)).unwrap_or(u32::MAX)),
        },
        privacy: PrivacyFacts {
            // No code path in this server sets a cookie (tests check every response type).
            cookies_set: 0,
            third_party_origins: app.security.third_party_origins(),
            // Logs carry method, route, status and duration only. The rate limiters keep a /24 or
            // /48 prefix in memory (never the address, never on disk); idle ones are dropped once
            // 10,000 networks are tracked, and all of them on restart.
            ip_stored: false,
            ip_logged: false,
        },
        // What `routes::finish` will send on this very response: the security set, and
        // `alt-svc` when HTTP/3 is announced.
        headers: app
            .security
            .names(tls.is_some())
            .chain(alt_svc(app, ctx).map(|_| "alt-svc"))
            .map(str::to_owned)
            .collect(),
    }
}

/// `HTTP/2.0`, `HTTP/1.1` (the `Debug` form of [`Version`], spelled out).
pub(crate) fn http_version(version: Version) -> &'static str {
    match version {
        Version::HTTP_09 => "HTTP/0.9",
        Version::HTTP_10 => "HTTP/1.0",
        Version::HTTP_11 => "HTTP/1.1",
        Version::HTTP_2 => "HTTP/2.0",
        Version::HTTP_3 => "HTTP/3.0",
        _ => "unknown",
    }
}
