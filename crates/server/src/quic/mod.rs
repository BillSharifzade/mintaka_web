//! HTTP/3: quinn (QUIC on UDP) → h3 → the same axum `Router`, [`conn::respond`] and
//! `routes::finish` as HTTP/1.1 and HTTP/2, so every response carries the same headers,
//! Server-Timing, metrics and access-log line.
//!
//! * TLS is our provider with TLS 1.3 only and ALPN `h3`, on the certificate resolver the TCP
//!   listener uses (so a renewed ACME certificate reaches both at once). [`handshake`] reads the
//!   cipher suite and key-exchange group back from our own ServerHello, because quinn does not
//!   expose them.
//! * A QUIC connection has no TCP socket, so `ReqCtx.rtt` (kernel `TCP_INFO`) is `None`;
//!   `ReqCtx.quic_rtt` carries quinn's own RTT estimate instead, under its own name.
//! * Limits match the TCP side: 10 s to finish the handshake, 10 s to the first request, 10 s
//!   for each request head, 256 concurrent requests per connection; plus a 30 s idle timeout.
//! * Shutdown: no new connections; on each connection a GOAWAY, the accepted requests finish
//!   (event streams end), then CONNECTION_CLOSE. The server closes whatever is left when its
//!   grace period runs out.

mod body;
pub(crate) mod handshake;

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{Context, anyhow};
use axum::Router;
use bytes::Bytes;
use h3::server::RequestResolver;
use quinn::crypto::rustls::QuicServerConfig;
use quinn::{IdleTimeout, VarInt};
use tokio::sync::{mpsc, watch};

use self::handshake::{Handshake, MeasuredConfig};
use crate::app::AppState;
use crate::conn::{self, ConnInfo, FIRST_REQUEST_TIMEOUT, HANDSHAKE_TIMEOUT, HEADER_READ_TIMEOUT, ReqCtx};
use crate::contact::ClientNet;
use crate::tls::TlsFacts;

/// Concurrent requests per connection: HTTP/2's `max_concurrent_streams` on the TCP side.
const MAX_REQUESTS: u32 = 256;
/// Client-initiated unidirectional streams: HTTP/3 needs three (control, QPACK encoder and
/// decoder); the rest is room for reserved "grease" stream types.
const MAX_UNI_STREAMS: u32 = 16;
/// A connection with no packets for this long is closed (QUIC's own keep-alive timer).
const IDLE_TIMEOUT: Duration = Duration::from_secs(30);
/// Flow-control credit for what clients send: requests are small and only the contact form has a
/// body (32 KiB at most), so this bounds memory per stream and per connection.
const STREAM_WINDOW: u32 = 64 * 1024;
const CONNECTION_WINDOW: u32 = 1024 * 1024;
/// Largest request head accepted (QPACK-decoded); larger ones get 431.
const MAX_FIELD_SECTION: u64 = 64 * 1024;
/// After the last response of a closing connection, how long to wait for the client to close
/// it: CONNECTION_CLOSE discards stream data the client has not acknowledged yet.
const LINGER: Duration = Duration::from_secs(1);
/// HTTP/3's "no error" and "internal error" application codes (RFC 9114 §8.1).
pub(crate) const H3_NO_ERROR: VarInt = VarInt::from_u32(0x100);
const H3_INTERNAL_ERROR: VarInt = VarInt::from_u32(0x102);

/// Binds the UDP socket for HTTP/3 with our transport limits.
pub(crate) fn endpoint(addr: SocketAddr, tls: Arc<rustls::ServerConfig>) -> anyhow::Result<quinn::Endpoint> {
    let crypto = QuicServerConfig::try_from(tls).map_err(|e| anyhow!("QUIC TLS configuration: {e}"))?;
    let mut config = quinn::ServerConfig::with_crypto(Arc::new(MeasuredConfig(Arc::new(crypto))));
    let mut transport = quinn::TransportConfig::default();
    transport
        .max_concurrent_bidi_streams(VarInt::from_u32(MAX_REQUESTS))
        .max_concurrent_uni_streams(VarInt::from_u32(MAX_UNI_STREAMS))
        .max_idle_timeout(Some(IdleTimeout::try_from(IDLE_TIMEOUT).context("idle timeout")?))
        .stream_receive_window(VarInt::from_u32(STREAM_WINDOW))
        .receive_window(VarInt::from_u32(CONNECTION_WINDOW))
        .keep_alive_interval(None);
    config.transport_config(Arc::new(transport));
    quinn::Endpoint::server(config, addr).with_context(|| format!("cannot listen on udp {addr} (--http3)"))
}

/// What every HTTP/3 connection needs.
#[derive(Clone)]
pub(crate) struct QuicListener {
    pub app: Arc<AppState>,
    pub router: Router,
}

/// Accepts until the endpoint is closed or the future dropped. Each connection task holds a
/// clone of `drain`, so its receiver learns when the last one has finished.
pub(crate) async fn accept_loop(endpoint: quinn::Endpoint, ctx: QuicListener, drain: mpsc::Sender<()>) {
    while let Some(incoming) = endpoint.accept().await {
        tokio::spawn(connection(incoming, ctx.clone(), drain.clone()));
    }
    // Shutdown drops this future before it gets here; reaching it means the UDP socket failed.
    tracing::error!("the HTTP/3 endpoint stopped accepting; serving HTTP/1.1 and HTTP/2 only");
}

