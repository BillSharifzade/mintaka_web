//! Records the toolchain and library versions that `/_proof` reports as `server.software`, so the
//! string describes the binary that was actually built rather than a hand-maintained guess.

use std::path::Path;
use std::process::Command;

fn main() {
    let rustc = std::env::var("RUSTC").unwrap_or_else(|_| "rustc".to_owned());
    let rustc_version = Command::new(rustc)
        .arg("--version")
        .output()
        .ok()
        .and_then(|out| String::from_utf8(out.stdout).ok())
        // "rustc 1.99.1 (58cf8f9c3 2026-10-13) (Arch Linux rust 1:1.99.1-1)" -> "1.99.1"
        .and_then(|s| s.split_whitespace().nth(1).map(str::to_owned))
        .unwrap_or_else(|| "unknown".to_owned());
    println!("cargo:rustc-env=MINTAKA_RUSTC_VERSION={rustc_version}");

    // The workspace lock file pins the exact axum and rustls the binary links.
    let lock_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../Cargo.lock");
    println!("cargo:rerun-if-changed={}", lock_path.display());
    let lock = std::fs::read_to_string(&lock_path).unwrap_or_default();
    for (krate, var) in [("axum", "MINTAKA_AXUM_VERSION"), ("rustls", "MINTAKA_RUSTLS_VERSION")] {
        let version = newest_locked(&lock, krate).unwrap_or_else(|| "unknown".to_owned());
        println!("cargo:rustc-env={var}={version}");
    }
    println!("cargo:rerun-if-changed=build.rs");
}

/// `major.minor` of the newest version of `krate` in a Cargo.lock (the one this crate depends on:
/// the server always tracks the newest axum/rustls in the workspace).
fn newest_locked(lock: &str, krate: &str) -> Option<String> {
    let needle = format!("name = \"{krate}\"\nversion = \"");
    lock.match_indices(&needle)
        .filter_map(|(at, _)| {
            let rest = &lock[at + needle.len()..];
            let version = &rest[..rest.find('"')?];
            let mut parts = version.split(['.', '-']).map(|p| p.parse::<u64>().ok());
            let (major, minor) = (parts.next()??, parts.next()??);
            Some((major, minor))
        })
        .max()
        .map(|(major, minor)| format!("{major}.{minor}"))
}
