//! Test harness: an in-process server on ephemeral ports, and a small HTTP client that keeps
//! full control over connections and headers.
#![allow(dead_code)] // each test binary uses a different subset

use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use bytes::Bytes;
use http::{HeaderMap, Request, Response, StatusCode, Version};
use http_body_util::{BodyExt, Full};
use hyper::body::Incoming;
use hyper::client::conn::{http1, http2};
use hyper_util::rt::{TokioExecutor, TokioIo};
use server::{Config, SelfSigned, Server};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::TcpStream;
use tokio::sync::oneshot;

/// The fixture site (`tests/fixtures/make-dist.sh`).
pub fn fixture_dist() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/dist")
}

/// A fresh, empty directory under `target/tmp`.
pub fn temp_dir(name: &str) -> PathBuf {
    static N: AtomicUsize = AtomicUsize::new(0);
    let n = N.fetch_add(1, Ordering::Relaxed);
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("{name}-{}-{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// Defaults for tests: plain HTTP on an ephemeral port, a private data dir, quick PoW.
pub fn config(name: &str) -> Config {
    Config {
        dist: fixture_dist(),
        listen: Some("127.0.0.1:0".parse().unwrap()),
        tls: None,
        redirect_http: None,
        region: "test".into(),
        location: Some(proof::Location { city: "Dushanbe".into(), country: "TJ".into(), lat: 38.5598, lon: 68.787 }),
        data_dir: temp_dir(name),
        contact_recipient: None,
        pow_difficulty: 12,
        telegram: None,
        max_telemetry_streams: 64,
        shutdown_grace: Duration::from_secs(2),
    }
}

/// A running server; shuts down when dropped.
pub struct TestServer {
    pub http: Option<SocketAddr>,
    pub https: Option<SocketAddr>,
    pub redirect: Option<SocketAddr>,
    pub self_signed: Option<SelfSigned>,
    pub data_dir: PathBuf,
    stop: Option<oneshot::Sender<()>>,
    task: Option<tokio::task::JoinHandle<anyhow::Result<()>>>,
}

impl TestServer {
    pub async fn start(config: Config) -> TestServer {
        let data_dir = config.data_dir.clone();
        let server = Server::bind(config).await.expect("server binds");
        let (stop, stopped) = oneshot::channel::<()>();
        let http = server.http_addr();
        let https = server.https_addr();
        let redirect = server.redirect_addr();
        let self_signed = server.self_signed().cloned();
        let task = tokio::spawn(server.run(async move {
            let _ = stopped.await;
        }));
        TestServer { http, https, redirect, self_signed, data_dir, stop: Some(stop), task: Some(task) }
    }

    pub fn addr(&self) -> SocketAddr {
        self.http.expect("plain HTTP listener")
    }

    /// Signals shutdown and waits for `run` to return.
    pub async fn stop(mut self) -> anyhow::Result<()> {
        if let Some(stop) = self.stop.take() {
            let _ = stop.send(());
        }
        match self.task.take() {
            Some(task) => task.await.expect("server task"),
            None => Ok(()),
        }
    }
}

impl Drop for TestServer {
    fn drop(&mut self) {
        if let Some(stop) = self.stop.take() {
            let _ = stop.send(());
        }
    }
}

/// A complete response.
#[derive(Debug)]
pub struct Reply {
    pub status: StatusCode,
    pub version: Version,
    pub headers: HeaderMap,
    pub body: Bytes,
}

impl Reply {
    /// A header as text, `""` if absent.
    pub fn header(&self, name: &str) -> &str {
        self.headers.get(name).map_or("", |v| v.to_str().unwrap())
    }

    pub fn text(&self) -> String {
        String::from_utf8(self.body.to_vec()).expect("UTF-8 body")
    }

    pub fn json<T: serde::de::DeserializeOwned>(&self) -> T {
        serde_json::from_slice(&self.body).unwrap_or_else(|e| panic!("JSON body ({e}): {}", self.text()))
    }

    /// `Server-Timing` as name → duration in ms.
    pub fn timing(&self) -> HashMap<String, f64> {
        server_timing(self.header("server-timing"))
    }
}

pub fn server_timing(value: &str) -> HashMap<String, f64> {
    value
        .split(',')
        .filter_map(|metric| {
            let mut parts = metric.trim().split(';');
            let name = parts.next()?.to_owned();
            let dur = parts.find_map(|p| p.strip_prefix("dur="))?.parse().ok()?;
            Some((name, dur))
        })
        .collect()
}

/// One client connection, HTTP/1.1 or HTTP/2.
pub enum Client {
    H1(http1::SendRequest<Full<Bytes>>),
    H2(http2::SendRequest<Full<Bytes>>),
}

impl Client {
    /// HTTP/1.1 over plain TCP.
    pub async fn h1(addr: SocketAddr) -> Client {
        Client::over(TcpStream::connect(addr).await.unwrap(), false).await
    }

    /// HTTP/2 with prior knowledge over plain TCP (h2c).
    pub async fn h2c(addr: SocketAddr) -> Client {
        Client::over(TcpStream::connect(addr).await.unwrap(), true).await
    }

    /// Over any established stream (e.g. TLS).
    pub async fn over<IO>(io: IO, h2: bool) -> Client
    where
        IO: AsyncRead + AsyncWrite + Unpin + Send + 'static,
    {
        let io = TokioIo::new(io);
        if h2 {
            let (sender, conn) = http2::handshake(TokioExecutor::new(), io).await.unwrap();
            tokio::spawn(conn);
            Client::H2(sender)
        } else {
            let (sender, conn) = http1::handshake(io).await.unwrap();
            tokio::spawn(conn);
            Client::H1(sender)
        }
    }

    /// Sends and returns the response with its body still streaming.
    pub async fn stream(&mut self, mut req: Request<Full<Bytes>>) -> Response<Incoming> {
        match self {
            Client::H1(s) => {
                // Browsers send origin-form (`GET /path HTTP/1.1` + `Host`), so do we.
                let origin_form = req.uri().path_and_query().map_or("/", |pq| pq.as_str()).to_owned();
                *req.uri_mut() = origin_form.parse().unwrap();
                s.ready().await.unwrap();
                s.send_request(req).await.unwrap()
            }
            Client::H2(s) => {
                s.ready().await.unwrap();
                s.send_request(req).await.unwrap()
            }
        }
    }

    pub async fn send(&mut self, req: Request<Full<Bytes>>) -> Reply {
        let response = self.stream(req).await;
        let (parts, body) = response.into_parts();
        let body = tokio::time::timeout(Duration::from_secs(10), body.collect())
            .await
            .expect("body within 10 s")
            .unwrap()
            .to_bytes();
        Reply { status: parts.status, version: parts.version, headers: parts.headers, body }
    }

    pub async fn get(&mut self, path: &str, headers: &[(&str, &str)]) -> Reply {
        self.send(request("GET", path, headers, Bytes::new())).await
    }

    fn is_h2(&self) -> bool {
        matches!(self, Client::H2(_))
    }
}

/// A request with a `Host` (HTTP/1.1) and the given headers.
pub fn request(method: &str, path: &str, headers: &[(&str, &str)], body: impl Into<Bytes>) -> Request<Full<Bytes>> {
    let mut builder = Request::builder().method(method).uri(format!("http://localhost{path}"));
    if !headers.iter().any(|(n, _)| n.eq_ignore_ascii_case("host")) {
        builder = builder.header("host", "localhost");
    }
    for (name, value) in headers {
        builder = builder.header(*name, *value);
    }
    builder.body(Full::new(body.into())).unwrap()
}

/// GET on a fresh HTTP/1.1 connection.
pub async fn get(addr: SocketAddr, path: &str, headers: &[(&str, &str)]) -> Reply {
    Client::h1(addr).await.get(path, headers).await
}

/// A browser-like `User-Agent` (curl mode is triggered by curl/Wget/HTTPie/xh).
pub const BROWSER: (&str, &str) = ("user-agent", "Mozilla/5.0 (X11; Linux x86_64; rv:140.0) Gecko/20100101 Firefox/140.0");

/// The security headers every response must carry (plain HTTP: no HSTS).
pub const SECURITY_HEADERS: [&str; 7] = [
    "content-security-policy",
    "cross-origin-opener-policy",
    "cross-origin-embedder-policy",
    "cross-origin-resource-policy",
    "permissions-policy",
    "referrer-policy",
    "x-content-type-options",
];

/// Exact header set of §4.2 on one response.
pub fn assert_security_headers(reply: &Reply, tls: bool, what: &str) {
    for name in SECURITY_HEADERS {
        assert!(reply.headers.contains_key(name), "{what}: missing {name}");
    }
    assert_eq!(reply.header("cross-origin-opener-policy"), "same-origin", "{what}");
    assert_eq!(reply.header("cross-origin-embedder-policy"), "require-corp", "{what}");
    assert_eq!(reply.header("cross-origin-resource-policy"), "same-origin", "{what}");
    assert_eq!(
        reply.header("permissions-policy"),
        "camera=(), microphone=(), geolocation=(), payment=(), usb=(), browsing-topics=()",
        "{what}"
    );
    assert_eq!(reply.header("referrer-policy"), "no-referrer", "{what}");
    assert_eq!(reply.header("x-content-type-options"), "nosniff", "{what}");
    let csp = reply.header("content-security-policy");
    if tls {
        assert_eq!(reply.header("strict-transport-security"), "max-age=63072000; includeSubDomains; preload", "{what}");
        assert!(csp.ends_with("; upgrade-insecure-requests"), "{what}: {csp}");
    } else {
        assert!(!reply.headers.contains_key("strict-transport-security"), "{what}: HSTS over plain HTTP");
        assert!(!csp.contains("upgrade-insecure-requests"), "{what}: {csp}");
    }
    assert!(!reply.headers.contains_key("set-cookie"), "{what}: Set-Cookie");
    assert!(!reply.headers.contains_key("server"), "{what}: Server header");
    let timing = reply.timing();
    for metric in ["app", "route", "render"] {
        assert!(timing.contains_key(metric), "{what}: Server-Timing lacks {metric}: {:?}", reply.header("server-timing"));
    }
    assert!(timing["app"] + 0.0011 >= timing["route"], "{what}: app ≥ route");
    #[cfg(target_os = "linux")]
    assert!(timing.contains_key("krtt"), "{what}: Server-Timing lacks krtt");
}
