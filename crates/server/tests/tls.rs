//! TLS: the handshake really is post-quantum hybrid, `/_proof` says so, and the TLS-only headers
//! appear only over TLS.

mod common;

use std::net::SocketAddr;
use std::sync::Arc;

use bytes::Bytes;
use common::{BROWSER, Client, TestServer, assert_security_headers, config, request};
use http::StatusCode;
use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::crypto::{CryptoProvider, aws_lc_rs, verify_tls12_signature, verify_tls13_signature};
use rustls::pki_types::{CertificateDer, ServerName, UnixTime};
use rustls::{ClientConfig, DigitallySignedStruct, NamedGroup, SignatureScheme, SupportedProtocolVersion};
use server::{CertSource, TlsSettings};
use tokio::net::TcpStream;
use tokio_rustls::TlsConnector;
use tokio_rustls::client::TlsStream;

/// Trusts exactly one certificate (the server's self-signed one), with real signature checks.
#[derive(Debug)]
struct Pinned {
    cert: CertificateDer<'static>,
    provider: Arc<CryptoProvider>,
}

impl ServerCertVerifier for Pinned {
    fn verify_server_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp: &[u8],
        _now: UnixTime,
    ) -> Result<ServerCertVerified, rustls::Error> {
        if end_entity.as_ref() == self.cert.as_ref() {
            Ok(ServerCertVerified::assertion())
        } else {
            Err(rustls::Error::General("not the pinned certificate".into()))
        }
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        verify_tls12_signature(message, cert, dss, &self.provider.signature_verification_algorithms)
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        verify_tls13_signature(message, cert, dss, &self.provider.signature_verification_algorithms)
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.provider.signature_verification_algorithms.supported_schemes()
    }
}

async fn tls_server(name: &str, allow_tls12: bool) -> TestServer {
    let mut cfg = config(name);
    cfg.tls = Some(TlsSettings {
        listen: "127.0.0.1:0".parse().unwrap(),
        source: CertSource::SelfSigned,
        allow_tls12,
    });
    cfg.redirect_http = Some("127.0.0.1:0".parse().unwrap());
    TestServer::start(cfg).await
}

/// A client with rustls' *default* aws-lc-rs provider (what any rustls client would use).
fn client_config(
    server: &TestServer,
    provider: CryptoProvider,
    versions: &[&'static SupportedProtocolVersion],
    alpn: &[&[u8]],
) -> ClientConfig {
    let provider = Arc::new(provider);
    let cert = server.self_signed.as_ref().unwrap().cert.clone();
    let mut config = ClientConfig::builder_with_provider(Arc::clone(&provider))
        .with_protocol_versions(versions)
        .unwrap()
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(Pinned { cert, provider }))
        .with_no_client_auth();
    config.alpn_protocols = alpn.iter().map(|p| p.to_vec()).collect();
    config
}

async fn connect(addr: SocketAddr, config: ClientConfig) -> std::io::Result<TlsStream<TcpStream>> {
    let tcp = TcpStream::connect(addr).await?;
    TlsConnector::from(Arc::new(config)).connect(ServerName::try_from("localhost").unwrap(), tcp).await
}

