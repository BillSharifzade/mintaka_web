//! Types shared across the Mintaka web stack.
//!
//! * [`manifest`] — what the site generator writes to `dist/manifest.json` and the server loads.
//! * [`proof`] — the `GET /_proof` payload.
//! * [`telemetry`] — the `GET /_telemetry` Server-Sent Events payloads.
//! * [`contact`] — `GET /api/pow` and `POST /api/contact`, plus the stored (encrypted) brief.
//!
//! The JSON shapes are mirrored by hand in `web/shared/proof.ts`; `docs/CONTRACTS.md` is the
//! source of truth for both sides.

pub mod contact;
pub mod manifest;
pub mod proof;
pub mod telemetry;

pub use contact::{
    Budget, CONTACT_BODY_LIMIT, ContactRequest, ContactResponse, ErrorCode, Field, FieldError, Kind,
    POW_CHALLENGE_BYTES, POW_TTL_SECS, PowChallenge, StoredBrief, Timeline, UnknownChoice,
};
pub use manifest::{CachePolicy, Csp, Encoding, MANIFEST_VERSION, Manifest, Route, RouteKind};
pub use proof::{ConnectionFacts, HttpFacts, Location, PrivacyFacts, Proof, ServerFacts, TlsFacts};
pub use telemetry::{TelemetryHello, TelemetryTick};
