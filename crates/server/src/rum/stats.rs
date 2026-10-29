//! The aggregates behind `GET /_rum`: one HDR histogram per Web Vitals metric plus counters, for
//! two windows — since the process started, and the last 24 hours as a ring of hourly slots.
//!
//! Why these windows: nothing is written to disk, so the honest outer bound is the process's own
//! life (it restarts with every deploy, so "since start" also means "this build"). The 24-hour
//! window keeps a long-running process from averaging over weeks of old traffic, in whole hours so
//! it costs one slot per hour with traffic and needs no timer: slots rotate lazily, on the next
//! beacon or read in a new hour.

use std::collections::BTreeMap;

use hdrhistogram::Histogram;
use proof::rum::{EVENTS, METRICS, RUM_MIN_SAMPLES};
use proof::{RumPercentile, RumRejected, RumSummary, RumWindow};

use super::beacon::Beacon;

/// Histogram units per wire unit, per [`METRICS`] entry: times in tenths of a millisecond, CLS in
/// ten-thousandths. Two significant figures: exact below 25.6 ms (CLS 0.0256), within 1 % above.
pub(crate) const SCALE: [f64; METRICS.len()] = [10.0, 10.0, 10.0, 10_000.0, 10.0];
const SIGFIG: u8 = 2;
/// Hourly slots in the day window.
const HOURS: u64 = 24;

fn histogram(metric: usize) -> anyhow::Result<Histogram<u32>> {
    let high = (METRICS[metric].max * SCALE[metric]) as u64;
    Ok(Histogram::new_with_bounds(1, high, SIGFIG)?)
}

/// Counts for one window or one hour.
struct Totals {
    views: u64,
    routes: Vec<u64>,
    metrics: [Histogram<u32>; METRICS.len()],
    events: [u64; EVENTS.len()],
}

impl Totals {
    fn new(routes: usize) -> anyhow::Result<Totals> {
        Ok(Totals {
            views: 0,
            routes: vec![0; routes],
            metrics: [histogram(0)?, histogram(1)?, histogram(2)?, histogram(3)?, histogram(4)?],
            events: [0; EVENTS.len()],
        })
    }

    fn reset(&mut self) {
        self.views = 0;
        self.routes.fill(0);
        self.metrics.iter_mut().for_each(Histogram::reset);
        self.events = [0; EVENTS.len()];
    }

    fn add(&mut self, b: &Beacon) {
        if b.view {
            self.views += 1;
            self.routes[b.route] += 1;
        }
        for (histogram, value) in self.metrics.iter_mut().zip(b.metrics) {
            if let Some(v) = value {
                histogram.saturating_record(v);
            }
        }
        for (count, n) in self.events.iter_mut().zip(b.events) {
            *count += u64::from(n);
        }
    }

    fn merge(&mut self, other: &Totals) {
        self.views += other.views;
        for (a, b) in self.routes.iter_mut().zip(&other.routes) {
            *a += b;
        }
        for (a, b) in self.metrics.iter_mut().zip(&other.metrics) {
            // Same bounds on both sides, so `add` cannot fail.
            let _ = a.add(b);
        }
        for (a, b) in self.events.iter_mut().zip(other.events) {
            *a += b;
        }
    }

    fn window(&self, from: u64, routes: &[String]) -> RumWindow {
        let metrics = self.metrics.iter().enumerate().map(|(i, h)| {
            let n = h.len();
            // The upper end of the 75th percentile's bucket: never better than what was measured.
            let p75 = (n >= RUM_MIN_SAMPLES).then(|| h.value_at_quantile(0.75) as f64 / SCALE[i]);
            (METRICS[i].name.to_owned(), RumPercentile { n, p75 })
        });
        let counted = |pairs: &mut dyn Iterator<Item = (&str, u64)>| -> BTreeMap<String, u64> {
            pairs.filter(|(_, n)| *n > 0).map(|(k, n)| (k.to_owned(), n)).collect()
        };
        RumWindow {
            from,
            views: self.views,
            routes: counted(&mut routes.iter().map(String::as_str).zip(self.routes.iter().copied())),
            metrics: metrics.collect(),
            events: counted(&mut EVENTS.into_iter().zip(self.events)),
        }
    }
}

