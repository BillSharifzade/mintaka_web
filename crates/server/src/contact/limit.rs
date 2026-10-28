//! Per-network token buckets for `POST /api/contact` (docs/CONTRACTS.md §8.2): 10 briefs per hour
//! with a proof of work, 3 per hour without. In memory only, keyed by the client's IPv4 /24 or
//! IPv6 /48 — the full address is never kept, logged or written anywhere.

use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::{Mutex, PoisonError};
use std::time::Instant;

const PER_HOUR_WITH_POW: f64 = 10.0;
const PER_HOUR_WITHOUT_POW: f64 = 3.0;
/// Idle (full) buckets are dropped once the map holds this many networks.
const PRUNE_AT: usize = 10_000;
/// Beyond this many active networks, new ones are refused (fail closed under a flood).
const MAX_NETWORKS: usize = 100_000;

/// The network a client is in: the only trace of its address the server keeps.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum ClientNet {
    V4([u8; 3]),
    V6([u8; 6]),
}

impl ClientNet {
    /// IPv4 /24 or IPv6 /48 (IPv4-mapped IPv6 counts as IPv4).
    pub fn of(ip: IpAddr) -> ClientNet {
        match ip.to_canonical() {
            IpAddr::V4(v4) => {
                let [a, b, c, _] = v4.octets();
                ClientNet::V4([a, b, c])
            }
            IpAddr::V6(v6) => {
                let o = v6.octets();
                ClientNet::V6([o[0], o[1], o[2], o[3], o[4], o[5]])
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct Bucket {
    tokens: f64,
    at: Instant,
}

impl Bucket {
    fn full(capacity: f64, now: Instant) -> Self {
        Bucket { tokens: capacity, at: now }
    }

    fn refill(&mut self, capacity: f64, now: Instant) {
        let hours = now.saturating_duration_since(self.at).as_secs_f64() / 3600.0;
        self.tokens = (self.tokens + hours * capacity).min(capacity);
        self.at = now;
    }

    fn take(&mut self, capacity: f64, now: Instant) -> bool {
        self.refill(capacity, now);
        if self.tokens >= 1.0 {
            self.tokens -= 1.0;
            true
        } else {
            false
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct Buckets {
    with_pow: Bucket,
    without_pow: Bucket,
}

/// The limiter.
#[derive(Default)]
pub(crate) struct RateLimiter {
    nets: Mutex<HashMap<ClientNet, Buckets>>,
}

impl RateLimiter {
    /// Takes one token for `net`; `false` means "rate limited".
    pub fn admit(&self, net: ClientNet, with_pow: bool, now: Instant) -> bool {
        let mut nets = self.nets.lock().unwrap_or_else(PoisonError::into_inner);
        if nets.len() >= PRUNE_AT && !nets.contains_key(&net) {
            nets.retain(|_, b| {
                b.with_pow.refill(PER_HOUR_WITH_POW, now);
                b.without_pow.refill(PER_HOUR_WITHOUT_POW, now);
                b.with_pow.tokens < PER_HOUR_WITH_POW || b.without_pow.tokens < PER_HOUR_WITHOUT_POW
            });
            if nets.len() >= MAX_NETWORKS {
                return false;
            }
        }
        let buckets = nets.entry(net).or_insert_with(|| Buckets {
            with_pow: Bucket::full(PER_HOUR_WITH_POW, now),
            without_pow: Bucket::full(PER_HOUR_WITHOUT_POW, now),
        });
        if with_pow {
            buckets.with_pow.take(PER_HOUR_WITH_POW, now)
        } else {
            buckets.without_pow.take(PER_HOUR_WITHOUT_POW, now)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn prefixes() {
        let a = ClientNet::of("203.0.113.7".parse().unwrap());
        assert_eq!(a, ClientNet::of("203.0.113.250".parse().unwrap()));
        assert_eq!(a, ClientNet::of("::ffff:203.0.113.9".parse().unwrap()));
        assert_ne!(a, ClientNet::of("203.0.114.7".parse().unwrap()));
        let b = ClientNet::of("2001:db8:1:2::1".parse().unwrap());
        assert_eq!(b, ClientNet::of("2001:db8:1:ffff::9".parse().unwrap()));
        assert_ne!(b, ClientNet::of("2001:db8:2::1".parse().unwrap()));
    }

    #[test]
    fn three_per_hour_without_pow_ten_with() {
        let limiter = RateLimiter::default();
        let net = ClientNet::V4([192, 0, 2]);
        let t0 = Instant::now();
        assert!((0..3).all(|_| limiter.admit(net, false, t0)));
        assert!(!limiter.admit(net, false, t0));
        assert!((0..10).all(|_| limiter.admit(net, true, t0)), "separate bucket");
        assert!(!limiter.admit(net, true, t0));
        // One token comes back every 20 minutes without PoW.
        assert!(limiter.admit(net, false, t0 + Duration::from_secs(20 * 60 + 1)));
        assert!(!limiter.admit(net, false, t0 + Duration::from_secs(20 * 60 + 2)));
        assert!(limiter.admit(ClientNet::V4([192, 0, 3]), false, t0), "other networks unaffected");
    }
}
