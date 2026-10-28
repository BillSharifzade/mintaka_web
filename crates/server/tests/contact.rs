//! The contact API: proof of work (solved here in Rust), validation, honeypot, rate limits and
//! encrypted storage.

mod common;

use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::str::FromStr;

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use bytes::Bytes;
use common::{BROWSER, Client, TestServer, config, request};
use http::StatusCode;
use proof::{ContactResponse, PowChallenge, StoredBrief};
use sha2::{Digest, Sha256};

/// The client side of §8.1: smallest counter with enough leading zero bits.
fn solve(challenge: &str, difficulty: u8) -> u64 {
    let bytes = URL_SAFE_NO_PAD.decode(challenge).unwrap();
    assert_eq!(bytes.len(), 41);
    assert_eq!(bytes[24], difficulty, "difficulty byte inside the challenge");
    (0u64..)
        .find(|counter| {
            let mut h = Sha256::new();
            h.update(&bytes);
            h.update(counter.to_le_bytes());
            let digest = h.finalize();
            let mut zeros = 0;
            for byte in digest {
                if byte == 0 {
                    zeros += 8;
                } else {
                    zeros += byte.leading_zeros();
                    break;
                }
            }
            zeros >= u32::from(difficulty)
        })
        .unwrap()
}

async fn challenge(client: &mut Client) -> PowChallenge {
    let reply = client.get("/api/pow", &[BROWSER]).await;
    assert_eq!(reply.status, StatusCode::OK);
    assert_eq!(reply.header("cache-control"), "no-store");
    reply.json()
}

async fn post_json(client: &mut Client, body: serde_json::Value) -> common::Reply {
    let body = Bytes::from(body.to_string());
    client.send(request("POST", "/api/contact", &[("content-type", "application/json"), BROWSER], body)).await
}

async fn post_form(client: &mut Client, body: &str) -> common::Reply {
    let headers = [("content-type", "application/x-www-form-urlencoded"), BROWSER];
    client.send(request("POST", "/api/contact", &headers, Bytes::from(body.to_owned()))).await
}

fn stored(data_dir: &Path) -> Vec<std::path::PathBuf> {
    let mut files: Vec<_> = std::fs::read_dir(data_dir.join("contact")).unwrap().map(|e| e.unwrap().path()).collect();
    files.sort();
    files
}

fn decrypt(data_dir: &Path, file: &Path) -> StoredBrief {
    let text = std::fs::read_to_string(data_dir.join("dev-identity.txt")).unwrap();
    let key = text.lines().find(|l| l.starts_with("AGE-SECRET-KEY-")).unwrap();
    let identity = age::x25519::Identity::from_str(key).unwrap();
    serde_json::from_slice(&age::decrypt(&identity, &std::fs::read(file).unwrap()).unwrap()).unwrap()
}

