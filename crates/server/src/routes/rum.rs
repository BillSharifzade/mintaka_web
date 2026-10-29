//! `POST /_rum` and `GET|HEAD /_rum` (docs/CONTRACTS.md §14): real-user Web Vitals and event
//! counts in, aggregates out. Every answer is `no-store`.
//!
//! Order of checks for a beacon: Global Privacy Control / Do Not Track (answered `204`, nothing
//! counted) → same-origin → content type → declared size → rate limit → body size → parse.
//! Refusals answer with fixed text; nothing from the request is echoed.

use std::sync::Arc;
use std::time::Instant;

use axum::body::Body;
use axum::extract::{Request, State};
use axum::response::Response;
use http::header::{CACHE_CONTROL, CONTENT_LENGTH, CONTENT_TYPE, RETRY_AFTER};
use http::{HeaderMap, HeaderValue, Method, StatusCode};
use http_body_util::{BodyExt, LengthLimitError, Limited};
use proof::rum::RUM_BODY_LIMIT;

use super::{RouteName, ctx, internal_error, mark, method_not_allowed, no_store};
use crate::app::AppState;

const TEXT: &str = "text/plain; charset=utf-8";

pub(crate) async fn handler(State(app): State<Arc<AppState>>, req: Request) -> Response {
    let routed = Instant::now();
    let response = match *req.method() {
        Method::GET | Method::HEAD => no_store(StatusCode::OK, "application/json", app.rum.summary()),
        Method::POST => receive(&app, req).await,
        _ => method_not_allowed("GET, HEAD, POST"),
    };
    mark(response, RouteName::Named("/_rum"), routed)
}

async fn receive(app: &AppState, req: Request) -> Response {
    let Some(ctx) = ctx(&req) else { return internal_error() };
    let headers = req.headers();
    if declines(headers) {
        return accepted();
    }
    // Browsers say where a request comes from; a beacon from another site is not a visitor of ours.
    if headers.get("sec-fetch-site").is_some_and(|v| v.as_bytes() != b"same-origin") {
        return refuse(app, StatusCode::FORBIDDEN, "beacon refused: not sent by this site\n");
    }
    let content_type = headers.get(CONTENT_TYPE).and_then(|v| v.to_str().ok()).unwrap_or("");
    let mime = content_type.split(';').next().unwrap_or("").trim().to_ascii_lowercase();
    if mime != "text/plain" && mime != "application/json" {
        return refuse(app, StatusCode::UNSUPPORTED_MEDIA_TYPE, "beacon refused: send text/plain or application/json\n");
    }
    let declared = headers.get(CONTENT_LENGTH).and_then(|v| v.to_str().ok()?.parse::<u64>().ok());
    if declared.is_some_and(|n| n > RUM_BODY_LIMIT as u64) {
        return refuse(app, StatusCode::PAYLOAD_TOO_LARGE, "beacon refused: larger than 2 KiB\n");
    }
    if !app.rum.admit(ctx.conn.client_net) {
        let mut response = no_store(StatusCode::TOO_MANY_REQUESTS, TEXT, "beacon refused: too many from your network, try again later\n");
        response.headers_mut().insert(RETRY_AFTER, HeaderValue::from_static("60"));
        return response;
    }
    let body = match Limited::new(req.into_body(), RUM_BODY_LIMIT).collect().await {
        Ok(collected) => collected.to_bytes(),
        Err(e) if e.downcast_ref::<LengthLimitError>().is_some() => {
            return refuse(app, StatusCode::PAYLOAD_TOO_LARGE, "beacon refused: larger than 2 KiB\n");
        }
        Err(_) => return refuse(app, StatusCode::BAD_REQUEST, "beacon refused: the body did not arrive\n"),
    };
    match app.rum.receive(&body) {
        Ok(()) => accepted(),
        Err(why) => no_store(StatusCode::BAD_REQUEST, TEXT, why.reason()),
    }
}

/// `Sec-GPC: 1` (Global Privacy Control) or `DNT: 1`: the visitor asked not to be measured.
/// The page doesn't send beacons then; this catches any client that does anyway.
fn declines(headers: &HeaderMap) -> bool {
    ["sec-gpc", "dnt"].iter().any(|name| headers.get(*name).is_some_and(|v| v.as_bytes().trim_ascii() == b"1"))
}

/// `204 No Content`, `no-store`.
fn accepted() -> Response {
    let mut response = Response::new(Body::empty());
    *response.status_mut() = StatusCode::NO_CONTENT;
    response.headers_mut().insert(CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
}

fn refuse(app: &AppState, status: StatusCode, reason: &'static str) -> Response {
    app.rum.refuse();
    no_store(status, TEXT, reason)
}