/// One QUIC connection, from the first Initial packet to close.
async fn connection(incoming: quinn::Incoming, ctx: QuicListener, _drain: mpsc::Sender<()>) {
    let QuicListener { app, router } = ctx;
    let _gauge = app.metrics.connection_opened();
    let client_net = ClientNet::of(incoming.remote_address().ip());
    let id = conn::next_connection_id();
    let handshake = async { incoming.accept()?.await };
    let quic = match tokio::time::timeout(HANDSHAKE_TIMEOUT, handshake).await {
        Ok(Ok(quic)) => quic,
        Ok(Err(e)) => {
            tracing::debug!(conn = id, "QUIC handshake failed: {e}");
            return;
        }
        // Dropping the handshake future closes the connection.
        Err(_) => {
            tracing::debug!(conn = id, "QUIC handshake timed out");
            return;
        }
    };
    let Some(facts) = quic.handshake_data().and_then(|data| data.downcast::<Handshake>().ok()) else {
        // Impossible with our MeasuredConfig; close rather than report made-up facts.
        tracing::error!(conn = id, "QUIC connection without handshake facts; closing");
        quic.close(H3_INTERNAL_ERROR, b"");
        return;
    };
    let tls = TlsFacts::quic(
        facts.hello.map(|h| h.cipher),
        facts.hello.and_then(|h| h.group),
        facts.alpn.as_deref(),
        facts.duration,
    );
    let info = ConnInfo::new(id, Some(tls), client_net);

    let h3_conn = h3_quinn::Connection::new(quic.clone());
    let mut h3 = match h3::server::builder().max_field_section_size(MAX_FIELD_SECTION).build(h3_conn).await {
        Ok(h3) => h3,
        Err(e) => {
            tracing::debug!(conn = id, "HTTP/3 setup failed: {e}");
            return;
        }
    };

    // Each request task holds a clone; `idle` yields `None` once all of them are done.
    let (inflight, mut idle) = mpsc::channel::<()>(1);
    let mut shutdown = app.shutdown.clone();
    let first_request = tokio::time::sleep(FIRST_REQUEST_TIMEOUT);
    tokio::pin!(first_request);
    // Set here, not from the request task's count: `select!` checks the guard when it is
    // entered, and one long request (an event stream) never re-enters it.
    let mut opened_a_request = false;
    loop {
        tokio::select! {
            accepted = h3.accept() => match accepted {
                Ok(Some(resolver)) => {
                    opened_a_request = true;
                    let task = Request { quic: quic.clone(), info: Arc::clone(&info), app: Arc::clone(&app), router: router.clone() };
                    tokio::spawn(task.run(resolver, inflight.clone()));
                }
                // The client sent GOAWAY; finish up like a shutdown.
                Ok(None) => break,
                // Closed by the client, idle timeout, or a protocol error: nothing left to send.
                Err(e) => {
                    if !e.is_h3_no_error() {
                        tracing::debug!(conn = id, "HTTP/3 connection ended: {e}");
                    }
                    return;
                }
            },
            () = &mut first_request, if !opened_a_request => {
                tracing::debug!(conn = id, "no request within {FIRST_REQUEST_TIMEOUT:?}; closing");
                quic.close(H3_NO_ERROR, b"");
                return;
            }
            () = stopping(&mut shutdown) => break,
        }
    }

    // RFC 9114 §5.2: GOAWAY, let the accepted requests finish, then close. `h3` stays alive
    // meanwhile: dropping it closes the connection.
    let _ = h3.shutdown(0).await;
    drop(inflight);
    let _ = idle.recv().await;
    let _ = tokio::time::timeout(LINGER, quic.closed()).await;
    quic.close(H3_NO_ERROR, b"");
}

/// One request's share of its connection.
struct Request {
    quic: quinn::Connection,
    info: Arc<ConnInfo>,
    app: Arc<AppState>,
    router: Router,
}

impl Request {
    async fn run(self, resolver: RequestResolver<h3_quinn::Connection, Bytes>, _inflight: mpsc::Sender<()>) {
        let id = self.info.id;
        let (req, stream) = match tokio::time::timeout(HEADER_READ_TIMEOUT, resolver.resolve_request()).await {
            Ok(Ok(resolved)) => resolved,
            Ok(Err(e)) => {
                tracing::debug!(conn = id, "HTTP/3 request rejected: {e}");
                return;
            }
            Err(_) => {
                tracing::debug!(conn = id, "no HTTP/3 request head within {HEADER_READ_TIMEOUT:?}");
                return;
            }
        };
        // The request head is decoded: the same moment `t0` marks on TCP.
        let t0 = Instant::now();
        let seq = self.info.next_request();
        let ctx = ReqCtx { t0, conn: self.info, seq, rtt: None, quic_rtt: quic_rtt(&self.quic) };
        let (send, recv) = stream.split();
        let method = req.method().clone();
        let req = req.map(|()| body::RequestBody::new(recv));
        let response = conn::respond(&self.app, self.router, ctx, req).await;
        if let Err(e) = body::send(send, &method, response).await {
            tracing::debug!(conn = id, "HTTP/3 response not completed: {e}");
        }
    }
}

/// quinn's smoothed RTT for the connection (RFC 9002 §5.3). Before any acknowledgement quinn
/// returns its configured initial RTT (333 ms), which is not a measurement — so only once the
/// client has acknowledged something, which it has after any completed handshake.
fn quic_rtt(quic: &quinn::Connection) -> Option<Duration> {
    let stats = quic.stats();
    (stats.frame_rx.acks > 0).then_some(stats.path.rtt)
}

/// Completes once the server starts shutting down.
async fn stopping(shutdown: &mut watch::Receiver<bool>) {
    let _ = shutdown.wait_for(|stopping| *stopping).await;
}
