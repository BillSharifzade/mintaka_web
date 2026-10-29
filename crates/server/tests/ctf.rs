//! The console challenge over HTTP, played the way a visitor would: each stage is solved here
//! from the puzzle data alone (no access to the server's answers), then the pass goes with an
//! application through the contact API.

mod common;

use std::cmp::Reverse;
use std::collections::BinaryHeap;
use std::str::FromStr;

use bytes::Bytes;
use common::{BROWSER, Client, TestServer, config, request};
use http::StatusCode;
use proof::{CTF_CRIB, CtfPuzzle, CtfState, StoredBrief};

async fn post(client: &mut Client, path: &str, body: serde_json::Value) -> common::Reply {
    client.send(request("POST", path, &[("content-type", "application/json"), BROWSER], Bytes::from(body.to_string()))).await
}

fn unhex(s: &str) -> Vec<u8> {
    (0..s.len()).step_by(2).map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap()).collect()
}

/// What a player writes for each stage.
fn solve(puzzle: &CtfPuzzle) -> String {
    match puzzle {
        CtfPuzzle::Rigel { grid } => {
            let rows: Vec<&[u8]> = grid.lines().map(str::as_bytes).collect();
            let n = rows.len();
            let cost = |at: usize| u32::from(rows[at / n][at % n] - b'0');
            let mut dist = vec![u32::MAX; n * n];
            let mut queue = BinaryHeap::from([Reverse((0, 0))]);
            dist[0] = 0;
            while let Some(Reverse((d, at))) = queue.pop() {
                if d > dist[at] {
                    continue;
                }
                let (r, c) = (at / n, at % n);
                for (dr, dc) in [(-1i64, 0i64), (1, 0), (0, -1), (0, 1)] {
                    let (nr, nc) = (r as i64 + dr, c as i64 + dc);
                    if (0..n as i64).contains(&nr) && (0..n as i64).contains(&nc) {
                        let to = nr as usize * n + nc as usize;
                        if d + cost(to) < dist[to] {
                            dist[to] = d + cost(to);
                            queue.push(Reverse((dist[to], to)));
                        }
                    }
                }
            }
            dist[n * n - 1].to_string()
        }
        CtfPuzzle::Saiph { a, b } => {
            let plain: Vec<u8> = unhex(a).iter().zip(unhex(b)).zip(CTF_CRIB.bytes()).map(|((x, y), m)| x ^ y ^ m).collect();
            String::from_utf8(plain).unwrap().split_whitespace().last().unwrap().to_owned()
        }
        CtfPuzzle::Betelgeuse { tcp_info } => {
            let b = unhex(tcp_info);
            let field = |at: usize| u64::from(u32::from_le_bytes(b[at..at + 4].try_into().unwrap()));
            let (mss, rtt, cwnd) = (field(16), field(68), field(80));
            (cwnd * mss * 1_000_000 / rtt).to_string()
        }
        CtfPuzzle::Meissa { trials } => {
            let xs: Vec<f64> = trials.iter().map(|t| (f64::from(t[0]) / f64::from(t[1]) + 1.0).log2()).collect();
            let ys: Vec<f64> = trials.iter().map(|t| f64::from(t[2])).collect();
            let n = xs.len() as f64;
            let (mx, my) = (xs.iter().sum::<f64>() / n, ys.iter().sum::<f64>() / n);
            let sxy: f64 = xs.iter().zip(&ys).map(|(x, y)| (x - mx) * (y - my)).sum();
            let sxx: f64 = xs.iter().map(|x| (x - mx).powi(2)).sum();
            format!("{:.1}", 1000.0 / (sxy / sxx))
        }
    }
}

