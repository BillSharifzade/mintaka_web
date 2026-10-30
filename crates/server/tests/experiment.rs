//! The A/B test (docs/CONTRACTS.md §17) end to end: a variant drawn for every page load, each with
//! its own bytes, encodings and validators, privately cacheable; curl mode untouched; beacons
//! counted per variant; and a verdict only once every variant has its planned page views.

mod common;

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use bytes::Bytes;
use common::{BROWSER, Client, Reply, TestServer, assert_security_headers, config, fixture_dist, get, request, temp_dir};
use http::StatusCode;
use proof::RumSummary;

const VARIANTS: [&str; 3] = ["hero-title", "hero-title-hardware", "hero-title-cycle"];
const TITLES: [&str; 3] = ["Software at the edge of physics.", "As fast as the hardware allows.", "Engineered to the last cycle."];
const FILES: [&str; 3] = ["en/index.html", "en/index.hero-title-hardware.html", "en/index.hero-title-cycle.html"];

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

/// A copy of the fixture site whose `/en/` tests its headline, stopping at `sample_size` page
/// views per variant; `edit` may then change the manifest.
fn dist(name: &str, sample_size: u64, edit: impl FnOnce(&mut serde_json::Value)) -> PathBuf {
    let dir = temp_dir(name);
    copy_dir(&fixture_dist(), &dir);
    let path = dir.join("manifest.json");
    let mut m: serde_json::Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    assert_eq!(m["routes"][0]["path"], "/en/");
    m["routes"][0]["variants"] = serde_json::json!([
        { "id": VARIANTS[1], "file": FILES[1], "encodings": ["br", "gzip"] },
        { "id": VARIANTS[2], "file": FILES[2], "encodings": ["br", "gzip"] },
    ]);
    m["experiment"] = serde_json::json!({
        "id": "hero-headline",
        "variants": VARIANTS,
        "metric": "cta_start_project",
        "secondary": ["contact_submit"],
        "plan": { "baseline": 0.05, "mde": 0.3, "alpha": 0.05, "power": 0.8, "sample_size": sample_size },
    });
    edit(&mut m);
    std::fs::write(&path, serde_json::to_vec_pretty(&m).unwrap()).unwrap();
    dir
}

async fn start(name: &str, sample_size: u64) -> TestServer {
    let mut cfg = config(name);
    cfg.dist = dist(name, sample_size, |_| {});
    TestServer::start(cfg).await
}

fn fixture(path: &str) -> Vec<u8> {
    std::fs::read(fixture_dist().join(path)).unwrap()
}

/// Which variant a response carries, by its bytes (identity, or the precompressed sibling its
/// `Content-Encoding` names).
fn variant_of(reply: &Reply) -> usize {
    let suffix = match reply.header("content-encoding") {
        "br" => ".br",
        "gzip" => ".gz",
        _ => "",
    };
    (0..3)
        .find(|&i| reply.body == fixture(&format!("{}{suffix}", FILES[i])))
        .unwrap_or_else(|| panic!("not one of the variants: {:?}", reply.headers))
}

/// Pearson's chi-square of `counts` against equal shares.
fn chi_square(counts: &[u32]) -> f64 {
    let expected = f64::from(counts.iter().sum::<u32>()) / counts.len() as f64;
    counts.iter().map(|&n| (f64::from(n) - expected).powi(2) / expected).sum()
}