/// One hour of the day window.
struct Hour {
    /// Unix time ÷ 3600.
    hour: u64,
    totals: Totals,
}

/// Everything `/_rum` keeps. Behind one mutex in [`super::Rum`]: a beacon costs a few histogram
/// increments, so there is nothing to shard.
pub(crate) struct Stats {
    routes: Vec<String>,
    since: u64,
    start: Totals,
    /// Indexed by hour % 24; allocated the first time an hour has traffic.
    ring: Vec<Option<Hour>>,
    /// Scratch space for summing the ring on a read.
    day: Totals,
    rejected: RumRejected,
    /// Bumped on every change; the cached summary is reused until it moves or the hour does.
    version: u64,
    cached: Option<(u64, u64, bytes::Bytes)>,
}

impl Stats {
    pub fn new(routes: Vec<String>, since: u64) -> anyhow::Result<Stats> {
        let n = routes.len();
        Ok(Stats {
            routes,
            since,
            start: Totals::new(n)?,
            ring: (0..HOURS).map(|_| None).collect(),
            day: Totals::new(n)?,
            rejected: RumRejected::default(),
            version: 0,
            cached: None,
        })
    }

    pub fn routes(&self) -> &[String] {
        &self.routes
    }

    pub fn record(&mut self, b: &Beacon, now: u64) -> anyhow::Result<()> {
        let hour = now / 3600;
        let slot = &mut self.ring[(hour % HOURS) as usize];
        match slot {
            Some(h) if h.hour == hour => {}
            Some(h) => {
                h.hour = hour;
                h.totals.reset();
            }
            None => *slot = Some(Hour { hour, totals: Totals::new(self.routes.len())? }),
        }
        if let Some(h) = slot {
            h.totals.add(b);
        }
        self.start.add(b);
        self.version += 1;
        Ok(())
    }

    pub fn reject(&mut self, rate_limited: bool) {
        if rate_limited {
            self.rejected.rate_limited += 1;
        } else {
            self.rejected.invalid += 1;
        }
        self.version += 1;
    }

