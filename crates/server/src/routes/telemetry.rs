//! `GET /_telemetry` (docs/CONTRACTS.md §6): Server-Sent Events. `event: hello`, the latest
//! `event: tick` right away, then one tick per second and a `: ping` comment every 15 s. At most
//! [`crate::Config::max_telemetry_streams`] at once; beyond that, `503`.

use std::convert::Infallible;
use std::sync::Arc;
use std::time::{Duration, Instant};

use axum::body::Body;
use axum::extract::{Request, State};
use axum::response::Response;
use bytes::Bytes;
use http::header::{CACHE_CONTROL, CONTENT_TYPE, RETRY_AFTER};
use http::{HeaderValue, StatusCode};
use tokio::sync::watch;

use super::{GET_HEAD, Handled, RouteName, is_read, mark, method_not_allowed, no_store};
use crate::app::AppState;
use crate::telemetry::StreamPermit;

const PING: Bytes = Bytes::from_static(b": ping\n\n");
const PING_EVERY: Duration = Duration::from_secs(15);

pub(crate) async fn handler(State(app): State<Arc<AppState>>, req: Request) -> Response {
    let routed = Instant::now();
    let name = RouteName::Named("/_telemetry");
    if !is_read(req.method()) {
        return mark(method_not_allowed(GET_HEAD), name, routed);
    }
    let Some(permit) = app.streams.try_acquire() else {
        let mut response = no_store(StatusCode::SERVICE_UNAVAILABLE, "text/plain; charset=utf-8", "too many telemetry streams, try again shortly\n");
        response.headers_mut().insert(RETRY_AFTER, HeaderValue::from_static("15"));
        return mark(response, name, routed);
    };

    let mut ticks = app.metrics.subscribe();
    let latest = ticks.borrow_and_update().clone();
    let stream = Stream {
        queued: [Some(app.hello.clone()), latest],
        ticks,
        shutdown: app.shutdown.clone(),
        ping: tokio::time::interval_at(tokio::time::Instant::now() + PING_EVERY, PING_EVERY),
        _permit: permit,
    };
    let body = futures_util::stream::unfold(stream, Stream::next);
    let mut response = Response::new(Body::from_stream(body));
    let h = response.headers_mut();
    h.insert(CONTENT_TYPE, HeaderValue::from_static("text/event-stream"));
    h.insert(CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response.extensions_mut().insert(Handled { route: name, routed, long_lived: true });
    response
}

/// One subscriber's state.
struct Stream {
    /// Frames to send before waiting: hello, then the latest tick if there is one.
    queued: [Option<Bytes>; 2],
    ticks: watch::Receiver<Option<Bytes>>,
    shutdown: watch::Receiver<bool>,
    ping: tokio::time::Interval,
    /// Holds this stream's slot until the client goes away.
    _permit: StreamPermit,
}

impl Stream {
    async fn next(mut self) -> Option<(Result<Bytes, Infallible>, Stream)> {
        if let Some(frame) = self.queued.iter_mut().find_map(Option::take) {
            return Some((Ok(frame), self));
        }
        loop {
            if *self.shutdown.borrow() {
                return None;
            }
            tokio::select! {
                changed = self.shutdown.changed() => {
                    if changed.is_err() || *self.shutdown.borrow() {
                        return None;
                    }
                }
                changed = self.ticks.changed() => {
                    changed.ok()?;
                    let frame = self.ticks.borrow_and_update().clone();
                    if let Some(frame) = frame {
                        return Some((Ok(frame), self));
                    }
                }
                _ = self.ping.tick() => return Some((Ok(PING), self)),
            }
        }
    }
}
