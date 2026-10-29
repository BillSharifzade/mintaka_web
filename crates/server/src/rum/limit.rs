//! Per-network token buckets for `POST /_rum`: a burst of 60 beacons, refilled at 60 per hour,
//! per IPv4 /24 or IPv6 /48 ([`ClientNet`]). A page view sends one beacon (a second only if the
//! visitor comes back to the tab and does something), so this only bites a sender trying to skew
//! the aggregates. In memory only; the full address is never kept, logged or written anywhere.

use std::collections::HashMap;
use std::sync::{Mutex, PoisonError};
use std::time::Instant;

use crate::contact::ClientNet;

const CAPACITY: f64 = 60.0;
const PER_HOUR: f64 = 60.0;
/// Idle (full) buckets are dropped once the map holds this many networks.
const PRUNE_AT: usize = 10_000;
/// Beyond this many active networks, new ones are refused (fail closed under a flood).
const MAX_NETWORKS: usize = 100_000;

#[derive(Debug, Clone, Copy)]
struct Bucket {
    tokens: f64,
    at: Instant,
}

impl Bucket {
    fn refill(&mut self, now: Instant) {
        let hours = now.saturating_duration_since(self.at).as_secs_f64() / 3600.0;
        self.tokens = (self.tokens + hours * PER_HOUR).min(CAPACITY);
        self.at = now;
    }
}

#[derive(Default)]
pub(crate) struct Limiter {
    nets: Mutex<HashMap<ClientNet, Bucket>>,
}

impl Limiter {
    /// Takes one token for `net`; `false` means "rate limited".
    pub fn admit(&self, net: ClientNet, now: Instant) -> bool {
        let mut nets = self.nets.lock().unwrap_or_else(PoisonError::into_inner);
        if nets.len() >= PRUNE_AT && !nets.contains_key(&net) {
            nets.retain(|_, b| {
                b.refill(now);
                b.tokens < CAPACITY
            });
            if nets.len() >= MAX_NETWORKS {
                return false;
            }
        }
        let bucket = nets.entry(net).or_insert(Bucket { tokens: CAPACITY, at: now });
        bucket.refill(now);
        if bucket.tokens >= 1.0 {
            bucket.tokens -= 1.0;
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn sixty_then_one_a_minute() {
        let limiter = Limiter::default();
        let net = ClientNet::of("198.51.100.7".parse().unwrap());
        let t0 = Instant::now();
        assert!((0..60).all(|_| limiter.admit(net, t0)));
        assert!(!limiter.admit(ClientNet::of("198.51.100.200".parse().unwrap()), t0), "same /24");
        assert!(limiter.admit(ClientNet::of("198.51.101.7".parse().unwrap()), t0), "other networks unaffected");
        assert!(limiter.admit(net, t0 + Duration::from_secs(61)));
        assert!(!limiter.admit(net, t0 + Duration::from_secs(62)));
    }
}
