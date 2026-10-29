//! State shared by every connection and handler.

use std::time::Instant;

use bytes::Bytes;
use http::HeaderValue;
use proof::Location;
use tokio::sync::watch;

use crate::assets::Site;
use crate::contact::Contact;
use crate::ctf::Ctf;
use crate::headers::SecurityHeaders;
use crate::pow::Pow;
use crate::rum::Rum;
use crate::telemetry::{Metrics, StreamLimit};

/// `server.software` in `/_proof`: crate version plus the compiler and library versions recorded
/// by `build.rs` from the toolchain and `Cargo.lock` of this very build.
pub(crate) const SOFTWARE: &str = concat!(
    "mintaka-server ",
    env!("CARGO_PKG_VERSION"),
    " · rustc ",
    env!("MINTAKA_RUSTC_VERSION"),
    " · axum ",
    env!("MINTAKA_AXUM_VERSION"),
    " · rustls ",
    env!("MINTAKA_RUSTLS_VERSION"),
);

/// Facts about this process that `/_proof` and `/_telemetry` report.
pub(crate) struct ServerInfo {
    pub region: String,
    pub location: Option<Location>,
    pub started: Instant,
    pub cores: u32,
}

/// Process-wide state, behind one `Arc`.
pub(crate) struct AppState {
    pub site: Site,
    pub security: SecurityHeaders,
    pub info: ServerInfo,
    pub metrics: Metrics,
    pub streams: StreamLimit,
    pub pow: Pow,
    pub contact: Contact,
    pub ctf: Ctf,
    /// Real-user measurements (`/_rum`): in-memory aggregates and their rate limiter.
    pub rum: Rum,
    /// The `event: hello` frame of `/_telemetry`.
    pub hello: Bytes,
    /// `Alt-Svc` for TLS responses (`h3=":443"; ma=86400`) when HTTP/3 is on.
    pub alt_svc: Option<HeaderValue>,
    /// Turns `true` when the server starts shutting down (ends event streams).
    pub shutdown: watch::Receiver<bool>,
}