#[tokio::test]
async fn four_stages_then_an_application() {
    let server = TestServer::start(config("ctf-play")).await;
    let mut client = Client::h1(server.addr()).await;

    let reply = post(&mut client, "/api/ctf/start", serde_json::json!({})).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
    assert_eq!(reply.header("cache-control"), "no-store");
    common::assert_security_headers(&reply, false, "/api/ctf/start");
    let mut state: CtfState = reply.json();
    assert_eq!(state.stage, 1);

    for (stage, star) in [(1, "rigel"), (2, "saiph"), (3, "betelgeuse"), (4, "meissa")] {
        let puzzle = state.puzzle.clone().expect("a puzzle");
        assert!(serde_json::to_string(&puzzle).unwrap().contains(&format!(r#""star":"{star}""#)));

        // Reading it again gives the same puzzle and the same token.
        let again: CtfState = post(&mut client, "/api/ctf/puzzle", serde_json::json!({ "run": state.run })).await.json();
        assert_eq!((again.correct, &again.run, again.stage, &again.puzzle), (None, &state.run, state.stage, &state.puzzle));

        let wrong: CtfState = post(&mut client, "/api/ctf/flag", serde_json::json!({ "run": state.run, "flag": "0" })).await.json();
        assert_eq!((wrong.correct, wrong.stage, &wrong.run), (Some(false), stage, &state.run));

        let answer = solve(&puzzle);
        let right: CtfState =
            post(&mut client, "/api/ctf/flag", serde_json::json!({ "run": state.run, "flag": answer })).await.json();
        assert_eq!((right.correct, right.stage), (Some(true), stage + 1), "stage {stage} refused {answer:?}");
        assert_ne!(right.run, state.run);
        state = right;
    }
    assert_eq!(state.puzzle, None);

    let application = serde_json::json!({
        "name": "Ada", "email": "ada@example.com", "message": "I'd build the fifth stage.",
        "lang": "en", "website": "", "ctf_pass": state.run,
    });
    let reply = post(&mut client, "/api/contact", application).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
    let mut files: Vec<_> = std::fs::read_dir(server.data_dir.join("contact")).unwrap().map(|e| e.unwrap().path()).collect();
    assert_eq!(files.len(), 1);
    let text = std::fs::read_to_string(server.data_dir.join("dev-identity.txt")).unwrap();
    let key = text.lines().find(|l| l.starts_with("AGE-SECRET-KEY-")).unwrap();
    let identity = age::x25519::Identity::from_str(key).unwrap();
    let brief: StoredBrief =
        serde_json::from_slice(&age::decrypt(&identity, &std::fs::read(files.pop().unwrap()).unwrap()).unwrap()).unwrap();
    assert!(brief.ctf_solved);
    assert!(brief.kinds.is_empty());
}

#[tokio::test]
async fn bad_requests_and_passes_are_refused() {
    let server = TestServer::start(config("ctf-bad")).await;
    let mut client = Client::h1(server.addr()).await;

    let reply = client.get("/api/ctf/start", &[BROWSER]).await;
    assert_eq!(reply.status, StatusCode::METHOD_NOT_ALLOWED);
    assert_eq!(reply.header("allow"), "POST");

    let reply = post(&mut client, "/api/ctf/puzzle", serde_json::json!({ "run": "r1.AAAA" })).await;
    assert_eq!((reply.status, reply.text()), (StatusCode::BAD_REQUEST, r#"{"error":"bad-run"}"#.to_owned()));
    let reply = post(&mut client, "/api/ctf/flag", serde_json::json!(["not", "an", "object"])).await;
    assert_eq!((reply.status, reply.text()), (StatusCode::BAD_REQUEST, r#"{"error":"bad-request"}"#.to_owned()));
    let reply = post(&mut client, "/api/ctf/flag", serde_json::json!({ "run": "x".repeat(2000) })).await;
    assert_eq!((reply.status, reply.text()), (StatusCode::PAYLOAD_TOO_LARGE, r#"{"error":"too-large"}"#.to_owned()));

    // A stage-one token is not a pass: the brief is stored, unmarked.
    let state: CtfState = post(&mut client, "/api/ctf/start", serde_json::json!({})).await.json();
    let application = serde_json::json!({ "name": "Eve", "email": "eve@example.com", "ctf_pass": state.run });
    assert_eq!(post(&mut client, "/api/contact", application).await.status, StatusCode::OK);
}

#[tokio::test]
async fn starts_are_rate_limited_per_network() {
    let server = TestServer::start(config("ctf-limit")).await;
    let mut client = Client::h1(server.addr()).await;
    for _ in 0..20 {
        assert_eq!(post(&mut client, "/api/ctf/start", serde_json::json!({})).await.status, StatusCode::OK);
    }
    let reply = post(&mut client, "/api/ctf/start", serde_json::json!({})).await;
    assert_eq!((reply.status, reply.text()), (StatusCode::TOO_MANY_REQUESTS, r#"{"error":"rate-limited"}"#.to_owned()));
}