#[tokio::test]
async fn pow_solved_brief_is_stored_encrypted_and_single_use() {
    let mut cfg = config("contact-pow");
    cfg.pow_difficulty = 16;
    let server = TestServer::start(cfg).await;
    let mut client = Client::h1(server.addr()).await;

    let c = challenge(&mut client).await;
    assert_eq!(c.difficulty, 16);
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
    assert!(c.expires_at >= now + 590 && c.expires_at <= now + 601, "valid for 10 minutes");
    let counter = solve(&c.challenge, c.difficulty);
    let brief = serde_json::json!({
        "kind": ["performance", "security-audit"],
        "message": "Our API is slow.\r\nHelp?",
        "timeline": "asap",
        "budget": "10-50k",
        "name": "  Bill  ",
        "email": "bill@mintaka.tj",
        "telegram": "@mintaka_tj",
        "company": "Mintaka",
        "lang": "en",
        "website": "",
        "pow_challenge": c.challenge,
        "pow_counter": counter,
    });
    let reply = post_json(&mut client, brief.clone()).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
    assert_eq!(reply.text(), r#"{"ok":true}"#);
    assert_eq!(reply.header("cache-control"), "no-store");

    let files = stored(&server.data_dir);
    assert_eq!(files.len(), 1);
    let file = &files[0];
    let name = file.file_name().unwrap().to_str().unwrap();
    let (ms, rest) = name.split_once('-').unwrap();
    assert!(ms.parse::<u64>().is_ok() && rest.ends_with(".json.age") && rest.len() == 16 + 9, "{name}");
    assert_eq!(std::fs::metadata(file).unwrap().permissions().mode() & 0o777, 0o600);
    let raw = std::fs::read(file).unwrap();
    assert!(raw.starts_with(b"age-encryption.org/v1\n"), "age format");
    assert!(!raw.windows(4).any(|w| w == b"Bill"), "not plaintext");

    let brief_stored = decrypt(&server.data_dir, file);
    assert_eq!(brief_stored.name, "Bill");
    assert_eq!(brief_stored.message.as_deref(), Some("Our API is slow.\nHelp?"));
    assert_eq!(brief_stored.telegram.as_deref(), Some("mintaka_tj"), "stored without @");
    assert_eq!(brief_stored.kinds, [proof::Kind::Performance, proof::Kind::SecurityAudit]);
    assert_eq!(brief_stored.budget, Some(proof::Budget::From10To50k));
    assert!(brief_stored.proof_of_work);
    assert_eq!(format!("{}.json.age", brief_stored.id), name);

    // Single use.
    let replay = post_json(&mut client, brief).await;
    assert_eq!(replay.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(replay.text(), r#"{"ok":false,"errors":[{"field":"pow","code":"pow-invalid"}]}"#);
    assert_eq!(stored(&server.data_dir).len(), 1);
}

#[tokio::test]
async fn forged_or_unsolved_pow_is_rejected() {
    let server = TestServer::start(config("contact-forged")).await;
    let mut client = Client::h1(server.addr()).await;
    let c = challenge(&mut client).await;
    let counter = solve(&c.challenge, c.difficulty);
    let mut bytes = URL_SAFE_NO_PAD.decode(&c.challenge).unwrap();
    bytes[24] = 1; // claim an easier difficulty
    let base = serde_json::json!({ "name": "Bill", "email": "bill@mintaka.tj" });
    let with = |challenge: serde_json::Value, counter: serde_json::Value| {
        let mut b = base.clone();
        b["pow_challenge"] = challenge;
        b["pow_counter"] = counter;
        b
    };
    let cases = [
        with(URL_SAFE_NO_PAD.encode(&bytes).into(), counter.into()),
        with(c.challenge.clone().into(), serde_json::Value::Null),
        with("bm9wZQ".into(), 1.into()),
    ];
    for body in cases {
        let reply = post_json(&mut client, body.clone()).await;
        assert_eq!(reply.status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");
        assert_eq!(reply.json::<ContactResponse>().errors[0].code, proof::ErrorCode::PowInvalid, "{body}");
    }
    // Insufficient work: find a counter that does not solve it.
    let bytes = URL_SAFE_NO_PAD.decode(&c.challenge).unwrap();
    let weak = (0u64..)
        .find(|n| {
            let mut h = Sha256::new();
            h.update(&bytes);
            h.update(n.to_le_bytes());
            h.finalize()[0] != 0
        })
        .unwrap();
    let reply = post_json(&mut client, with(c.challenge.clone().into(), weak.into())).await;
    assert_eq!(reply.json::<ContactResponse>().errors[0].code, proof::ErrorCode::PowInvalid);
    assert!(stored(&server.data_dir).is_empty());
}

#[tokio::test]
async fn validation_errors_are_typed_codes() {
    let server = TestServer::start(config("contact-validation")).await;
    let mut client = Client::h1(server.addr()).await;
    let reply = post_json(
        &mut client,
        serde_json::json!({
            "kind": ["new-product", "rockets"],
            "message": "x".repeat(5001),
            "budget": "a lot",
            "name": "",
            "email": "bill@",
            "telegram": "@abc",
            "company": "c".repeat(121),
        }),
    )
    .await;
    assert_eq!(reply.status, StatusCode::UNPROCESSABLE_ENTITY);
    let errors: Vec<(String, String)> = reply
        .json::<ContactResponse>()
        .errors
        .iter()
        .map(|e| (e.field.as_str().to_owned(), e.code.as_str().to_owned()))
        .collect();
    let expected = [
        ("kind", "choice-invalid"),
        ("message", "message-too-long"),
        ("budget", "choice-invalid"),
        ("name", "name-required"),
        ("email", "email-invalid"),
        ("telegram", "telegram-invalid"),
        ("company", "company-too-long"),
    ];
    assert_eq!(errors, expected.map(|(f, c)| (f.to_owned(), c.to_owned())));

    let reply = post_json(&mut client, serde_json::json!({ "name": "Bill" })).await;
    assert_eq!(reply.text(), r#"{"ok":false,"errors":[{"field":"contact","code":"contact-required"}]}"#);
    let reply = post_json(&mut client, serde_json::json!({ "name": "x".repeat(121), "telegram": "mintaka_tj" })).await;
    assert_eq!(reply.json::<ContactResponse>().errors[0].code, proof::ErrorCode::NameTooLong);

    // Malformed JSON and unsupported types.
    let bad = client.send(request("POST", "/api/contact", &[("content-type", "application/json")], Bytes::from_static(b"{nope"))).await;
    assert_eq!(bad.status, StatusCode::BAD_REQUEST);
    let xml = client.send(request("POST", "/api/contact", &[("content-type", "text/xml")], Bytes::from_static(b"<a/>"))).await;
    assert_eq!(xml.status, StatusCode::UNSUPPORTED_MEDIA_TYPE);
    assert!(stored(&server.data_dir).is_empty());
}

#[tokio::test]
async fn no_js_form_redirects_with_error_fragments() {
    let server = TestServer::start(config("contact-form")).await;
    let mut client = Client::h1(server.addr()).await;
    let reply = post_form(&mut client, "lang=en&kind=ai&name=&email=bill%40mintaka.tj").await;
    assert_eq!(reply.status, StatusCode::SEE_OTHER);
    assert_eq!(reply.header("location"), "/en/sent/#e-name-required");
    assert_eq!(reply.header("cache-control"), "no-store");

    let reply = post_form(&mut client, "lang=ru&name=Bill").await;
    assert_eq!(reply.header("location"), "/ru/sent/#e-contact-required");

    let reply = post_form(&mut client, "lang=en&kind=ai&kind=design&name=Bill+K&telegram=mintaka_tj&message=Hi%0D%0Athere").await;
    assert_eq!(reply.status, StatusCode::SEE_OTHER);
    assert_eq!(reply.header("location"), "/en/sent/");
    let files = stored(&server.data_dir);
    assert_eq!(files.len(), 1);
    let brief = decrypt(&server.data_dir, &files[0]);
    assert_eq!(brief.name, "Bill K");
    assert_eq!(brief.kinds, [proof::Kind::Ai, proof::Kind::Design]);
    assert_eq!(brief.message.as_deref(), Some("Hi\nthere"));
    assert!(!brief.proof_of_work);
    // An unknown language falls back to the default for the redirect.
    let reply = post_form(&mut client, "lang=fr&name=").await;
    assert_eq!(reply.header("location"), "/en/sent/#e-name-required");
}

#[tokio::test]
async fn honeypot_answers_like_success_and_stores_nothing() {
    let server = TestServer::start(config("contact-honeypot")).await;
    let mut client = Client::h1(server.addr()).await;
    let json = post_json(&mut client, serde_json::json!({ "name": "Bot", "email": "bot@spam.example", "website": "http://spam.example" })).await;
    let real = post_json(&mut client, serde_json::json!({ "name": "Bill", "email": "bill@mintaka.tj" })).await;
    assert_eq!((json.status, json.text()), (real.status, real.text()));
    let form = post_form(&mut client, "lang=en&name=Bot&email=bot%40spam.example&website=x").await;
    assert_eq!(form.status, StatusCode::SEE_OTHER);
    assert_eq!(form.header("location"), "/en/sent/");
    // Even an otherwise invalid submission "succeeds" when the trap is filled.
    let invalid = post_json(&mut client, serde_json::json!({ "website": "x" })).await;
    assert_eq!(invalid.text(), r#"{"ok":true}"#);
    assert_eq!(stored(&server.data_dir).len(), 1, "only the real brief");
}

#[tokio::test]
async fn rate_limits_three_without_pow_ten_with() {
    let server = TestServer::start(config("contact-limits")).await;
    let mut client = Client::h1(server.addr()).await;
    for i in 0..3 {
        let reply = post_form(&mut client, &format!("lang=en&name=Bill{i}&email=bill%40mintaka.tj")).await;
        assert_eq!(reply.header("location"), "/en/sent/", "{i}");
    }
    let limited = post_form(&mut client, "lang=en&name=Bill&email=bill%40mintaka.tj").await;
    assert_eq!(limited.header("location"), "/en/sent/#e-rate-limited");
    let limited = post_json(&mut client, serde_json::json!({ "name": "Bill", "email": "bill@mintaka.tj" })).await;
    assert_eq!(limited.status, StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(limited.text(), r#"{"ok":false,"errors":[{"field":"form","code":"rate-limited"}]}"#);

    // With proof of work: a separate, larger budget.
    for i in 0..10 {
        let c = challenge(&mut client).await;
        let counter = solve(&c.challenge, c.difficulty);
        let body = serde_json::json!({ "name": format!("P{i}"), "email": "p@mintaka.tj", "pow_challenge": c.challenge, "pow_counter": counter });
        assert_eq!(post_json(&mut client, body).await.status, StatusCode::OK, "{i}");
    }
    let c = challenge(&mut client).await;
    let counter = solve(&c.challenge, c.difficulty);
    let body = serde_json::json!({ "name": "P", "email": "p@mintaka.tj", "pow_challenge": c.challenge, "pow_counter": counter });
    assert_eq!(post_json(&mut client, body).await.status, StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(stored(&server.data_dir).len(), 13);
}

#[tokio::test]
async fn bodies_over_32_kib_are_refused() {
    let server = TestServer::start(config("contact-large")).await;
    let mut client = Client::h1(server.addr()).await;
    let big = serde_json::json!({ "name": "Bill", "email": "bill@mintaka.tj", "message": "x".repeat(40 * 1024) });
    let reply = post_json(&mut client, big).await;
    assert_eq!(reply.status, StatusCode::PAYLOAD_TOO_LARGE);
    assert_eq!(reply.text(), r#"{"ok":false,"errors":[{"field":"form","code":"too-large"}]}"#);
    let mut client = Client::h1(server.addr()).await;
    let form = post_form(&mut client, &format!("name=Bill&message={}", "x".repeat(40 * 1024))).await;
    assert_eq!(form.header("location"), "/en/sent/#e-too-large");
    // Chunked bodies without Content-Length are limited while reading (HTTP/2 here).
    let mut h2 = Client::h2c(server.addr()).await;
    let mut req = request("POST", "/api/contact", &[("content-type", "application/json")], Bytes::from(vec![b' '; 33 * 1024]));
    req.headers_mut().remove("content-length");
    assert_eq!(h2.send(req).await.status, StatusCode::PAYLOAD_TOO_LARGE);
    assert!(stored(&server.data_dir).is_empty());
}
