//! HTTP/3: a quinn + h3 client against the self-signed server. Every response passes the same
//! `routes::finish` as HTTP/1.1 and HTTP/2; `/_proof` reports this QUIC connection's real
//! handshake; bodies stream both ways; Alt-Svc announces it; shutdown closes it.

mod common;

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};

use bytes::{Buf, Bytes, BytesMut};
use common::{BROWSER, Client, Reply, TestServer, assert_security_headers, pinned_client, tls_config};
use h3::client::{RequestStream, SendRequest};
use http::{Request, StatusCode, Version};
use quinn::crypto::rustls::QuicClientConfig;
use rustls::CipherSuite;
use rustls::crypto::{CryptoProvider, aws_lc_rs};
use rustls::pki_types::ServerName;
use tokio::net::TcpStream;
use tokio_rustls::TlsConnector;

/// One HTTP/3 connection from a quinn client.
struct H3 {
    send: SendRequest<h3_quinn::OpenStreams, Bytes>,
    quic: quinn::Connection,
    authority: String,
    _endpoint: quinn::Endpoint,
}

impl H3 {
    async fn connect(server: &TestServer, provider: CryptoProvider) -> H3 {
        let addr = server.http3.expect("HTTP/3 endpoint");
        let tls = pinned_client(&server.self_signed.as_ref().unwrap().cert, provider, &[&rustls::version::TLS13], &[b"h3"]);
        let config = quinn::ClientConfig::new(Arc::new(QuicClientConfig::try_from(tls).unwrap()));
        let mut endpoint = quinn::Endpoint::client("127.0.0.1:0".parse().unwrap()).unwrap();
        endpoint.set_default_client_config(config);
        let quic = endpoint.connect(addr, "localhost").unwrap().await.expect("QUIC handshake");
        let (mut driver, send) = h3::client::new(h3_quinn::Connection::new(quic.clone())).await.unwrap();
        tokio::spawn(async move {
            let _ = std::future::poll_fn(|cx| driver.poll_close(cx)).await;
        });
        H3 { send, quic, authority: format!("localhost:{}", addr.port()), _endpoint: endpoint }
    }

    fn request(&self, method: &str, path: &str, headers: &[(&str, &str)]) -> Request<()> {
        let mut builder = Request::builder().method(method).uri(format!("https://{}{path}", self.authority));
        for (name, value) in headers {
            builder = builder.header(*name, *value);
        }
        builder.body(()).unwrap()
    }

    /// Sends the request (and body), returns the response head with the body still streaming.
    async fn open(&mut self, req: Request<()>, body: Bytes) -> (http::Response<()>, RequestStream<h3_quinn::BidiStream<Bytes>, Bytes>) {
        let mut stream = self.send.send_request(req).await.unwrap();
        if !body.is_empty() {
            stream.send_data(body).await.unwrap();
        }
        stream.finish().await.unwrap();
        let mut head = stream.recv_response().await.unwrap();
        *head.version_mut() = Version::HTTP_3;
        (head, stream)
    }

    async fn send(&mut self, method: &str, path: &str, headers: &[(&str, &str)], body: impl Into<Bytes>) -> Reply {
        let req = self.request(method, path, headers);
        let (head, mut stream) = self.open(req, body.into()).await;
        let mut body = BytesMut::new();
        let read = async {
            while let Some(mut chunk) = stream.recv_data().await.unwrap() {
                body.extend_from_slice(&chunk.copy_to_bytes(chunk.remaining()));
            }
        };
        tokio::time::timeout(Duration::from_secs(10), read).await.expect("body within 10 s");
        let (parts, ()) = head.into_parts();
        Reply { status: parts.status, version: parts.version, headers: parts.headers, body: body.freeze() }
    }

    async fn get(&mut self, path: &str, headers: &[(&str, &str)]) -> Reply {
        self.send("GET", path, headers, Bytes::new()).await
    }
}

/// A client that offers ChaCha20 first: rustls follows the client's order, so `/_proof` must
/// report ChaCha20, not the server's or quinn's default (AES-128-GCM stays for QUIC's Initial
/// packets, which always use it).
fn chacha_first() -> CryptoProvider {
    let mut provider = aws_lc_rs::default_provider();
    provider.cipher_suites.sort_by_key(|s| s.suite() != CipherSuite::TLS13_CHACHA20_POLY1305_SHA256);
    provider
}

async fn h3_server(name: &str) -> TestServer {
    TestServer::start(tls_config(name, false, true)).await
}