#[tokio::test]
async fn default_rustls_client_negotiates_x25519mlkem768_and_proof_reports_it() {
    let server = tls_server("tls-pq", false).await;
    let config = client_config(&server, aws_lc_rs::default_provider(), &[&rustls::version::TLS13], &[b"h2"]);
    let stream = connect(server.https.unwrap(), config).await.unwrap();
    let (_, session) = stream.get_ref();
    assert_eq!(session.negotiated_key_exchange_group().map(|g| g.name()), Some(NamedGroup::X25519MLKEM768));
    assert_eq!(session.protocol_version(), Some(rustls::ProtocolVersion::TLSv1_3));
    assert_eq!(session.alpn_protocol(), Some(&b"h2"[..]));
    let cipher = session.negotiated_cipher_suite().unwrap().suite().as_str().unwrap().to_owned();

    let mut client = Client::over(stream, true).await;
    let first = client.get("/_proof", &[BROWSER]).await;
    assert_eq!(first.status, StatusCode::OK);
    assert_security_headers(&first, true, "proof over TLS");
    let p: proof::Proof = first.json();
    let tls = p.tls.expect("tls facts over TLS");
    assert_eq!(tls.group, "X25519MLKEM768");
    assert!(tls.post_quantum);
    assert_eq!(tls.version, "TLSv1.3");
    assert_eq!(tls.cipher, cipher, "the suite the client saw");
    assert_eq!(tls.alpn.as_deref(), Some("h2"));
    assert!(tls.handshake_us.is_some_and(|us| us > 0));
    assert_eq!(p.http.version, "HTTP/2.0");
    assert_eq!(p.headers.last().map(String::as_str), Some("strict-transport-security"));
    assert_eq!(p.headers.len(), 8);
    for name in &p.headers {
        assert!(first.headers.contains_key(name.as_str()), "{name}");
    }

    // `tls` in Server-Timing on the first request of the connection only.
    let timing = first.timing();
    assert!(timing.contains_key("tls"), "{:?}", first.header("server-timing"));
    assert!(first.header("server-timing").contains(r#"tls;dur="#));
    assert!(first.header("server-timing").contains(r#";desc="TLS handshake""#));
    let handshake_ms = tls.handshake_us.unwrap() as f64 / 1000.0;
    assert!((timing["tls"] - handshake_ms).abs() < 0.0011, "{} vs {handshake_ms}", timing["tls"]);
    let second = client.get("/en/", &[BROWSER]).await;
    assert!(!second.timing().contains_key("tls"));
    assert_security_headers(&second, true, "page over TLS");
    let p2: proof::Proof = client.get("/_proof", &[BROWSER]).await.json();
    assert_eq!(p2.connection.requests, 3);
}

#[tokio::test]
async fn http11_over_tls_and_curl_text_say_post_quantum() {
    let server = tls_server("tls-h1", false).await;
    let config = client_config(&server, aws_lc_rs::default_provider(), &[&rustls::version::TLS13], &[b"http/1.1"]);
    let stream = connect(server.https.unwrap(), config).await.unwrap();
    let mut client = Client::over(stream, false).await;
    let text = client.get("/", &[("user-agent", "curl/8.23.0")]).await;
    assert!(text.text().contains("TLS 1.3 · X25519MLKEM768 (post-quantum hybrid) · HTTP/1.1"), "{}", text.text());
    let p: proof::Proof = client.get("/_proof", &[BROWSER]).await.json();
    assert_eq!(p.tls.unwrap().alpn.as_deref(), Some("http/1.1"));
    assert_eq!(p.http.version, "HTTP/1.1");
}

#[tokio::test]
async fn a_classical_client_is_reported_as_classical() {
    let server = tls_server("tls-classic", false).await;
    let mut provider = aws_lc_rs::default_provider();
    provider.kx_groups = vec![aws_lc_rs::kx_group::X25519];
    let config = client_config(&server, provider, &[&rustls::version::TLS13], &[b"h2"]);
    let stream = connect(server.https.unwrap(), config).await.unwrap();
    let mut client = Client::over(stream, true).await;
    let tls = client.get("/_proof", &[BROWSER]).await.json::<proof::Proof>().tls.unwrap();
    assert_eq!(tls.group, "X25519");
    assert!(!tls.post_quantum);
}

#[tokio::test]
async fn tls12_is_refused_unless_enabled() {
    let strict = tls_server("tls13-only", false).await;
    let tls12 = client_config(&strict, aws_lc_rs::default_provider(), &[&rustls::version::TLS12], &[b"h2"]);
    assert!(connect(strict.https.unwrap(), tls12).await.is_err(), "TLS 1.2 handshake must fail");

    let lenient = tls_server("tls12-ok", true).await;
    let tls12 = client_config(&lenient, aws_lc_rs::default_provider(), &[&rustls::version::TLS12], &[b"h2"]);
    let stream = connect(lenient.https.unwrap(), tls12).await.unwrap();
    let mut client = Client::over(stream, true).await;
    let tls = client.get("/_proof", &[BROWSER]).await.json::<proof::Proof>().tls.unwrap();
    assert_eq!(tls.version, "TLSv1.2");
    assert_eq!(tls.group, "X25519", "hybrid groups are TLS 1.3 only");
    assert!(!tls.post_quantum);
}

#[tokio::test]
async fn plain_listener_next_to_tls_sends_no_hsts() {
    let server = tls_server("tls-plain", false).await;
    let reply = common::get(server.addr(), "/en/", &[BROWSER]).await;
    assert_security_headers(&reply, false, "plain listener of a TLS server");
    let p: proof::Proof = common::get(server.addr(), "/_proof", &[BROWSER]).await.json();
    assert!(p.tls.is_none());
    assert_eq!(p.headers.len(), 7);
}

#[tokio::test]
async fn http_redirects_to_https_on_the_same_host() {
    let server = tls_server("tls-redirect", false).await;
    let port = server.https.unwrap().port();
    let addr = server.redirect.unwrap();
    let reply = Client::h1(addr).await.send(request("GET", "/en/?a=1", &[("host", "localhost:8080"), BROWSER], Bytes::new())).await;
    assert_eq!(reply.status, StatusCode::MOVED_PERMANENTLY);
    assert_eq!(reply.header("location"), format!("https://localhost:{port}/en/?a=1"));
    assert_security_headers(&reply, false, "redirect listener");
    // A Host that could rewrite the URL falls back to the manifest origin's host.
    let reply = Client::h1(addr).await.send(request("GET", "/", &[("host", "evil.example@mintaka.tj"), BROWSER], Bytes::new())).await;
    assert!(reply.header("location").starts_with(&format!("https://mintaka.tj:{port}/")), "{}", reply.header("location"));
}
