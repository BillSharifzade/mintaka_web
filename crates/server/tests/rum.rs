//! `/_rum`: beacons in (strictly parsed, rate limited, GPC/DNT honoured), aggregates out.

mod common;

use bytes::Bytes;
use common::{BROWSER, Client, TestServer, assert_security_headers, config, request};
use http::StatusCode;
use proof::RumSummary;

const BEACON: &str = r#"{"route":"/en/","view":true,"metrics":{"ttfb":12.5,"fcp":80,"lcp":120,"cls":0,"inp":24},"events":{"proof_open":1,"render_tier:B":1}}"#;

async fn post(client: &mut Client, body: &str, headers: &[(&str, &str)]) -> common::Reply {
    // What sendBeacon(url, string) sends; `headers` replace these by name.
    let defaults = [("content-type", "text/plain;charset=UTF-8"), ("sec-fetch-site", "same-origin"), BROWSER];
    let mut all: Vec<_> = defaults.into_iter().filter(|(n, _)| !headers.iter().any(|(m, _)| m == n)).collect();
    all.extend_from_slice(headers);
    client.send(request("POST", "/_rum", &all, Bytes::from(body.to_owned()))).await
}

async fn summary(client: &mut Client) -> RumSummary {
    let reply = client.get("/_rum", &[BROWSER]).await;
    assert_eq!(reply.status, StatusCode::OK);
    assert_eq!(reply.header("cache-control"), "no-store");
    assert_eq!(reply.header("content-type"), "application/json");
    assert_security_headers(&reply, false, "GET /_rum");
    reply.json()
}

#[tokio::test]
async fn beacons_are_counted_and_summarised() {
    let server = TestServer::start(config("rum-count")).await;
    let mut client = Client::h1(server.addr()).await;

    let before = summary(&mut client).await;
    assert_eq!(before.start.views, 0);
    assert_eq!(before.min_n, 20);
    assert!(before.now >= before.since);

    for _ in 0..20 {
        let reply = post(&mut client, BEACON, &[]).await;
        assert_eq!(reply.status, StatusCode::NO_CONTENT);
        assert_eq!(reply.header("cache-control"), "no-store");
        assert!(reply.body.is_empty());
        assert!(!reply.headers.contains_key("set-cookie"));
        assert_security_headers(&reply, false, "POST /_rum");
    }
    // A later beacon of the same page view: events only, not another view.
    let reply = post(&mut client, r#"{"route":"/en/","events":{"contact_submit":1}}"#, &[]).await;
    assert_eq!(reply.status, StatusCode::NO_CONTENT);
    // JSON is accepted too.
    let reply = post(&mut client, r#"{"route":"404","view":true}"#, &[("content-type", "application/json")]).await;
    assert_eq!(reply.status, StatusCode::NO_CONTENT);

    let s = summary(&mut client).await;
    assert_eq!(s.start.views, 21);
    assert_eq!(s.start.routes["/en/"], 20);
    assert_eq!(s.start.routes["404"], 1);
    assert_eq!(s.start.metrics["lcp"].n, 20);
    // Rounded up to the histogram's resolution (1 % above 25.6 ms), exact below.
    let lcp = s.start.metrics["lcp"].p75.unwrap();
    assert!((120.0..=121.2).contains(&lcp), "{lcp}");
    assert_eq!(s.start.metrics["cls"].p75, Some(0.0));
    assert_eq!(s.start.metrics["ttfb"].p75, Some(12.5));
    assert_eq!(s.start.events["proof_open"], 20);
    assert_eq!(s.start.events["contact_submit"], 1);
    assert_eq!(s.day.views, 21);
    assert_eq!(s.rejected.invalid, 0);

    let head = client.send(request("HEAD", "/_rum", &[BROWSER], Bytes::new())).await;
    assert_eq!(head.status, StatusCode::OK);
    assert!(head.body.is_empty());
}

#[tokio::test]
async fn refusals_answer_without_counting() {
    let server = TestServer::start(config("rum-refuse")).await;
    let mut client = Client::h1(server.addr()).await;

    for (body, why) in [
        ("{", "not JSON"),
        (r#"{"route":"/en/","view":true,"ua":"x"}"#, "unknown field"),
        (r#"{"route":"/en/secret/","view":true}"#, "free-form path"),
        (r#"{"route":"/en/","view":true,"metrics":{"lcp":1e9}}"#, "out of range"),
        (r#"{"route":"/en/","events":{"page_view":1}}"#, "unknown event"),
        (r#"{"route":"/en/"}"#, "nothing to count"),
    ] {
        let reply = post(&mut client, body, &[]).await;
        assert_eq!(reply.status, StatusCode::BAD_REQUEST, "{why}");
        assert_eq!(reply.header("cache-control"), "no-store", "{why}");
        assert!(!reply.text().contains("secret"), "never echoes the request");
    }
    let reply = post(&mut client, BEACON, &[("content-type", "application/x-www-form-urlencoded")]).await;
    assert_eq!(reply.status, StatusCode::UNSUPPORTED_MEDIA_TYPE);
    let reply = post(&mut client, BEACON, &[("sec-fetch-site", "cross-site")]).await;
    assert_eq!(reply.status, StatusCode::FORBIDDEN);
    let big = format!(r#"{{"route":"/en/","view":true,"pad":"{}"}}"#, "x".repeat(3000));
    let reply = post(&mut client, &big, &[]).await;
    assert_eq!(reply.status, StatusCode::PAYLOAD_TOO_LARGE);

    // Global Privacy Control and Do Not Track: answered like success, counted nowhere.
    for header in [("sec-gpc", "1"), ("dnt", "1")] {
        let reply = post(&mut client, BEACON, &[header]).await;
        assert_eq!(reply.status, StatusCode::NO_CONTENT, "{header:?}");
    }

    let reply = client.send(request("PUT", "/_rum", &[BROWSER], Bytes::from_static(b"{}"))).await;
    assert_eq!(reply.status, StatusCode::METHOD_NOT_ALLOWED);
    assert_eq!(reply.header("allow"), "GET, HEAD, POST");

    let s = summary(&mut client).await;
    assert_eq!(s.start.views, 0);
    assert!(s.start.events.is_empty());
    assert_eq!(s.rejected.invalid, 9);
    assert_eq!(s.rejected.rate_limited, 0);
}

#[tokio::test]
async fn sixty_beacons_per_network_then_429() {
    let server = TestServer::start(config("rum-limit")).await;
    let mut client = Client::h2c(server.addr()).await;
    for i in 0..60 {
        assert_eq!(post(&mut client, BEACON, &[]).await.status, StatusCode::NO_CONTENT, "beacon {i}");
    }
    let reply = post(&mut client, BEACON, &[]).await;
    assert_eq!(reply.status, StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(reply.header("cache-control"), "no-store");
    assert_eq!(reply.header("retry-after"), "60");
    let s = summary(&mut client).await;
    assert_eq!(s.start.views, 60);
    assert_eq!(s.rejected.rate_limited, 1);
}