#[tokio::test]
async fn pages_and_proof_over_http3_report_the_real_handshake() {
    let server = h3_server("h3-proof").await;
    let mut h3 = H3::connect(&server, chacha_first()).await;
    assert_eq!(server.http3.unwrap().port(), server.https.unwrap().port(), "UDP shares the TLS port");

    let page = h3.get("/en/", &[BROWSER]).await;
    assert_eq!(page.status, StatusCode::OK);
    assert!(page.text().contains("Software at the edge of physics."));
    assert_security_headers(&page, true, "page over HTTP/3");
    let alt_svc = format!("h3=\":{}\"; ma=86400", server.http3.unwrap().port());
    assert_eq!(page.header("alt-svc"), alt_svc);
    let timing = page.timing();
    assert!(timing.contains_key("tls"), "handshake time on the first request: {}", page.header("server-timing"));
    assert!(timing["qrtt"] > 0.0);
    assert!(page.header("server-timing").contains(r#";desc="QUIC RTT""#));
    assert_eq!(page.header("content-length"), page.body.len().to_string());

    let reply = h3.get("/_proof", &[BROWSER]).await;
    assert_security_headers(&reply, true, "proof over HTTP/3");
    assert!(!reply.timing().contains_key("tls"), "second request: no handshake metric");
    let p: proof::Proof = reply.json();
    assert_eq!(p.http.version, "HTTP/3.0");
    let tls = p.tls.expect("QUIC is always TLS");
    assert_eq!(tls.alpn.as_deref(), Some("h3"));
    assert_eq!(tls.version, "TLSv1.3");
    assert_eq!(tls.group.as_deref(), Some("X25519MLKEM768"), "rustls' default client offers it first");
    assert!(tls.post_quantum);
    assert_eq!(tls.cipher.as_deref(), Some("TLS13_CHACHA20_POLY1305_SHA256"), "the suite this client chose");
    let handshake = tls.handshake_us.expect("measured");
    assert!(handshake > 0);
    assert!((timing["tls"] - handshake as f64 / 1000.0).abs() < 0.0011, "{} vs {handshake} µs", timing["tls"]);
    assert_eq!(p.connection.kernel_rtt_us, None, "no TCP socket");
    assert_eq!(p.connection.kernel_rttvar_us, None);
    let quic_rtt = p.connection.quic_rtt_us.expect("QUIC's own estimate");
    assert!(quic_rtt > 0 && u128::from(quic_rtt) <= h3.quic.rtt().as_micros() * 10 + 1000, "{quic_rtt} µs");
    assert_eq!(p.connection.requests, 2);
    assert_eq!(p.headers.last().map(String::as_str), Some("alt-svc"));
    assert_eq!(p.headers.len(), 9);
    for name in &p.headers {
        assert!(reply.headers.contains_key(name.as_str()), "{name} listed but not sent");
    }

    // curl mode speaks HTTP/3 too.
    let text = h3.get("/", &[("user-agent", "curl/8.23.0")]).await;
    assert!(text.text().contains("TLS 1.3 · X25519MLKEM768 (post-quantum hybrid) · HTTP/3"), "{}", text.text());
    let proof_text = h3.get("/proof", &[("user-agent", "curl/8.23.0")]).await.text();
    assert!(proof_text.contains("ALPN h3") && proof_text.contains("HTTP/3.0"), "{proof_text}");
    assert!(proof_text.contains("not available (QUIC: no TCP socket)") && proof_text.contains("QUIC's own estimate"), "{proof_text}");
}

#[tokio::test]
async fn a_classical_quic_client_is_reported_as_classical() {
    let server = h3_server("h3-classic").await;
    let mut provider = aws_lc_rs::default_provider();
    provider.kx_groups = vec![aws_lc_rs::kx_group::X25519];
    let mut h3 = H3::connect(&server, provider).await;
    let tls = h3.get("/_proof", &[BROWSER]).await.json::<proof::Proof>().tls.unwrap();
    assert_eq!(tls.group.as_deref(), Some("X25519"));
    assert!(!tls.post_quantum);
    assert_eq!(tls.cipher.as_deref(), Some("TLS13_AES_256_GCM_SHA384"), "rustls' default client order");
}

#[tokio::test]
async fn request_bodies_stream_into_the_contact_handler_with_its_limit() {
    let server = h3_server("h3-contact").await;
    let mut h3 = H3::connect(&server, aws_lc_rs::default_provider()).await;
    let form = [("content-type", "application/x-www-form-urlencoded"), BROWSER];
    let ok = h3.send("POST", "/api/contact", &form, "lang=en&name=Bill&email=bill%40mintaka.tj&message=over+QUIC").await;
    assert_eq!(ok.status, StatusCode::SEE_OTHER);
    assert_eq!(ok.header("location"), "/en/sent/");
    assert_security_headers(&ok, true, "contact over HTTP/3");
    let stored: Vec<_> = std::fs::read_dir(server.data_dir.join("contact")).unwrap().collect();
    assert_eq!(stored.len(), 1, "the brief arrived whole and was stored");

    let json = [("content-type", "application/json"), BROWSER];
    let invalid = h3.send("POST", "/api/contact", &json, r#"{"name":"","email":"x"}"#).await;
    assert_eq!(invalid.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(invalid.text().contains("name-required"), "{}", invalid.text());

    // 40 KiB, with no Content-Length: refused while reading, at 32 KiB.
    let big = h3.send("POST", "/api/contact", &json, vec![b' '; 40 * 1024]).await;
    assert_eq!(big.status, StatusCode::PAYLOAD_TOO_LARGE);
    assert_eq!(big.text(), r#"{"ok":false,"errors":[{"field":"form","code":"too-large"}]}"#);
    // And with a declared Content-Length, before reading.
    let declared = [("content-type", "application/json"), ("content-length", "40960"), BROWSER];
    let big = h3.send("POST", "/api/contact", &declared, vec![b' '; 40 * 1024]).await;
    assert_eq!(big.status, StatusCode::PAYLOAD_TOO_LARGE);
    assert_eq!(std::fs::read_dir(server.data_dir.join("contact")).unwrap().count(), 1);
}

#[tokio::test]
async fn telemetry_events_stream_over_http3() {
    let server = h3_server("h3-sse").await;
    let mut h3 = H3::connect(&server, aws_lc_rs::default_provider()).await;
    let req = h3.request("GET", "/_telemetry", &[BROWSER]);
    let (head, mut stream) = h3.open(req, Bytes::new()).await;
    assert_eq!(head.status(), StatusCode::OK);
    assert_eq!(head.headers()["content-type"], "text/event-stream");
    assert!(!head.headers().contains_key("content-length"), "a stream has no length");

    let mut text = String::new();
    let started = Instant::now();
    let read = async {
        // `hello` arrives at once; then a `tick` frame per second (the latest one is sent at once
        // if the sampler has produced one).
        while text.matches("event: tick").count() < 2 {
            let mut chunk = stream.recv_data().await.unwrap().expect("stream stays open");
            text.push_str(std::str::from_utf8(&chunk.copy_to_bytes(chunk.remaining())).unwrap());
        }
    };
    tokio::time::timeout(Duration::from_secs(5), read).await.expect("two ticks within 5 s");
    assert!(text.starts_with("event: hello\n"), "{text}");
    assert!(started.elapsed() >= Duration::from_millis(500), "ticks arrive as they happen, not batched");
}

/// Regression: the first-request timer (10 s) once closed connections whose only request was
/// still running.
#[tokio::test]
async fn a_long_request_outlives_the_first_request_timeout() {
    let server = h3_server("h3-long").await;
    let mut h3 = H3::connect(&server, aws_lc_rs::default_provider()).await;
    let req = h3.request("GET", "/_telemetry", &[BROWSER]);
    let (_, mut stream) = h3.open(req, Bytes::new()).await;
    let until = Instant::now() + Duration::from_secs(11);
    while Instant::now() < until {
        let frame = tokio::time::timeout(Duration::from_secs(3), stream.recv_data()).await.expect("a tick within 3 s");
        assert!(frame.unwrap().is_some(), "stream ended after {:?}", until.elapsed());
    }
    assert!(h3.quic.close_reason().is_none(), "{:?}", h3.quic.close_reason());
}

#[tokio::test]
async fn head_and_not_modified_carry_no_body() {
    let server = h3_server("h3-head").await;
    let mut h3 = H3::connect(&server, aws_lc_rs::default_provider()).await;
    let get = h3.get("/en/", &[BROWSER]).await;
    let head = h3.send("HEAD", "/en/", &[BROWSER], Bytes::new()).await;
    assert_eq!(head.status, StatusCode::OK);
    assert!(head.body.is_empty());
    assert_eq!(head.header("content-length"), get.body.len().to_string());
    let etag = get.header("etag").to_owned();
    let cached = h3.get("/en/", &[("if-none-match", &etag), BROWSER]).await;
    assert_eq!(cached.status, StatusCode::NOT_MODIFIED);
    assert!(cached.body.is_empty());
    assert!(!cached.headers.contains_key("content-length"));
    let ping = h3.get("/_ping", &[BROWSER]).await;
    assert_eq!(ping.status, StatusCode::NO_CONTENT);
}

/// HTTP/2 over the TCP listener of an HTTP/3 server.
async fn h2(server: &TestServer) -> Client {
    let tls = pinned_client(&server.self_signed.as_ref().unwrap().cert, aws_lc_rs::default_provider(), &[&rustls::version::TLS13], &[b"h2"]);
    let tcp = TcpStream::connect(server.https.unwrap()).await.unwrap();
    let stream = TlsConnector::from(Arc::new(tls)).connect(ServerName::try_from("localhost").unwrap(), tcp).await.unwrap();
    Client::over(stream, true).await
}

#[tokio::test]
async fn alt_svc_announces_http3_on_tls_responses_only() {
    let server = h3_server("h3-alt-svc").await;
    let mut client = h2(&server).await;
    let page = client.get("/en/", &[BROWSER]).await;
    assert_eq!(page.version, Version::HTTP_2);
    assert_eq!(page.header("alt-svc"), format!("h3=\":{}\"; ma=86400", server.http3.unwrap().port()));
    let reply = client.get("/_proof", &[BROWSER]).await;
    let p: proof::Proof = reply.json();
    assert_eq!(p.headers.last().map(String::as_str), Some("alt-svc"));
    assert_eq!(p.tls.unwrap().alpn.as_deref(), Some("h2"));
    assert!(p.connection.kernel_rtt_us.is_some() && p.connection.quic_rtt_us.is_none());

    // The plain listener next to it never announces it (and /_proof does not list it).
    let plain = common::get(server.addr(), "/_proof", &[BROWSER]).await;
    assert!(!plain.headers.contains_key("alt-svc"));
    assert!(!plain.json::<proof::Proof>().headers.iter().any(|h| h == "alt-svc"));
}

#[tokio::test]
async fn the_advertised_port_can_differ_from_the_bound_one() {
    let mut cfg = tls_config("h3-advertise", false, true);
    cfg.tls.as_mut().unwrap().http3.as_mut().unwrap().advertise_port = Some(443);
    let server = TestServer::start(cfg).await;
    let page = h2(&server).await.get("/en/", &[BROWSER]).await;
    assert_eq!(page.header("alt-svc"), "h3=\":443\"; ma=86400");
}

#[tokio::test]
async fn shutdown_ends_event_streams_and_closes_quic_within_the_grace_period() {
    let server = h3_server("h3-shutdown").await;
    let addr: SocketAddr = server.http3.unwrap();
    let mut h3 = H3::connect(&server, aws_lc_rs::default_provider()).await;
    let req = h3.request("GET", "/_telemetry", &[BROWSER]);
    let (_, mut stream) = h3.open(req, Bytes::new()).await;
    let first = stream.recv_data().await.unwrap();
    assert!(first.is_some());

    let started = Instant::now();
    tokio::time::timeout(Duration::from_secs(4), server.stop()).await.expect("stops within the 2 s grace + margin").unwrap();
    assert!(started.elapsed() < Duration::from_secs(3), "{:?}", started.elapsed());
    // The event stream ended cleanly (FIN after the last frame), and the connection is closed.
    let drained = async {
        while let Ok(Some(_)) = stream.recv_data().await {}
    };
    tokio::time::timeout(Duration::from_secs(2), drained).await.expect("stream ended");
    let reason = tokio::time::timeout(Duration::from_secs(2), h3.quic.closed()).await.expect("connection closed");
    assert!(matches!(reason, quinn::ConnectionError::ApplicationClosed(_)), "{reason:?}");

    // And nothing answers on that port any more.
    let tls = pinned_client(&rcgen_cert(), aws_lc_rs::default_provider(), &[&rustls::version::TLS13], &[b"h3"]);
    let mut endpoint = quinn::Endpoint::client("127.0.0.1:0".parse().unwrap()).unwrap();
    endpoint.set_default_client_config(quinn::ClientConfig::new(Arc::new(QuicClientConfig::try_from(tls).unwrap())));
    let attempt = tokio::time::timeout(Duration::from_millis(500), endpoint.connect(addr, "localhost").unwrap()).await;
    assert!(!matches!(attempt, Ok(Ok(_))), "no QUIC handshake after shutdown");
}

/// Any certificate: the post-shutdown connection attempt must fail before verification matters.
fn rcgen_cert() -> rustls::pki_types::CertificateDer<'static> {
    rcgen::generate_simple_self_signed(vec!["localhost".to_owned()]).unwrap().cert.der().clone()
}