#[tokio::test]
async fn every_page_load_draws_a_variant() {
    let server = start("ab-serve", 4578).await;
    let mut client = Client::h1(server.addr()).await;

    let mut counts = [0u32; 3];
    let mut etags: [Option<String>; 3] = Default::default();
    for _ in 0..600 {
        let reply = client.get("/en/", &[BROWSER]).await;
        assert_eq!(reply.status, StatusCode::OK);
        let v = variant_of(&reply);
        assert!(reply.text().contains(&format!("<h1>{}</h1>", TITLES[v])));
        assert_eq!(reply.header("cache-control"), "private, no-cache", "no shared cache may keep one variant for everybody");
        assert_eq!(reply.header("vary"), "Accept-Encoding, User-Agent");
        assert_eq!(reply.header("content-language"), "en");
        assert_eq!(reply.header("content-length"), reply.body.len().to_string());
        let etag = reply.header("etag").to_owned();
        assert_eq!(etags[v].get_or_insert_with(|| etag.clone()), &etag, "one validator per variant");
        counts[v] += 1;
    }
    assert_eq!(etags.iter().flatten().collect::<HashSet<_>>().len(), 3, "{etags:?}");
    // Uniform: 2 degrees of freedom, and chance exceeds 30 with probability 3e-7.
    assert!(chi_square(&counts) < 30.0, "{counts:?}");
    assert_security_headers(&client.get("/en/", &[BROWSER]).await, false, "a variant");

    // Each variant has its own precompressed siblings, and a validator per coding.
    let mut seen = HashSet::new();
    for _ in 0..60 {
        let br = client.get("/en/", &[BROWSER, ("accept-encoding", "gzip, br")]).await;
        assert_eq!(br.header("content-encoding"), "br");
        let v = variant_of(&br);
        let identity = etags[v].as_deref().unwrap();
        assert_eq!(br.header("etag"), format!("{}-br\"", identity.trim_end_matches('"')));
        assert_eq!(br.header("cache-control"), "private, no-cache");
        seen.insert(v);
    }
    assert_eq!(seen.len(), 3);

    // `/` serves the negotiated language's page, a page of the test like `/en/`.
    let mut roots = HashSet::new();
    for _ in 0..60 {
        let root = client.get("/", &[BROWSER]).await;
        assert_eq!(root.header("cache-control"), "private, no-cache");
        assert_eq!(root.header("vary"), "Accept-Encoding, Accept-Language, User-Agent");
        roots.insert(variant_of(&root));
    }
    assert_eq!(roots.len(), 3);

    // HEAD: the headers of a variant, drawn the same way, without its body.
    let lengths: Vec<String> = FILES.iter().map(|f| fixture(f).len().to_string()).collect();
    for _ in 0..20 {
        let head = client.send(request("HEAD", "/en/", &[BROWSER], Bytes::new())).await;
        assert_eq!(head.status, StatusCode::OK);
        assert!(head.body.is_empty());
        assert!(lengths.iter().any(|l| l == head.header("content-length")), "{:?}", head.headers);
        assert_eq!(head.header("cache-control"), "private, no-cache");
    }

    // Pages outside the test keep their own policy.
    for path in ["/ru/", "/en/sent/"] {
        assert_eq!(client.get(path, &[BROWSER]).await.header("cache-control"), "no-cache", "{path}");
    }
    // curl mode has no variants.
    for path in ["/", "/en/"] {
        let text = get(server.addr(), path, &[("user-agent", "curl/8.23.0")]).await;
        assert_eq!(text.header("content-type"), "text/plain; charset=utf-8", "{path}");
        assert_eq!(text.header("cache-control"), "no-store", "{path}");
        assert!(text.text().contains("software at the edge of physics"), "{path}");
    }
}

#[tokio::test]
async fn a_cached_variant_is_confirmed_only_when_drawn_again() {
    let server = start("ab-etag", 4578).await;
    let mut client = Client::h1(server.addr()).await;
    let cached = loop {
        let reply = client.get("/en/", &[BROWSER]).await;
        if variant_of(&reply) == 1 {
            break reply;
        }
    };
    let etag = cached.header("etag").to_owned();
    let (mut confirmed, mut replaced) = (0, [0u32; 3]);
    for _ in 0..300 {
        let reply = client.get("/en/", &[BROWSER, ("if-none-match", &etag)]).await;
        match reply.status {
            StatusCode::NOT_MODIFIED => {
                assert_eq!(reply.header("etag"), etag, "a 304 confirms exactly the cached variant");
                assert_eq!(reply.header("cache-control"), "private, no-cache");
                assert!(reply.body.is_empty());
                confirmed += 1;
            }
            StatusCode::OK => {
                let v = variant_of(&reply);
                assert_ne!(v, 1, "the cached variant would have been a 304");
                assert_ne!(reply.header("etag"), etag);
                replaced[v] += 1;
            }
            other => panic!("{other}"),
        }
    }
    // The draw ignores what the browser has: a third confirmed, the rest replaced evenly.
    assert!(chi_square(&[confirmed, replaced[0], replaced[2]]) < 30.0, "{confirmed} {replaced:?}");
}

