//! Plain-HTTP behaviour against the fixture site: headers on every response type, routing,
//! negotiation, caching, curl mode, `/_proof`, `/_ping`, `/_telemetry`.

mod common;

use std::time::Duration;

use bytes::Bytes;
use common::{BROWSER, Client, SECURITY_HEADERS, TestServer, assert_security_headers, config, fixture_dist, get, request};
use http::StatusCode;
use http_body_util::BodyExt;

fn fixture(path: &str) -> Vec<u8> {
    std::fs::read(fixture_dist().join(path)).unwrap()
}

fn manifest() -> proof::Manifest {
    serde_json::from_slice(&fixture("manifest.json")).unwrap()
}

#[tokio::test]
async fn every_response_type_carries_the_security_headers() {
    let server = TestServer::start(config("headers")).await;
    let addr = server.addr();
    let etag = get(addr, "/en/", &[BROWSER]).await.header("etag").to_owned();

    let cases: Vec<(&str, http::Request<_>, StatusCode)> = vec![
        ("static page", request("GET", "/en/", &[BROWSER], Bytes::new()), StatusCode::OK),
        ("negotiated root", request("GET", "/", &[BROWSER], Bytes::new()), StatusCode::OK),
        ("asset", request("GET", "/assets/main.3F2A1B9C.js", &[BROWSER], Bytes::new()), StatusCode::OK),
        ("HEAD", request("HEAD", "/robots.txt", &[BROWSER], Bytes::new()), StatusCode::OK),
        ("304", request("GET", "/en/", &[BROWSER, ("if-none-match", &etag)], Bytes::new()), StatusCode::NOT_MODIFIED),
        ("404", request("GET", "/no/such/page", &[BROWSER], Bytes::new()), StatusCode::NOT_FOUND),
        ("405", request("DELETE", "/en/", &[BROWSER], Bytes::new()), StatusCode::METHOD_NOT_ALLOWED),
        ("redirect", request("GET", "/en", &[BROWSER], Bytes::new()), StatusCode::MOVED_PERMANENTLY),
        ("proof", request("GET", "/_proof", &[BROWSER], Bytes::new()), StatusCode::OK),
        ("ping", request("GET", "/_ping", &[BROWSER], Bytes::new()), StatusCode::NO_CONTENT),
        ("pow", request("GET", "/api/pow", &[BROWSER], Bytes::new()), StatusCode::OK),
        (
            "contact 422",
            request("POST", "/api/contact", &[("content-type", "application/json")], Bytes::from_static(b"{}")),
            StatusCode::UNPROCESSABLE_ENTITY,
        ),
        ("contact 405", request("GET", "/api/contact", &[BROWSER], Bytes::new()), StatusCode::METHOD_NOT_ALLOWED),
        ("curl text", request("GET", "/", &[("user-agent", "curl/8.23.0")], Bytes::new()), StatusCode::OK),
    ];
    for (what, req, status) in cases {
        let reply = Client::h1(addr).await.send(req).await;
        assert_eq!(reply.status, status, "{what}");
        assert_security_headers(&reply, false, what);
    }

    // The event stream: headers arrive before the body ends.
    let mut client = Client::h1(addr).await;
    let response = client.stream(request("GET", "/_telemetry", &[BROWSER], Bytes::new())).await;
    let (parts, _body) = response.into_parts();
    let reply = common::Reply { status: parts.status, version: parts.version, headers: parts.headers, body: Bytes::new() };
    assert_eq!(reply.status, StatusCode::OK);
    assert_security_headers(&reply, false, "SSE");
}

#[tokio::test]
async fn csp_is_assembled_from_the_manifest_hashes() {
    let server = TestServer::start(config("csp")).await;
    let reply = get(server.addr(), "/en/", &[BROWSER]).await;
    let hashes: Vec<String> = manifest().csp.style_src.iter().map(|h| format!("'{h}'")).collect();
    assert_eq!(hashes.len(), 1);
    let expected = format!(
        "default-src 'none'; script-src 'self' 'wasm-unsafe-eval'; style-src 'self' {}; img-src 'self'; \
         font-src 'self'; connect-src 'self'; worker-src 'self'; manifest-src 'self'; form-action 'self'; \
         base-uri 'none'; frame-ancestors 'none'; require-trusted-types-for 'script'; trusted-types mintaka",
        hashes.join(" ")
    );
    assert_eq!(reply.header("content-security-policy"), expected);
    // The hash really is the fixture page's inline style.
    let html = reply.text();
    let style = html.split("<style>").nth(1).unwrap().split("</style>").next().unwrap();
    use base64::Engine;
    use sha2::Digest;
    let digest = base64::engine::general_purpose::STANDARD.encode(sha2::Sha256::digest(style.as_bytes()));
    assert_eq!(hashes[0], format!("'sha256-{digest}'"));
}