    /// The `GET /_rum` body, recomputed only when something changed.
    pub fn summary(&mut self, now: u64) -> bytes::Bytes {
        let hour = now / 3600;
        if let Some((version, at, body)) = &self.cached
            && *version == self.version
            && *at == hour
        {
            return body.clone();
        }
        self.day.reset();
        let oldest = hour.saturating_sub(HOURS - 1);
        for h in self.ring.iter().flatten().filter(|h| (oldest..=hour).contains(&h.hour)) {
            self.day.merge(&h.totals);
        }
        let summary = RumSummary {
            since: self.since,
            now,
            min_n: RUM_MIN_SAMPLES,
            start: self.start.window(self.since, &self.routes),
            day: self.day.window((oldest * 3600).max(self.since), &self.routes),
            rejected: self.rejected,
        };
        let body = bytes::Bytes::from(serde_json::to_vec(&summary).unwrap_or_else(|_| b"null".to_vec()));
        self.cached = Some((self.version, hour, body.clone()));
        body
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rum::beacon::parse;

    const T0: u64 = 1_793_102_400; // a whole hour

    fn stats() -> Stats {
        Stats::new(["/", "/en/", "404"].map(String::from).to_vec(), T0).unwrap()
    }

    fn beacon(s: &Stats, json: &str) -> Beacon {
        parse(json.as_bytes(), s.routes()).unwrap()
    }

    fn summary(s: &mut Stats, now: u64) -> RumSummary {
        serde_json::from_slice(&s.summary(now)).unwrap()
    }

    #[test]
    fn percentiles_need_enough_samples() {
        let mut s = stats();
        for lcp in 1..=19 {
            let b = beacon(&s, &format!(r#"{{"route":"/en/","view":true,"metrics":{{"lcp":{}}}}}"#, lcp * 100));
            s.record(&b, T0 + 10).unwrap();
        }
        let r = summary(&mut s, T0 + 20);
        assert_eq!(r.start.metrics["lcp"], RumPercentile { n: 19, p75: None }, "19 < 20 samples");
        assert_eq!(r.start.metrics["inp"], RumPercentile { n: 0, p75: None });
        let b = beacon(&s, r#"{"route":"/en/","view":true,"metrics":{"lcp":2000}}"#);
        s.record(&b, T0 + 30).unwrap();
        let r = summary(&mut s, T0 + 40);
        let lcp = r.start.metrics["lcp"];
        assert_eq!(lcp.n, 20);
        // 100, 200 … 2000 ms: the 15th of 20 is 1500 ms, reported as the top of its 1 % bucket.
        let p75 = lcp.p75.unwrap();
        assert!((1500.0..=1515.0).contains(&p75), "{p75}");
        assert_eq!(r.start.views, 20);
        assert_eq!(r.start.routes["/en/"], 20);
        assert!(!r.start.routes.contains_key("/"), "routes without views are left out");
        assert_eq!(r.day, r.start, "within the first hour both windows agree");
    }

    #[test]
    fn small_values_and_zero_cls_are_exact() {
        let mut s = stats();
        for _ in 0..20 {
            let b = beacon(&s, r#"{"route":"/","view":true,"metrics":{"cls":0,"ttfb":0.4}}"#);
            s.record(&b, T0).unwrap();
        }
        let r = summary(&mut s, T0);
        assert_eq!(r.start.metrics["cls"].p75, Some(0.0));
        assert_eq!(r.start.metrics["ttfb"].p75, Some(0.4));
    }

    #[test]
    fn events_and_the_day_window() {
        let mut s = stats();
        let b = beacon(&s, r#"{"route":"/en/","view":true,"events":{"proof_open":2,"render_tier:A":1}}"#);
        s.record(&b, T0 + 60).unwrap();
        let later = T0 + 30 * 3600; // 30 hours on: the first hour has left the day window
        let b = beacon(&s, r#"{"route":"/en/","events":{"contact_submit":1}}"#);
        s.record(&b, later).unwrap();
        s.reject(false);
        s.reject(true);
        let r = summary(&mut s, later + 5);
        assert_eq!(r.since, T0);
        assert_eq!(r.start.views, 1, "an events-only beacon is not a page view");
        assert_eq!(r.start.events["proof_open"], 2);
        assert_eq!(r.start.events["contact_submit"], 1);
        assert_eq!(r.start.events.len(), 3, "only keys that happened");
        assert_eq!(r.day.views, 0);
        assert_eq!(r.day.events.len(), 1);
        assert_eq!(r.day.from, (later / 3600 - 23) * 3600);
        assert_eq!(r.rejected, RumRejected { invalid: 1, rate_limited: 1 });
    }

    #[test]
    fn a_slot_is_reused_a_day_later() {
        let mut s = stats();
        let b = beacon(&s, r#"{"route":"/","view":true}"#);
        s.record(&b, T0).unwrap();
        s.record(&b, T0 + 24 * 3600).unwrap(); // same slot, next day
        let r = summary(&mut s, T0 + 24 * 3600);
        assert_eq!(r.start.views, 2);
        assert_eq!(r.day.views, 1);
    }

    #[test]
    fn summaries_are_cached_until_something_changes() {
        let mut s = stats();
        let a = s.summary(T0);
        assert_eq!(s.summary(T0 + 1).as_ptr(), a.as_ptr(), "same version, same hour");
        let b = beacon(&s, r#"{"route":"/","view":true}"#);
        s.record(&b, T0 + 2).unwrap();
        assert_ne!(s.summary(T0 + 3), a);
    }
}
