//! Bodies across the h3 boundary. Request bodies reach handlers as an `http_body::Body` that
//! reads the QUIC stream on demand (so `Limited` in the contact handler stops reading at its
//! limit); response bodies leave frame by frame as the handler produces them (so `/_telemetry`
//! events are sent the moment they exist).

use std::pin::Pin;
use std::task::{Context, Poll, ready};

use axum::response::Response;
use bytes::{Buf, Bytes};
use h3::error::{Code, StreamError};
use h3::server::RequestStream;
use http::header::{CONNECTION, CONTENT_LENGTH, HeaderName, TRANSFER_ENCODING, UPGRADE};
use http::{HeaderValue, Method, StatusCode};
use http_body::{Body, Frame};
use http_body_util::BodyExt;

/// The receiving half of an HTTP/3 request stream.
pub(crate) struct RequestBody {
    recv: RequestStream<h3_quinn::RecvStream, Bytes>,
    state: State,
}

enum State {
    Data,
    Trailers,
    Done,
}

impl RequestBody {
    pub fn new(recv: RequestStream<h3_quinn::RecvStream, Bytes>) -> Self {
        RequestBody { recv, state: State::Data }
    }
}

impl Body for RequestBody {
    type Data = Bytes;
    type Error = StreamError;

    fn poll_frame(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Result<Frame<Bytes>, StreamError>>> {
        let this = &mut *self;
        loop {
            match this.state {
                State::Data => match ready!(this.recv.poll_recv_data(cx)) {
                    Ok(Some(mut data)) => return Poll::Ready(Some(Ok(Frame::data(data.copy_to_bytes(data.remaining()))))),
                    Ok(None) => this.state = State::Trailers,
                    Err(e) => {
                        this.state = State::Done;
                        return Poll::Ready(Some(Err(e)));
                    }
                },
                State::Trailers => {
                    let trailers = ready!(this.recv.poll_recv_trailers(cx));
                    this.state = State::Done;
                    return Poll::Ready(match trailers {
                        Ok(Some(trailers)) => Some(Ok(Frame::trailers(trailers))),
                        Ok(None) => None,
                        Err(e) => Some(Err(e)),
                    });
                }
                State::Done => return Poll::Ready(None),
            }
        }
    }

    fn is_end_stream(&self) -> bool {
        matches!(self.state, State::Done)
    }
}

/// Connection-specific fields, which HTTP/3 forbids (RFC 9114 §4.2). Our handlers never set
/// them; this keeps one stray header from failing a whole response.
const HOP_BY_HOP: [HeaderName; 5] = [
    CONNECTION,
    TRANSFER_ENCODING,
    UPGRADE,
    HeaderName::from_static("keep-alive"),
    HeaderName::from_static("proxy-connection"),
];

/// Sends a finished response: the head, then each body frame as it comes, then FIN. No body
/// for `HEAD`, 1xx, 204 and 304 — the job hyper does for HTTP/1.1 and HTTP/2.
pub(crate) async fn send(
    mut stream: RequestStream<h3_quinn::SendStream<Bytes>, Bytes>,
    method: &Method,
    response: Response,
) -> Result<(), StreamError> {
    let (mut parts, body) = response.into_parts();
    for name in HOP_BY_HOP {
        parts.headers.remove(name);
    }
    let status = parts.status;
    let bodiless = method == Method::HEAD
        || status.is_informational()
        || status == StatusCode::NO_CONTENT
        || status == StatusCode::NOT_MODIFIED;
    if !bodiless
        && !parts.headers.contains_key(CONTENT_LENGTH)
        && let Some(len) = body.size_hint().exact()
    {
        parts.headers.insert(CONTENT_LENGTH, HeaderValue::from(len));
    }
    stream.send_response(Response::from_parts(parts, ())).await?;
    if !bodiless {
        let mut body = std::pin::pin!(body);
        while let Some(frame) = body.frame().await {
            let frame = match frame {
                Ok(frame) => frame,
                Err(e) => {
                    tracing::debug!("HTTP/3 response body failed: {e}");
                    stream.stop_stream(Code::H3_INTERNAL_ERROR);
                    return Ok(());
                }
            };
            match frame.into_data() {
                Ok(data) if data.is_empty() => {}
                Ok(data) => stream.send_data(data).await?,
                Err(frame) => {
                    if let Ok(trailers) = frame.into_trailers() {
                        stream.send_trailers(trailers).await?;
                    }
                }
            }
        }
    }
    stream.finish().await
}
