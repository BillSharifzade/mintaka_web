//! Startup checks (fail fast with a clear message) and the real binary: CLI, self-signed TLS
//! fingerprint, graceful shutdown on SIGTERM.

mod common;

use std::path::Path;
use std::process::Stdio;
use std::time::Duration;

use common::{BROWSER, config, fixture_dist, temp_dir};
use tokio::io::{AsyncBufReadExt, BufReader};

/// A copy of the fixture dist with a rewritten manifest.
fn dist_with(name: &str, edit: impl FnOnce(&mut serde_json::Value)) -> std::path::PathBuf {
    let dir = temp_dir(name);
    copy_dir(&fixture_dist(), &dir);
    let path = dir.join("manifest.json");
    let mut manifest: serde_json::Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    edit(&mut manifest);
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

async fn bind_error(dist: std::path::PathBuf) -> String {
    let mut cfg = config("startup-error");
    cfg.dist = dist;
    match server::Server::bind(cfg).await {
        Ok(_) => panic!("bind should fail"),
        Err(e) => format!("{e:#}"),
    }
}

#[tokio::test]
async fn missing_files_and_wrong_versions_fail_fast() {
    let missing = dist_with("missing-file", |m| m["routes"][0]["file"] = "en/gone.html".into());
    let err = bind_error(missing).await;
    assert!(err.contains("en/gone.html") && err.contains("/en/"), "{err}");

    let missing_sibling = dist_with("missing-br", |m| m["routes"][4]["encodings"] = serde_json::json!(["br"]));
    let err = bind_error(missing_sibling).await;
    assert!(err.contains("favicon.svg.br"), "{err}");

    let version = dist_with("version", |m| m["version"] = 2.into());
    let err = bind_error(version).await;
    assert!(err.contains("manifest version 2") && err.contains("reads version 1"), "{err}");

    let traversal = dist_with("traversal", |m| m["routes"][0]["file"] = "../../etc/passwd".into());
    let err = bind_error(traversal).await;
    assert!(err.contains("plain path relative to dist/"), "{err}");

    let csp = dist_with("csp", |m| m["csp"]["style_src"] = serde_json::json!(["sha256-x'; script-src *"]));
    let err = bind_error(csp).await;
    assert!(err.contains("hash source"), "{err}");

    let no_root = dist_with("no-root", |m| m["langs"] = serde_json::json!(["en", "ru", "tg"]));
    let err = bind_error(no_root).await;
    assert!(err.contains("\"tg\"") && err.contains("/tg/"), "{err}");

    let err = bind_error(temp_dir("empty")).await;
    assert!(err.contains("manifest.json") && err.contains("--dist"), "{err}");
}

#[tokio::test]
async fn binary_serves_tls_and_shuts_down_gracefully_on_sigterm() {
    let data = temp_dir("binary");
    let mut child = tokio::process::Command::new(env!("CARGO_BIN_EXE_mintaka-server"))
        .args(["--listen", "127.0.0.1:0", "--tls-self-signed", "--tls-listen", "127.0.0.1:0"])
        .arg("--dist")
        .arg(fixture_dist())
        .arg("--data-dir")
        .arg(&data)
        .args(["--region", "ci", "--location", "Dushanbe,TJ,38.5598,68.7870"])
        .env("RUST_LOG", "info")
        .env_remove("MINTAKA_CONTACT_RECIPIENT")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .unwrap();
    let mut lines = BufReader::new(child.stdout.take().unwrap()).lines();
    let (mut http, mut fingerprint) = (None, None);
    let mut log = Vec::new();
    while http.is_none() || fingerprint.is_none() {
        let line = tokio::time::timeout(Duration::from_secs(20), lines.next_line()).await.unwrap().unwrap().unwrap();
        if let Some(rest) = line.split("listening on http://").nth(1) {
            http = Some(rest.split_whitespace().next().unwrap().parse::<std::net::SocketAddr>().unwrap());
        }
        if let Some(rest) = line.split("SHA-256 fingerprint ").nth(1) {
            fingerprint = Some(rest.trim().to_owned());
        }
        log.push(line);
    }
    let fingerprint = fingerprint.unwrap();
    assert_eq!(fingerprint.len(), 95, "{fingerprint}");
    assert!(log.iter().any(|l| l.contains("DEVELOPMENT identity")), "loud dev-identity warning");

    let reply = common::get(http.unwrap(), "/_proof", &[BROWSER]).await;
    let p: proof::Proof = reply.json();
    assert_eq!(p.server.region, "ci");
    assert_eq!(p.server.location.unwrap().lon, 68.787);

    let status = std::process::Command::new("kill").args(["-TERM", &child.id().unwrap().to_string()]).status().unwrap();
    assert!(status.success());
    let exit = tokio::time::timeout(Duration::from_secs(12), child.wait()).await.expect("exits within the grace period").unwrap();
    assert!(exit.success(), "{exit:?}");
    while let Ok(Some(line)) = lines.next_line().await {
        log.push(line);
    }
    let log = log.join("\n");
    assert!(log.contains("SIGTERM received"), "{log}");
    assert!(log.contains("all connections closed"), "{log}");
    // Access lines carry method, route, status and duration only.
    let access: Vec<&str> = log.lines().filter(|l| l.contains(" access: ")).collect();
    assert_eq!(access.len(), 1, "{log}");
    assert!(access[0].contains("access: method=GET route=/_proof status=200 us="), "{}", access[0]);
    assert!(!access[0].contains("127.0.0.1") && !access[0].contains("Mozilla"), "no address or user agent: {}", access[0]);
}

#[tokio::test]
async fn binary_rejects_bad_flags_with_a_message() {
    let out = tokio::process::Command::new(env!("CARGO_BIN_EXE_mintaka-server"))
        .args(["--redirect-http", "127.0.0.1:0"])
        .output()
        .await
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&out.stderr).contains("--redirect-http needs a TLS listener"));

    let out = tokio::process::Command::new(env!("CARGO_BIN_EXE_mintaka-server"))
        .args(["--listen", "127.0.0.1:0", "--dist"])
        .arg(temp_dir("no-dist"))
        .arg("--data-dir")
        .arg(temp_dir("no-dist-data"))
        .output()
        .await
        .unwrap();
    assert_eq!(out.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&out.stderr).contains("manifest.json"));
}
