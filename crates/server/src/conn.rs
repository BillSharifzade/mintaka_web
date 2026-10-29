//! Our own accept loop: `TcpListener` → (rustls) → hyper-util auto (HTTP/1.1 + HTTP/2) → axum.
//!
//! Each connection gets a [`ConnService`] that stamps every request with a [`ReqCtx`] — the
//! instant its head was parsed, the connection's facts (id, TLS, request count) and the kernel's
//! RTT for the socket — and hands it to [`respond`], which HTTP/3 (`quic`) shares: the router,
//! then [`routes::finish`] for the headers every response carries.
//!
//! A ClientHello that offers only `acme-tls/1` is an ACME validation: it gets the challenge
//! configuration, and the connection ends with the handshake.

use std::convert::Infallible;
use std::future::Future;
use std::net::SocketAddr;
use std::os::fd::{AsRawFd, RawFd};
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use axum::Router;
use axum::response::Response;
use bytes::Bytes;
use http::Request;
use hyper::body::Incoming;
use hyper_util::rt::{TokioExecutor, TokioIo, TokioTimer};
use hyper_util::server::conn::auto;
use hyper_util::server::graceful::{GracefulShutdown, Watcher};
use tokio::io::AsyncWriteExt;
use tokio::net::{TcpListener, TcpStream};
use tokio_rustls::LazyConfigAcceptor;
use tower_service::Service;

use crate::acme;
use crate::app::AppState;
use crate::contact::ClientNet;
use crate::routes;
use crate::sys::{self, TcpRtt};
use crate::tls::{TcpTls, TlsFacts};

/// A client must finish the TLS handshake within this time (QUIC too).
pub(crate) const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(10);
/// …and start its first request within this time (covers connections that never send a byte,
/// which hyper's protocol detection would otherwise wait on forever).
pub(crate) const FIRST_REQUEST_TIMEOUT: Duration = Duration::from_secs(10);
/// …and send each complete HTTP/1.1 request head (HTTP/3: each request's HEADERS) within this
/// time (also closes idle keep-alive connections).
pub(crate) const HEADER_READ_TIMEOUT: Duration = Duration::from_secs(10);

/// Process-local connection ids, starting at 1. Never derived from the client.
static NEXT_CONNECTION_ID: AtomicU64 = AtomicU64::new(1);

/// The next connection id (TCP and QUIC share the counter).
pub(crate) fn next_connection_id() -> u64 {
    NEXT_CONNECTION_ID.fetch_add(1, Ordering::Relaxed)
}

/// One client connection.
pub(crate) struct ConnInfo {
    pub id: u64,
    /// `None` on plain HTTP.
    pub tls: Option<TlsFacts>,
    /// Requests received so far (the current one included, once counted).
    requests: AtomicU64,
    /// The client's /24 or /48, for rate limiting. The address itself is not kept.
    pub client_net: ClientNet,
}

impl ConnInfo {
    pub fn new(id: u64, tls: Option<TlsFacts>, client_net: ClientNet) -> Arc<ConnInfo> {
        Arc::new(ConnInfo { id, tls, requests: AtomicU64::new(0), client_net })
    }

    /// Counts a request whose head has arrived; returns its 1-based position.
    pub fn next_request(&self) -> u64 {
        self.requests.fetch_add(1, Ordering::Relaxed) + 1
    }

    /// Requests counted so far.
    pub fn requests(&self) -> u64 {
        self.requests.load(Ordering::Relaxed)
    }
}

/// What every handler knows about the request it serves (a request extension).
#[derive(Clone)]
pub(crate) struct ReqCtx {
    /// When hyper handed us the parsed request head.
    pub t0: Instant,
    pub conn: Arc<ConnInfo>,
    /// 1-based position of this request on its connection.
    pub seq: u64,
    /// `TCP_INFO` of the socket, read when the request arrived. `None` on QUIC (no TCP socket).
    pub rtt: Option<TcpRtt>,
    /// QUIC's own smoothed RTT for the connection (quinn's RFC 9002 estimator), read when the
    /// request arrived. `None` on TCP.
    pub quic_rtt: Option<Duration>,
}

impl ReqCtx {
    pub fn is_tls(&self) -> bool {
        self.conn.tls.is_some()
    }
}

/// Everything an accept loop needs.
#[derive(Clone)]
pub(crate) struct Listener {
    pub app: Arc<AppState>,
    pub router: Router,
    pub tls: Option<TcpTls>,
    pub http: Arc<auto::Builder<TokioExecutor>>,
}

/// The HTTP/1.1 + HTTP/2 connection builder shared by all listeners.
pub(crate) fn http_builder() -> auto::Builder<TokioExecutor> {
    let mut builder = auto::Builder::new(TokioExecutor::new());
    builder
        .http1()
        .timer(TokioTimer::new())
        .header_read_timeout(HEADER_READ_TIMEOUT)
        .keep_alive(true)
        .http2()
        .timer(TokioTimer::new())
        .max_concurrent_streams(256)
        .keep_alive_interval(Duration::from_secs(30))
        .keep_alive_timeout(Duration::from_secs(20));
    builder
}

