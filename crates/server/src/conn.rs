//! Our own accept loop: `TcpListener` → (rustls) → hyper-util auto (HTTP/1.1 + HTTP/2) → axum.
//!
//! Each connection gets a [`ConnService`] that stamps every request with a [`ReqCtx`] — the
//! instant its head was parsed, the connection's facts (id, TLS, request count) and the kernel's
//! RTT for the socket — and hands the response to [`routes::finish`] for the headers every
//! response carries.

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
use http::Request;
use hyper::body::Incoming;
use hyper_util::rt::{TokioExecutor, TokioIo, TokioTimer};
use hyper_util::server::conn::auto;
use hyper_util::server::graceful::{GracefulShutdown, Watcher};
use rustls::ServerConfig;
use tokio::net::{TcpListener, TcpStream};
use tokio_rustls::LazyConfigAcceptor;
use tower_service::Service;

use crate::app::AppState;
use crate::contact::ClientNet;
use crate::routes;
use crate::sys::{self, TcpRtt};
use crate::tls::TlsFacts;

/// A client must finish the TLS handshake within this time.
const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(10);
/// …and start its first request within this time (covers connections that never send a byte,
/// which hyper's protocol detection would otherwise wait on forever).
const FIRST_REQUEST_TIMEOUT: Duration = Duration::from_secs(10);
/// …and send each complete HTTP/1.1 request head within this time (also closes idle
/// keep-alive connections).
const HEADER_READ_TIMEOUT: Duration = Duration::from_secs(10);

/// Process-local connection ids, starting at 1. Never derived from the client.
static NEXT_CONNECTION_ID: AtomicU64 = AtomicU64::new(1);

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

/// What every handler knows about the request it serves (a request extension).
#[derive(Clone)]
pub(crate) struct ReqCtx {
    /// When hyper handed us the parsed request head.
    pub t0: Instant,
    pub conn: Arc<ConnInfo>,
    /// 1-based position of this request on its connection.
    pub seq: u64,
    /// `TCP_INFO` of the socket, read when the request arrived.
    pub rtt: Option<TcpRtt>,
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
    pub tls: Option<Arc<ServerConfig>>,
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
    let id = NEXT_CONNECTION_ID.fetch_add(1, Ordering::Relaxed);

    let Some(tls_config) = &ctx.tls else {
        let conn = Arc::new(ConnInfo { id, tls: None, requests: AtomicU64::new(0), client_net });
        let service = ConnService { app: Arc::clone(&ctx.app), router: ctx.router.clone(), conn: Arc::clone(&conn), fd };
        let connection = ctx.http.serve_connection(TokioIo::new(stream), service);
        drive(watcher.watch(connection), &conn).await;
        return;
    };

    let handshake = async {
        let start = LazyConfigAcceptor::new(rustls::server::Acceptor::default(), stream).await?;
        // The ClientHello is complete: from here to the client's Finished is the server-side
        // handshake (our flight, one round trip, the client's flight, verification).
        let began = Instant::now();
        let tls_stream = start.into_stream(Arc::clone(tls_config)).await?;
        Ok::<_, std::io::Error>((tls_stream, began.elapsed()))
    };
    let (tls_stream, took) = match tokio::time::timeout(HANDSHAKE_TIMEOUT, handshake).await {
        Ok(Ok(done)) => done,
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
    let conn = Arc::new(ConnInfo { id, tls: Some(facts), requests: AtomicU64::new(0), client_net });
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
            if conn.requests.load(Ordering::Relaxed) == 0 {
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

    fn call(&self, mut req: Request<Incoming>) -> Self::Future {
        let t0 = Instant::now();
        let seq = self.conn.requests.fetch_add(1, Ordering::Relaxed) + 1;
        let ctx = ReqCtx { t0, conn: Arc::clone(&self.conn), seq, rtt: sys::tcp_rtt(self.fd) };
        let method = req.method().clone();
        req.extensions_mut().insert(ctx.clone());
        let mut router = self.router.clone();
        let app = Arc::clone(&self.app);
        Box::pin(async move {
            let Ok(mut response) = router.call(req).await;
            routes::finish(&app, &ctx, &method, &mut response);
            Ok(response)
        })
    }
}
