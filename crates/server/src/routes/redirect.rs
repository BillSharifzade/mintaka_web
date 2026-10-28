//! The `--redirect-http` listener: every request gets `301` to `https://` on the same host (and
//! the TLS listener's port, unless it is 443), path and query kept.

use std::sync::Arc;
use std::time::Instant;

use axum::body::Body;
use axum::extract::{Request, State};
use axum::response::Response;
use http::header::{HOST, LOCATION};
use http::uri::Authority;
use http::{HeaderValue, StatusCode};

use super::{RouteName, internal_error, mark};

/// Where redirects point.
pub(crate) struct RedirectTarget {
    /// Port of the TLS listener.
    pub https_port: u16,
    /// Host used when the request names none (or an unusable one): the manifest origin's host.
    pub fallback_host: String,
}

pub(crate) async fn handler(State(target): State<Arc<RedirectTarget>>, req: Request) -> Response {
    let routed = Instant::now();
    let requested = req
        .uri()
        .authority()
        .cloned()
        .or_else(|| req.headers().get(HOST)?.to_str().ok()?.parse::<Authority>().ok());
    let host = requested
        .as_ref()
        .map(Authority::host)
        .filter(|h| is_hostname(h))
        .unwrap_or(&target.fallback_host);
    let port = if target.https_port == 443 { String::new() } else { format!(":{}", target.https_port) };
    let path = req.uri().path_and_query().map_or("/", |pq| pq.as_str());
    let response = match HeaderValue::try_from(format!("https://{host}{port}{path}")) {
        Ok(location) => {
            let mut response = Response::new(Body::empty());
            *response.status_mut() = StatusCode::MOVED_PERMANENTLY;
            response.headers_mut().insert(LOCATION, location);
            response
        }
        Err(_) => internal_error(),
    };
    mark(response, RouteName::Named("redirect to https"), routed)
}

/// A DNS name or an IP literal (`[::1]` included); nothing that could change the URL's meaning.
fn is_hostname(host: &str) -> bool {
    !host.is_empty()
        && host.len() <= 253
        && host.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'.' | b'[' | b']' | b':'))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hostnames() {
        assert!(is_hostname("mintaka.tj"));
        assert!(is_hostname("[::1]"));
        assert!(!is_hostname("evil.com/x"));
        assert!(!is_hostname("a@b"));
        assert!(!is_hostname(""));
    }
}
