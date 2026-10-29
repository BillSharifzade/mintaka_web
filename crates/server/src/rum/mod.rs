//! Self-hosted real-user measurements (docs/CONTRACTS.md §14): beacons in, aggregates out. The
//! HTTP side is `routes::rum`.
//!
//! What a beacon becomes: one increment per counter and one sample per histogram, in memory.
//! No address, user agent, cookie or timestamp of a single beacon is kept anywhere, and nothing is
//! written to disk; the only per-client state is the rate limiter's bucket per /24 or /48.

mod beacon;
mod limit;
mod stats;

use std::sync::{Mutex, PoisonError};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use bytes::Bytes;

pub(crate) use beacon::Refusal;

use crate::contact::ClientNet;

/// Route names a beacon may carry besides the manifest's pages: `/` (served by language
/// negotiation, not a manifest route) and the not-found page, which has no path of its own.
const EXTRA_ROUTES: [&str; 2] = ["/", "404"];

fn unix_now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs())
}

/// The `/_rum` state of this process.
pub(crate) struct Rum {
    limiter: limit::Limiter,
    stats: Mutex<stats::Stats>,
}

impl Rum {
    /// Counts for `/`, `404` and every page route of the manifest.
    pub fn new<'a>(pages: impl Iterator<Item = &'a str>) -> anyhow::Result<Rum> {
        let mut routes: Vec<String> = EXTRA_ROUTES.map(str::to_owned).to_vec();
        for page in pages {
            if !routes.iter().any(|r| r == page) {
                routes.push(page.to_owned());
            }
        }
        Ok(Rum { limiter: limit::Limiter::default(), stats: Mutex::new(stats::Stats::new(routes, unix_now())?) })
    }

    fn stats(&self) -> std::sync::MutexGuard<'_, stats::Stats> {
        self.stats.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Takes a token from `net`'s bucket; counts a rate-limited beacon when there is none.
    pub fn admit(&self, net: ClientNet) -> bool {
        let admitted = self.limiter.admit(net, Instant::now());
        if !admitted {
            self.stats().reject(true);
        }
        admitted
    }

    /// Parses and counts one beacon.
    pub fn receive(&self, body: &[u8]) -> Result<(), Refusal> {
        let mut stats = self.stats();
        let parsed = beacon::parse(body, stats.routes()).and_then(|b| {
            stats.record(&b, unix_now()).map_err(|e| {
                tracing::error!(target: "rum", "recording a beacon failed: {e}");
                Refusal::Shape
            })
        });
        if parsed.is_err() {
            stats.reject(false);
        }
        parsed
    }

    /// Counts a beacon refused before parsing (too large, wrong type, cross-site).
    pub fn refuse(&self) {
        self.stats().reject(false);
    }

    /// The `GET /_rum` body.
    pub fn summary(&self) -> Bytes {
        self.stats().summary(unix_now())
    }
}
