//! `GET|HEAD /_ping` (docs/CONTRACTS.md §7): `204`, for the terminal's `ping`.

use std::time::Instant;

use axum::body::Body;
use axum::extract::Request;
use axum::response::Response;
use http::header::CACHE_CONTROL;
use http::{HeaderValue, StatusCode};

use super::{GET_HEAD, RouteName, is_read, mark, method_not_allowed};

pub(crate) async fn handler(req: Request) -> Response {
    let routed = Instant::now();
    let response = if is_read(req.method()) {
        let mut response = Response::new(Body::empty());
        *response.status_mut() = StatusCode::NO_CONTENT;
        response.headers_mut().insert(CACHE_CONTROL, HeaderValue::from_static("no-store"));
        response
    } else {
        method_not_allowed(GET_HEAD)
    };
    mark(response, RouteName::Named("/_ping"), routed)
}