/// Accepts until the future is dropped; every connection is watched by `graceful`.
pub(crate) async fn accept_loop(listener: TcpListener, ctx: Listener, graceful: &GracefulShutdown) {
    loop {
        match listener.accept().await {
            Ok((stream, peer)) => {
                let watcher = graceful.watcher();
                tokio::spawn(serve(stream, peer, ctx.clone(), watcher));
            }
            Err(e) => {
                // EMFILE and friends: back off instead of spinning.
                tracing::warn!("accept failed: {e}");
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
        }
    }
}

/// One connection, from accept to close.
async fn serve(stream: TcpStream, peer: SocketAddr, ctx: Listener, watcher: Watcher) {
    let _gauge = ctx.app.metrics.connection_opened();
    let _ = stream.set_nodelay(true);
    let fd = stream.as_raw_fd();
    let client_net = ClientNet::of(peer.ip());
    let id = next_connection_id();

    let Some(tls) = &ctx.tls else {
        let conn = ConnInfo::new(id, None, client_net);
        let service = ConnService { app: Arc::clone(&ctx.app), router: ctx.router.clone(), conn: Arc::clone(&conn), fd };
        let connection = ctx.http.serve_connection(TokioIo::new(stream), service);
        drive(watcher.watch(connection), &conn).await;
        return;
    };

    let handshake = async {
        let start = LazyConfigAcceptor::new(rustls::server::Acceptor::default(), stream).await?;
        if let Some(challenge) = &tls.challenge
            && acme::is_challenge(&start.client_hello())
        {
            // TLS-ALPN-01 (RFC 8737 §3): the CA checks the certificate of this handshake and
            // nothing else. Close right after it; no HTTP is ever spoken here.
            let name = start.client_hello().server_name().unwrap_or("").to_owned();
            let mut tls_stream = start.into_stream(Arc::clone(challenge)).await?;
            let _ = tls_stream.shutdown().await;
            tracing::info!(target: "acme", conn = id, "answered a TLS-ALPN-01 validation for {name}");
            return Ok(None);
        }
        // The ClientHello is complete: from here to the client's Finished is the server-side
        // handshake (our flight, one round trip, the client's flight, verification).
        let began = Instant::now();
        let tls_stream = start.into_stream(Arc::clone(&tls.config)).await?;
        Ok::<_, std::io::Error>(Some((tls_stream, began.elapsed())))
    };
    let (tls_stream, took) = match tokio::time::timeout(HANDSHAKE_TIMEOUT, handshake).await {
        Ok(Ok(Some(done))) => done,
        Ok(Ok(None)) => return,
        Ok(Err(e)) => {
            tracing::debug!(conn = id, "TLS handshake failed: {e}");
            return;
        }
        Err(_) => {
            tracing::debug!(conn = id, "TLS handshake timed out");
            return;
        }
    };
    let facts = TlsFacts::of(tls_stream.get_ref().1, took);
    let conn = ConnInfo::new(id, Some(facts), client_net);
    let service = ConnService { app: Arc::clone(&ctx.app), router: ctx.router.clone(), conn: Arc::clone(&conn), fd };
    let connection = ctx.http.serve_connection(TokioIo::new(tls_stream), service);
    drive(watcher.watch(connection), &conn).await;
}

/// Runs a connection to its end, but drops it if no request has started within
/// [`FIRST_REQUEST_TIMEOUT`]. After the first request, hyper's own timers apply.
async fn drive<E: std::fmt::Display>(connection: impl Future<Output = Result<(), E>>, conn: &ConnInfo) {
    let mut connection = std::pin::pin!(connection);
    let result = tokio::select! {
        result = &mut connection => result,
        () = tokio::time::sleep(FIRST_REQUEST_TIMEOUT) => {
            if conn.requests() == 0 {
                tracing::debug!(conn = conn.id, "no request within {FIRST_REQUEST_TIMEOUT:?}; closing");
                return;
            }
            connection.await
        }
    };
    if let Err(e) = result {
        tracing::debug!(conn = conn.id, "connection ended with an error: {e}");
    }
}

/// The hyper service of one connection.
#[derive(Clone)]
struct ConnService {
    app: Arc<AppState>,
    router: Router,
    conn: Arc<ConnInfo>,
    /// The connection's socket. Only read inside [`ConnService::call`], which hyper runs while
    /// polling the connection that owns the socket (so the descriptor is open and not reused).
    fd: RawFd,
}

impl hyper::service::Service<Request<Incoming>> for ConnService {
    type Response = Response;
    type Error = Infallible;
    type Future = Pin<Box<dyn Future<Output = Result<Response, Infallible>> + Send>>;

    fn call(&self, req: Request<Incoming>) -> Self::Future {
        let t0 = Instant::now();
        let seq = self.conn.next_request();
        let ctx = ReqCtx { t0, conn: Arc::clone(&self.conn), seq, rtt: sys::tcp_rtt(self.fd), quic_rtt: None };
        let router = self.router.clone();
        let app = Arc::clone(&self.app);
        Box::pin(async move { Ok(respond(&app, router, ctx, req).await) })
    }
}

/// One request, whatever the transport: attach the [`ReqCtx`], route, then
/// [`routes::finish`] (security headers, Server-Timing, metrics, access log).
pub(crate) async fn respond<B>(app: &AppState, mut router: Router, ctx: ReqCtx, mut req: Request<B>) -> Response
where
    B: http_body::Body<Data = Bytes> + Send + 'static,
    B::Error: Into<axum::BoxError>,
{
    let method = req.method().clone();
    req.extensions_mut().insert(ctx.clone());
    let Ok(mut response) = router.call(req).await;
    routes::finish(app, &ctx, &method, &mut response);
    response
}
