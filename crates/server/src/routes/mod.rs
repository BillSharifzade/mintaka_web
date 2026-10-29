//! The axum routers and what every response gets on the way out ([`finish`]).
//!
//! Handlers take the whole `Request` and never use rejecting extractors, so every response —
//! including 404, 405 and 413 — is built here and passes through [`finish`].

mod contact;
mod ctf;
mod curl;
mod ping;
mod proof;
mod redirect;
mod rum;
mod static_files;
mod telemetry;

use std::sync::Arc;
use std::time::{Duration, Instant};

use axum::Router;
use axum::body::Body;
use axum::response::Response;
use axum::routing::any;
use bytes::Bytes;
use http::header::{ALLOW, ALT_SVC, CACHE_CONTROL, CONTENT_LENGTH, CONTENT_TYPE};
use http::{HeaderValue, Method, StatusCode};

use crate::app::AppState;
use crate::conn::ReqCtx;
use crate::headers::{SERVER_TIMING, server_timing};

pub(crate) use redirect::RedirectTarget;

/// What a handler reports about its response to [`finish`] (a response extension).
#[derive(Clone, Copy)]
pub(crate) struct Handled {
    pub route: RouteName,
    /// When the route was known: `route` = `routed − t0`, `render` = `ready − routed`.
    pub routed: Instant,
    /// Event streams: counted, but not a latency sample.
    pub long_lived: bool,
}

/// The matched route, for the access log. Never the requested path of an unmatched request.
#[derive(Clone, Copy)]
pub(crate) enum RouteName {
    Named(&'static str),
    /// Index into the site's static routes.
    Static(usize),
}

/// Attaches [`Handled`] to a response.
pub(crate) fn mark(mut response: Response, route: RouteName, routed: Instant) -> Response {
    response.extensions_mut().insert(Handled { route, routed, long_lived: false });
    response
}

/// The router of the site listener(s).
pub(crate) fn site(app: Arc<AppState>) -> Router {
    Router::new()
        .route("/_proof", any(proof::handler))
        .route("/_ping", any(ping::handler))
        .route("/_telemetry", any(telemetry::handler))
        .route("/_rum", any(rum::handler))
        .route("/api/pow", any(contact::pow))
        .route("/api/contact", any(contact::submit))
        .route("/api/ctf/start", any(ctf::start))
        .route("/api/ctf/puzzle", any(ctf::puzzle))
        .route("/api/ctf/flag", any(ctf::flag))
        .fallback(static_files::handler)
        .with_state(app)
}

/// The router of the `--redirect-http` listener.
pub(crate) fn redirect(target: RedirectTarget) -> Router {
    Router::new().fallback(redirect::handler).with_state(Arc::new(target))
}

/// Security headers, `Server-Timing`, metrics and the access log, for every response.
pub(crate) fn finish(app: &AppState, ctx: &ReqCtx, method: &Method, response: &mut Response) {
    let handled = response.extensions_mut().remove::<Handled>();
    let status = response.status();
    let headers = response.headers_mut();
    // RFC 9110 §8.6: no Content-Length on 1xx/204; on 304 only the length of the 200 body.
    if status.is_informational() || status == StatusCode::NO_CONTENT || status == StatusCode::NOT_MODIFIED {
        headers.remove(CONTENT_LENGTH);
    }
    app.security.apply(headers, ctx.is_tls());
    if let Some(alt_svc) = alt_svc(app, ctx) {
        headers.insert(ALT_SVC, alt_svc.clone());
    }

    let ready = Instant::now();
    let total = ready.saturating_duration_since(ctx.t0);
    let (route, render, name, long_lived) = match handled {
        Some(h) => (
            h.routed.saturating_duration_since(ctx.t0),
            ready.saturating_duration_since(h.routed),
            h.route,
            h.long_lived,
        ),
        // Only axum itself answers without a handler; that is routing.
        None => (total, Duration::ZERO, RouteName::Named("(router)"), false),
    };
    let tls = if ctx.seq == 1 { ctx.conn.tls.as_ref().and_then(|t| t.handshake) } else { None };
    if let Some(value) = server_timing(total, route, render, tls, ctx.rtt, ctx.quic_rtt) {
        headers.insert(SERVER_TIMING, value);
    }

    app.metrics.record(total, long_lived);
    let route = match name {
        RouteName::Named(name) => name,
        RouteName::Static(i) => app.site.route_at(i).map_or("?", |r| r.path.as_str()),
    };
    tracing::info!(
        target: "access",
        method = %method,
        route = %route,
        status = status.as_u16(),
        us = crate::headers::micros(total),
    );
}

/// `Alt-Svc` goes on TLS responses (TCP and HTTP/3) when HTTP/3 is on; never over plain HTTP,
/// where a browser would not use it.
pub(crate) fn alt_svc<'a>(app: &'a AppState, ctx: &ReqCtx) -> Option<&'a HeaderValue> {
    app.alt_svc.as_ref().filter(|_| ctx.is_tls())
}

/// `GET` or `HEAD`.
pub(crate) fn is_read(method: &Method) -> bool {
    method == Method::GET || method == Method::HEAD
}

/// The request context the connection layer attached.
pub(crate) fn ctx(req: &axum::extract::Request) -> Option<ReqCtx> {
    req.extensions().get::<ReqCtx>().cloned()
}

pub(crate) const GET_HEAD: &str = "GET, HEAD";

/// `405` with `Allow` (and `no-store`: `/api/*` and `/_*` must never be cached, whatever the
/// status, and a 405 is not worth caching anywhere).
pub(crate) fn method_not_allowed(allow: &'static str) -> Response {
    let mut response = Response::new(Body::empty());
    *response.status_mut() = StatusCode::METHOD_NOT_ALLOWED;
    response.headers_mut().insert(ALLOW, HeaderValue::from_static(allow));
    response.headers_mut().insert(CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
}

/// A body with `Content-Type` and `Cache-Control: no-store`.
pub(crate) fn no_store(status: StatusCode, content_type: &'static str, body: impl Into<Bytes>) -> Response {
    let mut response = Response::new(Body::from(body.into()));
    *response.status_mut() = status;
    let headers = response.headers_mut();
    headers.insert(CONTENT_TYPE, HeaderValue::from_static(content_type));
    headers.insert(CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
}

/// JSON with `Cache-Control: no-store`.
pub(crate) fn json(status: StatusCode, value: &impl serde::Serialize) -> Response {
    match serde_json::to_vec(value) {
        Ok(body) => no_store(status, "application/json", body),
        Err(e) => {
            tracing::error!("serialising a response failed: {e}");
            internal_error()
        }
    }
}

/// `500`, for states that should be impossible.
pub(crate) fn internal_error() -> Response {
    no_store(StatusCode::INTERNAL_SERVER_ERROR, "text/plain; charset=utf-8", "internal error\n")
}
