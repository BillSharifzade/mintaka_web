//! A site under a URL prefix (`manifest.base`), behind a trusted reverse proxy that ends the
//! visitors' TLS: the prefix comes off before routing and goes back on every `Location`; each
//! request is rate-limited by the visitor the proxy names; `/_proof` reports the edge, never the
//! proxy hop's TLS or kernel RTT as the visitor's.

mod common;

use std::path::Path;

use bytes::Bytes;
use common::{BROWSER, Client, TestServer, config, fixture_dist, request, temp_dir};
use http::StatusCode;
use server::ProxySettings;

const VISITOR: &str = "x-mintaka-client-ip";

/// The fixture dist, built for `/mintaka`.
fn prefixed_dist(name: &str) -> std::path::PathBuf {
    let dir = temp_dir(name);
    copy_dir(&fixture_dist(), &dir);
    let path = dir.join("manifest.json");
    let mut manifest: serde_json::Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    manifest["base"] = "/mintaka".into();
    std::fs::write(&path, serde_json::to_vec_pretty(&manifest).unwrap()).unwrap();
    dir
}

fn copy_dir(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for entry in std::fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let target = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_dir(&entry.path(), &target);
        } else {
            std::fs::copy(entry.path(), target).unwrap();
        }
    }
}

/// Under `/mintaka`, with `trusted` as the proxy the test client connects from (or not).
async fn start(name: &str, trusted: &str) -> TestServer {
    let mut cfg = config(name);
    cfg.dist = prefixed_dist(name);
    cfg.proxy = Some(ProxySettings {
        from: vec![cidr(trusted)],
        client_ip_header: http::HeaderName::from_static(VISITOR),
    });
    TestServer::start(cfg).await
}

fn cidr(s: &str) -> server::Cidr {
    use clap::Parser;
    let cli = server::Cli::try_parse_from(["mintaka-server", "--trusted-proxy", s]).unwrap();
    cli.trusted_proxy[0]
}

#[tokio::test]
async fn the_prefix_comes_off_and_goes_back_on() {
    let server = start("edge-prefix", "127.0.0.1").await;
    let mut client = Client::h1(server.addr()).await;

    let page = client.get("/mintaka/en/", &[BROWSER]).await;
    assert_eq!(page.status, StatusCode::OK);
    assert!(page.text().contains("<html"), "the page itself");
    let root = client.get("/mintaka/", &[BROWSER, ("accept-language", "en")]).await;
    assert_eq!(root.status, StatusCode::OK);

    // Outside the prefix belongs to whoever shares the host: a bare 404, no page of ours.
    for path in ["/en/", "/", "/mintakax/en/", "/_proof"] {
        let reply = client.get(path, &[BROWSER]).await;
        assert_eq!(reply.status, StatusCode::NOT_FOUND, "{path}");
        assert_eq!(reply.text(), "not found\n", "{path}");
        assert!(reply.headers.contains_key("content-security-policy"), "{path}: still a response of ours");
    }

    // Redirects the server makes carry the prefix.
    let bare = client.get("/mintaka?utm=x", &[BROWSER]).await;
    assert_eq!((bare.status, bare.header("location")), (StatusCode::MOVED_PERMANENTLY, "/mintaka/?utm=x"));
    let slash = client.get("/mintaka/en", &[BROWSER]).await;
    assert_eq!((slash.status, slash.header("location")), (StatusCode::MOVED_PERMANENTLY, "/mintaka/en/"));
    let headers = [("content-type", "application/x-www-form-urlencoded"), BROWSER];
    let form = client.send(request("POST", "/mintaka/api/contact", &headers, Bytes::from_static(b"lang=en&name=&email=a%40b.tj"))).await;
    assert_eq!((form.status, form.header("location")), (StatusCode::SEE_OTHER, "/mintaka/en/sent/#e-name-required"));

    // The 404 page's suggestion links into the prefix too.
    let missing = client.get("/mintaka/en/sentt/", &[BROWSER]).await;
    assert_eq!(missing.status, StatusCode::NOT_FOUND);
    assert!(missing.text().contains("<a href=\"/mintaka/en/sent/\">/mintaka/en/sent/</a>"), "suggestion under the prefix");
}

#[tokio::test]
async fn behind_the_proxy_proof_reports_the_edge_not_the_hop() {
    let server = start("edge-proof", "127.0.0.1").await;
    let mut client = Client::h1(server.addr()).await;
    let reply = client.get("/mintaka/_proof", &[BROWSER, ("x-forwarded-proto", "https"), (VISITOR, "203.0.113.9")]).await;
    assert_eq!(reply.status, StatusCode::OK);
    let proof: proof::Proof = reply.json();
    assert_eq!(proof.edge, Some(proof::EdgeFacts { proto: Some("https".into()) }));
    assert!(proof.tls.is_none(), "the visitor's TLS ended at the proxy");
    assert_eq!(proof.connection.kernel_rtt_us, None, "the socket's RTT is the proxy hop's");
    assert!(!proof.headers.iter().any(|h| h == "strict-transport-security"), "a host-wide policy is the host's");
    let timing = reply.header("server-timing");
    assert!(timing.contains(", edge;desc=") && !timing.contains("krtt"), "{timing}");

    let odd = client.get("/mintaka/_proof", &[BROWSER, ("x-forwarded-proto", "gopher")]).await.json::<proof::Proof>();
    assert_eq!(odd.edge, Some(proof::EdgeFacts { proto: None }));
}

