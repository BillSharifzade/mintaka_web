//! The mintaka.tj HTTP server.
//!
//! The page claims that every number on it is measured live, so this server is part of the proof:
//! its security headers, TLS facts and timings are shown to the visitor. Shape of a request:
//!
//! ```text
//! TcpListener ─ accept ─▶ [rustls, TLS 1.3, X25519MLKEM768] ─▶ hyper-util auto (h1 + h2)
//!   ─▶ conn::ConnService (connection facts, t0, TCP_INFO) ─▶ axum Router ─▶ handler
//!   ◀─ routes::finish (security headers, Server-Timing, metrics, access log)
//! ```
//!
//! * Static files come from `dist/manifest.json` and are loaded into memory at startup
//!   (`assets`); the per-request path only clones pre-built header maps and `Bytes`.
//! * `/_proof`, `/_telemetry`, `/_ping`, curl mode and the contact API are in `routes`.
//!
//! The contract with the rest of the stack is `docs/CONTRACTS.md` §3–§8.

// `deny` rather than `forbid` so that `sys::tcp_rtt` — the single `getsockopt(TCP_INFO)` call —
// can opt out with a commented `#[allow(unsafe_code)]`. Nothing else may.
#![deny(unsafe_code)]
#![warn(missing_docs)]

mod app;
mod assets;
mod config;
mod conn;
mod contact;
mod headers;
mod negotiate;
mod pow;
mod routes;
mod server;
mod sys;
mod telemetry;
mod tls;

pub use config::{CertSource, Cli, Config, TelegramSettings, TlsSettings};
pub use server::Server;
pub use tls::SelfSigned;