#[tokio::test]
async fn static_routes_serve_the_exact_bytes_with_cache_policy() {
    let server = TestServer::start(config("static")).await;
    let addr = server.addr();
    let page = get(addr, "/en/", &[BROWSER]).await;
    assert_eq!(page.body, fixture("en/index.html"));
    assert_eq!(page.header("content-type"), "text/html; charset=utf-8");
    assert_eq!(page.header("cache-control"), "no-cache");
    assert_eq!(page.header("content-language"), "en");
    assert_eq!(page.header("content-length"), page.body.len().to_string());
    assert!(page.header("etag").starts_with('"') && page.header("etag").ends_with('"'));

    let asset = get(addr, "/assets/main.3F2A1B9C.js", &[BROWSER]).await;
    assert_eq!(asset.header("cache-control"), "public, max-age=31536000, immutable");
    assert_eq!(asset.header("content-type"), "text/javascript; charset=utf-8");
    let short = get(addr, "/.well-known/security.txt", &[BROWSER]).await;
    assert_eq!(short.header("cache-control"), "public, max-age=3600");
    assert_eq!(short.body, fixture(".well-known/security.txt"));
    assert!(short.header("vary").is_empty(), "no encodings, no curl text: no Vary");
    // manifest.json itself is not served.
    assert_eq!(get(addr, "/manifest.json", &[BROWSER]).await.status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn head_works_wherever_get_does() {
    let server = TestServer::start(config("head")).await;
    let mut client = Client::h1(server.addr()).await;
    for path in ["/en/", "/", "/assets/main.3F2A1B9C.js", "/_proof", "/_ping", "/api/pow", "/nope"] {
        let get = client.get(path, &[BROWSER]).await;
        let head = client.send(request("HEAD", path, &[BROWSER], Bytes::new())).await;
        assert_eq!(head.status, get.status, "{path}");
        assert!(head.body.is_empty(), "{path}: HEAD has no body");
        if get.status == StatusCode::OK && path != "/_proof" && path != "/api/pow" {
            assert_eq!(head.header("content-length"), get.body.len().to_string(), "{path}");
        }
    }
}

#[tokio::test]
async fn accept_language_negotiation_at_the_root() {
    let server = TestServer::start(config("lang")).await;
    let addr = server.addr();
    let cases = [
        (None, "en"),
        (Some("ru-RU,ru;q=0.9,en-US;q=0.8,en;q=0.7"), "ru"),
        (Some("de-DE,de;q=0.9,ru;q=0.5,en;q=0.4"), "ru"),
        (Some("en-GB,en;q=0.9,ru;q=0.8"), "en"),
        (Some("fr-FR"), "en"),
        (Some("ru;q=0, *;q=0.1"), "en"),
        (Some("en;q=0, *;q=0.1"), "ru"),
        (Some("RU"), "ru"),
    ];
    for (accept, lang) in cases {
        let mut headers = vec![BROWSER];
        if let Some(a) = accept {
            headers.push(("accept-language", a));
        }
        let reply = get(addr, "/", &headers).await;
        assert_eq!(reply.status, StatusCode::OK, "{accept:?}");
        assert_eq!(reply.header("content-language"), lang, "{accept:?}");
        assert_eq!(reply.body, fixture(&format!("{lang}/index.html")), "{accept:?}: served directly, no redirect");
        let vary = reply.header("vary");
        assert!(vary.contains("Accept-Language") && vary.contains("Accept-Encoding"), "{vary}");
        assert!(reply.text().contains(&format!(r#"<link rel="canonical" href="/{lang}/">"#)));
    }
}

#[tokio::test]
async fn accept_encoding_selects_the_precompressed_variant() {
    let server = TestServer::start(config("encoding")).await;
    let addr = server.addr();
    let cases = [
        (None, "", "en/index.html"),
        (Some("gzip, deflate, br, zstd"), "br", "en/index.html.br"),
        (Some("gzip;q=1.0, br;q=0.5"), "br", "en/index.html.br"),
        (Some("gzip, br;q=0"), "gzip", "en/index.html.gz"),
        (Some("deflate"), "", "en/index.html"),
        (Some("*"), "br", "en/index.html.br"),
        (Some("br;q=0, gzip;q=0"), "", "en/index.html"),
        (Some("identity"), "", "en/index.html"),
    ];
    let mut etags = std::collections::HashSet::new();
    for (accept, coding, file) in cases {
        let mut headers = vec![BROWSER];
        if let Some(a) = accept {
            headers.push(("accept-encoding", a));
        }
        let reply = get(addr, "/en/", &headers).await;
        assert_eq!(reply.header("content-encoding"), coding, "{accept:?}");
        assert_eq!(reply.body, fixture(file), "{accept:?}");
        assert_eq!(reply.header("vary"), "Accept-Encoding, User-Agent", "{accept:?}");
        etags.insert(reply.header("etag").to_owned());
    }
    assert_eq!(etags.len(), 3, "one strong validator per representation");
    // `/ru/` only has a gzip sibling.
    let ru = get(addr, "/ru/", &[BROWSER, ("accept-encoding", "br, gzip")]).await;
    assert_eq!(ru.header("content-encoding"), "gzip");
    // No encodings at all: no Content-Encoding, no Accept-Encoding in Vary.
    let robots = get(addr, "/robots.txt", &[BROWSER, ("accept-encoding", "br")]).await;
    assert_eq!(robots.header("content-encoding"), "");
    assert!(!robots.header("vary").contains("Accept-Encoding"));
}

#[tokio::test]
async fn if_none_match_gives_304() {
    let server = TestServer::start(config("etag")).await;
    let addr = server.addr();
    for accept in ["br", "gzip", "identity"] {
        let first = get(addr, "/en/", &[BROWSER, ("accept-encoding", accept)]).await;
        let etag = first.header("etag").to_owned();
        for inm in [etag.clone(), format!("W/{etag}"), format!("\"other\", {etag}"), "*".to_owned()] {
            let again = get(addr, "/en/", &[BROWSER, ("accept-encoding", accept), ("if-none-match", &inm)]).await;
            assert_eq!(again.status, StatusCode::NOT_MODIFIED, "{accept} {inm}");
            assert!(again.body.is_empty());
            assert_eq!(again.header("etag"), etag);
            assert_eq!(again.header("cache-control"), "no-cache");
            assert_eq!(again.header("vary"), "Accept-Encoding, User-Agent");
            assert!(!again.headers.contains_key("content-length"), "304 without Content-Length");
        }
        let stale = get(addr, "/en/", &[BROWSER, ("accept-encoding", accept), ("if-none-match", "\"stale\"")]).await;
        assert_eq!(stale.status, StatusCode::OK);
    }
    // A validator of the br representation does not validate the identity one.
    let br = get(addr, "/en/", &[BROWSER, ("accept-encoding", "br")]).await;
    let identity = get(addr, "/en/", &[BROWSER, ("if-none-match", br.header("etag"))]).await;
    assert_eq!(identity.status, StatusCode::OK);
}

#[tokio::test]
async fn not_found_suggests_the_closest_page_and_never_echoes_the_path() {
    let server = TestServer::start(config("404")).await;
    let addr = server.addr();

    let typo = get(addr, "/en/snt/", &[BROWSER]).await;
    assert_eq!(typo.status, StatusCode::NOT_FOUND);
    assert_eq!(typo.header("content-type"), "text/html; charset=utf-8");
    assert_eq!(typo.header("content-language"), "en");
    let body = typo.text();
    assert!(body.contains(r#"<p>The closest star: <a href="/en/sent/">/en/sent/</a></p>"#), "{body}");
    assert!(!body.contains("<!--mintaka:suggest-->"));
    assert!(!body.contains("/en/snt/"));

    // Language from the path prefix.
    let ru = get(addr, "/ru/nyet", &[BROWSER]).await;
    assert_eq!(ru.header("content-language"), "ru");
    assert!(ru.text().contains(r#"<a href="/ru/">/ru/</a>"#), "{}", ru.text());
    // …else negotiated.
    let negotiated = get(addr, "/xx/", &[BROWSER, ("accept-language", "ru")]).await;
    assert_eq!(negotiated.header("content-language"), "ru");
    assert!(negotiated.header("vary").contains("Accept-Language"));
    assert!(negotiated.text().contains(r#"<a href="/ru/">/ru/</a>"#), "ties prefer the page's language");

    // Unrelated paths get no suggestion, and hostile paths are never reflected.
    for probe in ["/zz-unique-probe-7f3a9c/really-long-and-unrelated", "/%3Cscript%3Ealert(1)%3C/script%3E", "/wp-login.php"] {
        let reply = get(addr, probe, &[BROWSER]).await;
        assert_eq!(reply.status, StatusCode::NOT_FOUND);
        let body = reply.text();
        assert!(!body.contains(probe), "{probe} echoed");
        assert!(!body.contains("zz-unique-probe") && !body.contains("script%3E") && !body.contains("wp-login"));
        assert!(body.contains("<p>The closest star: </p>"), "{probe}: {body}");
    }
}

#[tokio::test]
async fn lang_without_slash_redirects() {
    let server = TestServer::start(config("slash")).await;
    let addr = server.addr();
    let reply = get(addr, "/en", &[BROWSER]).await;
    assert_eq!(reply.status, StatusCode::MOVED_PERMANENTLY);
    assert_eq!(reply.header("location"), "/en/");
    let query = get(addr, "/ru?utm=x", &[BROWSER]).await;
    assert_eq!(query.header("location"), "/ru/?utm=x");
    assert_eq!(get(addr, "/en/sent", &[BROWSER]).await.header("location"), "/en/sent/");
    assert_eq!(get(addr, "/fr", &[BROWSER]).await.status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn other_methods_get_405_with_allow() {
    let server = TestServer::start(config("405")).await;
    let addr = server.addr();
    for (method, path, allow) in [
        ("POST", "/en/", "GET, HEAD"),
        ("PUT", "/assets/main.3F2A1B9C.js", "GET, HEAD"),
        ("DELETE", "/", "GET, HEAD"),
        ("OPTIONS", "/en", "GET, HEAD"),
        ("POST", "/_proof", "GET, HEAD"),
        ("POST", "/_ping", "GET, HEAD"),
        ("POST", "/_telemetry", "GET, HEAD"),
        ("POST", "/api/pow", "GET, HEAD"),
        ("GET", "/api/contact", "POST"),
        ("PUT", "/api/contact", "POST"),
    ] {
        let reply = Client::h1(addr).await.send(request(method, path, &[BROWSER], Bytes::new())).await;
        assert_eq!(reply.status, StatusCode::METHOD_NOT_ALLOWED, "{method} {path}");
        assert_eq!(reply.header("allow"), allow, "{method} {path}");
        assert_eq!(reply.header("cache-control"), "no-store", "{method} {path}");
    }
    // Unknown paths are 404 whatever the method.
    let reply = Client::h1(addr).await.send(request("POST", "/nope", &[BROWSER], Bytes::new())).await;
    assert_eq!(reply.status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn curl_mode_serves_text_with_tokens_filled() {
    let server = TestServer::start(config("curl")).await;
    let addr = server.addr();
    for agent in ["curl/8.23.0", "Wget/1.25.0", "HTTPie/3.2.4", "xh/0.24.0"] {
        for path in ["/", "/en/"] {
            let reply = get(addr, path, &[("user-agent", agent)]).await;
            assert_eq!(reply.status, StatusCode::OK, "{agent} {path}");
            assert_eq!(reply.header("content-type"), "text/plain; charset=utf-8");
            assert!(reply.header("vary").contains("User-Agent"));
            let text = reply.text();
            assert!(!text.contains("{{"), "{agent} {path}: {text}");
            assert!(text.contains("\x1b[38;2;61;123;255mmintaka"), "ANSI colour kept");
            assert!(text.contains("plain HTTP (development)"));
            assert!(text.contains("HTTP/1.1"));
            assert!(text.contains("build 2026-10-27.f1x7ure0"));
            let served = text.split("served in ").nth(1).unwrap().split(" · ").next().unwrap();
            assert!(served.ends_with(" µs") || served.ends_with(" ms"), "{served}");
        }
    }
    let pillars = get(addr, "/pillars", &[("user-agent", "curl/8")]).await;
    assert!(pillars.text().contains("fast by proof"));
    let ru = get(addr, "/ru/", &[("user-agent", "curl/8")]).await;
    assert_eq!(ru.header("content-language"), "ru");
    assert!(ru.text().contains("программы"));
    // `/ru/pillars` has no Russian text yet: falls back to English.
    let fallback = get(addr, "/ru/pillars", &[("user-agent", "curl/8")]).await;
    assert_eq!(fallback.header("content-language"), "en");
    // Over HTTP/2 the token says so.
    let h2 = Client::h2c(addr).await.get("/", &[("user-agent", "curl/8")]).await;
    assert!(h2.text().contains("HTTP/2"), "{}", h2.text());

    // Browsers get HTML / the normal 404 on the same paths.
    assert!(get(addr, "/", &[BROWSER]).await.text().starts_with("<!doctype html>"));
    for path in ["/proof", "/pillars", "/contact", "/en/pillars"] {
        let reply = get(addr, path, &[BROWSER]).await;
        assert_eq!(reply.status, StatusCode::NOT_FOUND, "{path}");
        assert!(reply.header("vary").contains("User-Agent"), "{path}");
    }
}

#[tokio::test]
async fn curl_proof_is_generated_from_the_facts() {
    let server = TestServer::start(config("curl-proof")).await;
    let reply = get(server.addr(), "/proof", &[("user-agent", "curl/8.23.0")]).await;
    assert_eq!(reply.status, StatusCode::OK);
    let text = reply.text();
    for needle in [
        "\x1b[38;2;201;206;216mserver      \x1b[0m",
        "test · Dushanbe, TJ (38.5598, 68.787)",
        "2026-10-27.f1x7ure0",
        "mintaka-server 0.1.0 · rustc ",
        "\x1b[38;2;226;85;45mplain HTTP (development)",
        "HTTP/1.1",
        "\x1b[38;2;47;191;113m0 cookies · 0 third-party origins · IP not stored · IP not logged",
        "content-security-policy",
        "x-content-type-options",
        "\x1b[38;2;61;123;255m",
        "\x1b[38;2;244;245;247m",
    ] {
        assert!(text.contains(needle), "missing {needle:?} in\n{text}");
    }
    assert!(!text.contains("strict-transport-security"), "headers listed are the ones sent");
    // Labels are aligned: every fact line starts with the label column.
    let fact_lines = text.lines().filter(|l| l.starts_with("  \x1b[38;2;201;206;216m")).count();
    assert!(fact_lines >= 12, "{fact_lines} lines\n{text}");
}

#[tokio::test]
async fn proof_reports_this_connection() {
    let server = TestServer::start(config("proof")).await;
    let mut client = Client::h1(server.addr()).await;
    let first = client.get("/_proof", &[BROWSER]).await;
    assert_eq!(first.header("content-type"), "application/json");
    assert_eq!(first.header("cache-control"), "no-store");
    let p: proof::Proof = first.json();
    assert_eq!(p.server.region, "test");
    assert_eq!(p.server.location.as_ref().unwrap().city, "Dushanbe");
    assert_eq!(p.server.build, "2026-10-27.f1x7ure0");
    assert!(p.server.software.starts_with("mintaka-server 0.1.0 · rustc "), "{}", p.server.software);
    assert!(p.server.software.contains(" · axum 0.8 · rustls 0.23"), "{}", p.server.software);
    assert!(p.server.cores >= 1);
    assert!(p.server.time_us < 1_000_000);
    assert_eq!(p.tls, None);
    assert_eq!(p.http.version, "HTTP/1.1");
    assert_eq!(p.connection.requests, 1);
    #[cfg(target_os = "linux")]
    assert!(p.connection.kernel_rtt_us.is_some_and(|rtt| rtt > 0));
    assert_eq!((p.privacy.cookies_set, p.privacy.third_party_origins), (0, 0));
    assert!(!p.privacy.ip_stored && !p.privacy.ip_logged);
    // `headers` names exactly the security headers on this response.
    assert_eq!(p.headers, SECURITY_HEADERS);
    for name in &p.headers {
        assert!(first.headers.contains_key(name.as_str()), "{name} listed but not sent");
    }

    let second: proof::Proof = client.get("/_proof", &[BROWSER]).await.json();
    assert_eq!(second.connection.id, p.connection.id, "same connection");
    assert_eq!(second.connection.requests, 2);
    let other: proof::Proof = Client::h1(server.addr()).await.get("/_proof", &[BROWSER]).await.json();
    assert_ne!(other.connection.id, p.connection.id);

    let h2: proof::Proof = Client::h2c(server.addr()).await.get("/_proof", &[BROWSER]).await.json();
    assert_eq!(h2.http.version, "HTTP/2.0");
}

#[tokio::test]
async fn ping_is_204_no_store() {
    let server = TestServer::start(config("ping")).await;
    let mut client = Client::h1(server.addr()).await;
    for method in ["GET", "HEAD"] {
        let reply = client.send(request(method, "/_ping", &[BROWSER], Bytes::new())).await;
        assert_eq!(reply.status, StatusCode::NO_CONTENT);
        assert_eq!(reply.header("cache-control"), "no-store");
        assert!(reply.body.is_empty());
        assert!(!reply.headers.contains_key("content-length"), "204 must not carry Content-Length");
        assert!(reply.timing().contains_key("app"));
    }
}

/// Reads SSE frames until `count` events (not comments) have arrived.
async fn read_events(body: &mut hyper::body::Incoming, count: usize) -> Vec<(String, String)> {
    let mut buffer = String::new();
    let mut events = Vec::new();
    while events.len() < count {
        let frame = tokio::time::timeout(Duration::from_secs(5), body.frame()).await.expect("event within 5 s");
        let frame = frame.expect("stream open").expect("frame");
        if let Ok(data) = frame.into_data() {
            buffer.push_str(std::str::from_utf8(&data).unwrap());
        }
        while let Some(end) = buffer.find("\n\n") {
            let block: String = buffer.drain(..end + 2).collect();
            let mut event = None;
            let mut data = None;
            for line in block.lines() {
                if let Some(e) = line.strip_prefix("event: ") {
                    event = Some(e.to_owned());
                } else if let Some(d) = line.strip_prefix("data: ") {
                    data = Some(d.to_owned());
                }
            }
            if let (Some(e), Some(d)) = (event, data) {
                events.push((e, d));
            }
        }
    }
    events
}

#[tokio::test]
async fn telemetry_streams_hello_then_ticks() {
    let server = TestServer::start(config("sse")).await;
    let addr = server.addr();
    for _ in 0..5 {
        get(addr, "/en/", &[BROWSER]).await;
    }
    let mut client = Client::h1(addr).await;
    let response = client.stream(request("GET", "/_telemetry", &[BROWSER], Bytes::new())).await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()["content-type"], "text/event-stream");
    assert_eq!(response.headers()["cache-control"], "no-store");
    let mut body = response.into_body();
    let events = read_events(&mut body, 3).await;
    assert_eq!(events[0].0, "hello");
    let hello: proof::TelemetryHello = serde_json::from_str(&events[0].1).unwrap();
    assert_eq!(hello.build, "2026-10-27.f1x7ure0");
    assert_eq!(hello.region, "test");
    assert!(hello.cores >= 1);
    let ticks: Vec<proof::TelemetryTick> =
        events[1..].iter().map(|(e, d)| {
            assert_eq!(e, "tick");
            serde_json::from_str(d).unwrap()
        }).collect();
    let last = ticks.last().unwrap();
    assert!(last.req_total >= 5, "{last:?}");
    assert!(last.conns >= 1);
    assert!(last.rss_bytes > 0);
    assert!(last.p50_us.is_some(), "the five page requests are in the 10 s window: {last:?}");
    assert!(last.p50_us <= last.p99_us && last.p99_us <= last.p999_us);
    assert!(ticks.windows(2).all(|w| w[1].t > w[0].t), "one tick per second");
}

#[tokio::test]
async fn telemetry_streams_are_capped() {
    let mut cfg = config("sse-cap");
    cfg.max_telemetry_streams = 2;
    let server = TestServer::start(cfg).await;
    let mut client = Client::h2c(server.addr()).await;
    let a = client.stream(request("GET", "/_telemetry", &[BROWSER], Bytes::new())).await;
    let b = client.stream(request("GET", "/_telemetry", &[BROWSER], Bytes::new())).await;
    assert_eq!((a.status(), b.status()), (StatusCode::OK, StatusCode::OK));
    let c = client.send(request("GET", "/_telemetry", &[BROWSER], Bytes::new())).await;
    assert_eq!(c.status, StatusCode::SERVICE_UNAVAILABLE);
    assert_security_headers(&c, false, "503");
    drop(a);
    // The slot frees once the server notices the stream is gone.
    let mut status = StatusCode::SERVICE_UNAVAILABLE;
    for _ in 0..50 {
        tokio::time::sleep(Duration::from_millis(20)).await;
        let d = client.stream(request("GET", "/_telemetry", &[BROWSER], Bytes::new())).await;
        status = d.status();
        if status == StatusCode::OK {
            break;
        }
    }
    assert_eq!(status, StatusCode::OK);
    drop(b);
}

#[tokio::test]
async fn telemetry_excludes_its_own_stream_from_latency() {
    let server = TestServer::start(config("sse-exclude")).await;
    let mut client = Client::h1(server.addr()).await;
    let response = client.stream(request("GET", "/_telemetry", &[BROWSER], Bytes::new())).await;
    let mut body = response.into_body();
    // No other traffic: the stream request itself is counted but never timed.
    let events = read_events(&mut body, 3).await;
    let tick: proof::TelemetryTick = serde_json::from_str(&events[2].1).unwrap();
    assert_eq!(tick.req_total, 1, "{tick:?}");
    assert_eq!(tick.p50_us, None, "{tick:?}");
}

#[tokio::test]
async fn server_timing_tls_metric_is_absent_on_plain_http() {
    let server = TestServer::start(config("timing")).await;
    let reply = get(server.addr(), "/en/", &[BROWSER]).await;
    let timing = reply.timing();
    assert!(!timing.contains_key("tls"));
    assert!(timing["app"] >= 0.0 && timing["app"] < 1000.0);
    let raw = reply.header("server-timing");
    assert!(raw.starts_with("app;dur="), "{raw}");
    for value in raw.split(", ").filter_map(|m| m.split(';').nth(1)) {
        let digits = value.strip_prefix("dur=").unwrap();
        let decimals = digits.split('.').nth(1).map_or(0, str::len);
        assert!(decimals <= 3, "{raw}");
    }
}

#[tokio::test]
async fn shutdown_returns_cleanly_with_an_open_event_stream() {
    let server = TestServer::start(config("shutdown")).await;
    let mut client = Client::h1(server.addr()).await;
    let response = client.stream(request("GET", "/_telemetry", &[BROWSER], Bytes::new())).await;
    let mut body = response.into_body();
    let started = std::time::Instant::now();
    server.stop().await.unwrap();
    assert!(started.elapsed() < Duration::from_secs(2), "event streams end at shutdown instead of holding the grace period");
    // The stream ends (cleanly or not) once the server is gone.
    let end = tokio::time::timeout(Duration::from_secs(3), async {
        while let Some(Ok(_)) = body.frame().await {}
    })
    .await;
    assert!(end.is_ok());
}

#[tokio::test]
async fn http2_bodiless_responses_have_the_right_framing() {
    let server = TestServer::start(config("h2-bodiless")).await;
    let mut h2 = Client::h2c(server.addr()).await;
    let ping = h2.get("/_ping", &[BROWSER]).await;
    assert_eq!((ping.status, ping.version), (StatusCode::NO_CONTENT, http::Version::HTTP_2));
    assert!(!ping.headers.contains_key("content-length"));
    assert_security_headers(&ping, false, "h2 204");

    let etag = h2.get("/en/", &[BROWSER, ("accept-encoding", "br")]).await.header("etag").to_owned();
    let not_modified = h2.get("/en/", &[BROWSER, ("accept-encoding", "br"), ("if-none-match", &etag)]).await;
    assert_eq!(not_modified.status, StatusCode::NOT_MODIFIED);
    assert!(!not_modified.headers.contains_key("content-length"));
    assert_eq!(not_modified.header("etag"), etag);

    let head = h2.send(request("HEAD", "/en/", &[BROWSER, ("accept-encoding", "br")], Bytes::new())).await;
    assert!(head.body.is_empty());
    assert_eq!(head.header("content-length"), fixture("en/index.html.br").len().to_string());
    assert_eq!(head.header("content-encoding"), "br");
}