#[tokio::test]
async fn an_untrusted_peer_cannot_claim_to_be_the_proxy() {
    let server = start("edge-untrusted", "10.9.8.7").await;
    let mut client = Client::h1(server.addr()).await;
    let reply = client.get("/mintaka/_proof", &[BROWSER, ("x-forwarded-proto", "https"), (VISITOR, "203.0.113.9")]).await;
    let proof: proof::Proof = reply.json();
    assert_eq!(proof.edge, None);
    assert!(proof.connection.kernel_rtt_us.is_some(), "a direct connection's own RTT");
    assert!(!reply.header("server-timing").contains("edge"));

    // …and its header doesn't move it into another network's rate limit.
    let start = |ip: &'static str| request("POST", "/mintaka/api/ctf/start", &[("content-type", "application/json"), BROWSER, (VISITOR, ip)], Bytes::from_static(b"{}"));
    for _ in 0..20 {
        assert_eq!(client.send(start("198.51.100.1")).await.status, StatusCode::OK);
    }
    assert_eq!(client.send(start("198.51.100.2")).await.status, StatusCode::TOO_MANY_REQUESTS);
}

#[tokio::test]
async fn each_visitor_the_proxy_names_has_their_own_limit() {
    let server = start("edge-limits", "127.0.0.1").await;
    // One proxy connection carries everyone's requests.
    let mut proxy = Client::h1(server.addr()).await;
    let start = |ip: &'static str| request("POST", "/mintaka/api/ctf/start", &[("content-type", "application/json"), BROWSER, (VISITOR, ip)], Bytes::from_static(b"{}"));
    for _ in 0..20 {
        assert_eq!(proxy.send(start("198.51.100.7")).await.status, StatusCode::OK);
    }
    assert_eq!(proxy.send(start("198.51.100.7")).await.status, StatusCode::TOO_MANY_REQUESTS, "same /24");
    assert_eq!(proxy.send(start("198.51.100.200")).await.status, StatusCode::TOO_MANY_REQUESTS, "same /24");
    assert_eq!(proxy.send(start("203.0.113.5")).await.status, StatusCode::OK, "another network");
    // The last address of a list counts: the one the proxy appended.
    let listed = request("POST", "/mintaka/api/ctf/start", &[("content-type", "application/json"), BROWSER, (VISITOR, "198.51.100.7, 192.0.2.44")], Bytes::from_static(b"{}"));
    assert_eq!(proxy.send(listed).await.status, StatusCode::OK);
}

/// Sends raw bytes and returns the start of the answer.
async fn raw(addr: std::net::SocketAddr, request: &[u8]) -> String {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let mut s = tokio::net::TcpStream::connect(addr).await.unwrap();
    let _ = s.write_all(request).await; // a refused head may close the connection mid-write
    let mut out = vec![0; 4096];
    let n = tokio::time::timeout(std::time::Duration::from_secs(5), s.read(&mut out)).await.unwrap().unwrap_or(0);
    String::from_utf8_lossy(&out[..n]).into_owned()
}

#[tokio::test]
async fn protocol_and_cross_site_guards() {
    let server = TestServer::start(config("guards")).await;
    let addr = server.addr();

    // RFC 9112 §3.2: HTTP/1.1 without Host is a 400; HTTP/1.0 may omit it.
    assert!(raw(addr, b"GET /en/ HTTP/1.1\r\nUser-Agent: x\r\n\r\n").await.starts_with("HTTP/1.1 400"));
    assert!(raw(addr, b"GET /en/ HTTP/1.0\r\nUser-Agent: x\r\n\r\n").await.starts_with("HTTP/1.0 200"));
    // Request heads are capped at 64 KiB.
    let big = format!("GET /en/ HTTP/1.1\r\nHost: x\r\nX-Big: {}\r\n\r\n", "v".repeat(70 * 1024));
    let answer = raw(addr, big.as_bytes()).await;
    assert!(answer.starts_with("HTTP/1.1 431"), "{answer:?}");

    // A brief posted from another site's page is refused; from our own pages (or by curl) it isn't.
    let mut client = Client::h1(addr).await;
    let form = |site: Option<&'static str>| {
        let mut headers = vec![("content-type", "application/x-www-form-urlencoded"), BROWSER];
        headers.extend(site.map(|v| ("sec-fetch-site", v)));
        request("POST", "/api/contact", &headers, Bytes::from_static(b"lang=en&name=&email=a%40b.tj"))
    };
    for site in ["cross-site", "same-site", "none"] {
        assert_eq!(client.send(form(Some(site))).await.status, StatusCode::FORBIDDEN, "{site}");
    }
    assert_eq!(client.send(form(Some("same-origin"))).await.status, StatusCode::SEE_OTHER);
    assert_eq!(client.send(form(None)).await.status, StatusCode::SEE_OTHER);
}