async fn post(client: &mut Client, body: &str) -> Reply {
    let headers = [("content-type", "text/plain;charset=UTF-8"), ("sec-fetch-site", "same-origin"), BROWSER];
    client.send(request("POST", "/_rum", &headers, Bytes::from(body.to_owned()))).await
}

async fn summary(client: &mut Client) -> RumSummary {
    let reply = client.get("/_rum", &[BROWSER]).await;
    assert_eq!(reply.status, StatusCode::OK);
    reply.json()
}

/// A page view's first beacon: `converted` lists what it reached already.
fn view(variant: &str, converted: &[&str]) -> String {
    let events: Vec<String> = converted.iter().map(|m| format!(r#""{m}":1"#)).collect();
    let converted: Vec<String> = converted.iter().map(|m| format!(r#""{m}""#)).collect();
    format!(
        r#"{{"route":"/en/","view":true,"variant":"{variant}","events":{{"render_tier:B":1,{}}},"converted":[{}]}}"#,
        events.join(","),
        converted.join(",")
    )
    .replace(",}", "}")
}

#[tokio::test]
async fn beacons_count_per_variant_and_the_verdict_waits_for_the_plan() {
    let server = start("ab-count", 4).await;
    let mut client = Client::h1(server.addr()).await;

    let e = summary(&mut client).await.experiment.expect("a test runs");
    assert_eq!((e.id.as_str(), e.metric.as_str(), e.secondary.as_slice()), ("hero-headline", "cta_start_project", &["contact_submit".to_string()][..]));
    assert_eq!(e.plan.sample_size, 4);
    assert_eq!(e.variants.iter().map(|v| v.id.as_str()).collect::<Vec<_>>(), VARIANTS);
    assert!(e.variants.iter().all(|v| v.views == 0 && v.conversions == 0));
    assert_eq!(e.result, None);

    // The control: 4 page views, one followed a link to the form.
    for converted in [&["cta_start_project"][..], &[], &[], &[]] {
        assert_eq!(post(&mut client, &view(VARIANTS[0], converted)).await.status, StatusCode::NO_CONTENT);
    }
    // hero-title-hardware: 3 of 4, one of them reported by a later beacon of its page view, which
    // then also sent the form.
    let later = r#"{"route":"/en/","variant":"hero-title-hardware","events":{"cta_start_project":2,"contact_submit":1},"converted":["cta_start_project","contact_submit"]}"#;
    for body in [view(VARIANTS[1], &[]), later.to_owned(), view(VARIANTS[1], &["cta_start_project"]), view(VARIANTS[1], &["cta_start_project"]), view(VARIANTS[1], &[])] {
        assert_eq!(post(&mut client, &body).await.status, StatusCode::NO_CONTENT, "{body}");
    }
    // hero-title-cycle: 3 so far.
    for _ in 0..3 {
        assert_eq!(post(&mut client, &view(VARIANTS[2], &[])).await.status, StatusCode::NO_CONTENT);
    }
    let s = summary(&mut client).await;
    let e = s.experiment.unwrap();
    assert_eq!(e.variants.iter().map(|v| (v.views, v.conversions)).collect::<Vec<_>>(), [(4, 1), (4, 3), (3, 0)]);
    assert_eq!(e.variants[1].secondary["contact_submit"], 1);
    assert_eq!(e.result, None, "no verdict while a variant is short of its planned page views");
    assert_eq!(s.start.views, 11, "every page view is a page view in the totals too");
    assert_eq!(s.start.events["cta_start_project"], 5);

    // A variant that has its page views counts no more, for the test.
    assert_eq!(post(&mut client, &view(VARIANTS[1], &["cta_start_project"])).await.status, StatusCode::NO_CONTENT);
    assert_eq!(summary(&mut client).await.experiment.unwrap().variants[1].views, 4);

    // The last page view of the plan: the comparison, once and for good.
    assert_eq!(post(&mut client, &view(VARIANTS[2], &[])).await.status, StatusCode::NO_CONTENT);
    let e = summary(&mut client).await.experiment.unwrap();
    let result = e.result.expect("every variant has its 4 page views");
    assert_eq!(result.iter().map(|c| c.variant.as_str()).collect::<Vec<_>>(), &VARIANTS[1..]);
    // 1 in 4 against 3 in 4: z = √2, p = 0.157 — not below 0.025, too few page views to tell.
    assert!((result[0].z - std::f64::consts::SQRT_2).abs() < 1e-9 && (result[0].p - 0.1573).abs() < 1e-4, "{result:?}");
    assert!(!result[0].significant && !result[1].significant);
    assert!(result[1].z < 0.0, "fewer conversions than the control: a negative z");
    for _ in 0..3 {
        post(&mut client, &view(VARIANTS[0], &["cta_start_project"])).await;
    }
    assert_eq!(summary(&mut client).await.experiment.unwrap().result.unwrap(), result, "final");

    // Refused, with fixed text, counted as invalid.
    for (body, why) in [
        (r#"{"route":"/en/","view":true,"variant":"hero-title-bold"}"#, "unknown variant"),
        (r#"{"route":"/en/sent/","view":true,"variant":"hero-title"}"#, "a page outside the test"),
        (r#"{"route":"/en/","view":true,"variant":"hero-title","converted":["cta_start_project"]}"#, "a conversion without its event"),
        (r#"{"route":"/en/","view":true,"events":{"cta_start_project":1},"converted":["cta_start_project"]}"#, "a conversion without its variant"),
    ] {
        let reply = post(&mut client, body).await;
        assert_eq!(reply.status, StatusCode::BAD_REQUEST, "{why}");
        assert_eq!(reply.text(), "beacon refused: unknown experiment variant or conversion\n", "{why}");
    }
    assert_eq!(summary(&mut client).await.rejected.invalid, 4);
}

#[tokio::test]
async fn without_a_test_there_is_no_experiment_block() {
    let server = TestServer::start(config("ab-none")).await;
    let mut client = Client::h1(server.addr()).await;
    let reply = client.get("/_rum", &[BROWSER]).await;
    assert!(reply.text().contains(r#""experiment":null"#), "{}", reply.text());
    let refused = post(&mut client, r#"{"route":"/en/","view":true,"variant":"hero-title"}"#).await;
    assert_eq!(refused.status, StatusCode::BAD_REQUEST);
    assert_eq!(client.get("/en/", &[BROWSER]).await.header("cache-control"), "no-cache");
}

#[tokio::test]
async fn a_test_the_manifest_gets_wrong_fails_at_startup() {
    async fn error(name: &str, edit: impl FnOnce(&mut serde_json::Value)) -> String {
        let mut cfg = config(name);
        cfg.dist = dist(name, 10, edit);
        match server::Server::bind(cfg).await {
            Ok(_) => panic!("{name}: bind should fail"),
            Err(e) => format!("{e:#}"),
        }
    }
    let err = error("ab-no-test", |m| m["experiment"] = serde_json::Value::Null).await;
    assert!(err.contains("lists variants") && err.contains("no `experiment`"), "{err}");
    let err = error("ab-order", |m| m["experiment"]["variants"] = serde_json::json!(["hero-title", "hero-title-cycle", "hero-title-hardware"])).await;
    assert!(err.contains("variants must be the experiment's"), "{err}");
    let err = error("ab-missing", |m| m["routes"][0]["variants"][1]["file"] = "en/index.gone.html".into()).await;
    assert!(err.contains("en/index.gone.html") && err.contains("hero-title-cycle"), "{err}");
    let err = error("ab-metric", |m| m["experiment"]["metric"] = "page_view".into()).await;
    assert!(err.contains("page_view"), "{err}");
    let err = error("ab-nowhere", |m| m["routes"][0]["variants"] = serde_json::json!([])).await;
    assert!(err.contains("no route serves its variants"), "{err}");
}
