//! Self-hosted real-user measurements: `POST /_rum` (one beacon per page view) and `GET /_rum`
//! (the aggregates), docs/CONTRACTS.md §14.
//!
//! [`RumBeacon`] is the *unparsed* shape a browser sends; the server checks every field against
//! the closed vocabularies below ([`METRICS`], [`EVENTS`]) and the manifest's page routes before a
//! single number is counted. What leaves the server is [`RumSummary`]: percentiles and counts,
//! never a single beacon. While an A/B test runs (§17), beacons also name the variant their page
//! view showed, and the summary counts page views and conversions per variant ([`RumExperiment`]).

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::experiment::Plan;

/// Largest accepted `POST /_rum` body. A full beacon (every metric, every event key) is ~1 kB.
pub const RUM_BODY_LIMIT: usize = 2 * 1024;

/// Fewest samples for which `GET /_rum` reports a 75th percentile; below it `p75` is `null`.
pub const RUM_MIN_SAMPLES: u64 = 20;

/// Highest count one beacon may report for one event key.
pub const RUM_MAX_EVENT_COUNT: u32 = 20;

/// A Web Vitals metric: its wire name and the range a real measurement can take.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MetricSpec {
    pub name: &'static str,
    /// Inclusive upper bound (milliseconds, or unitless for CLS). The lower bound is 0.
    pub max: f64,
}

/// Every metric a beacon may carry, in wire order. Times are milliseconds from the start of the
/// navigation (minus `activationStart` for prerendered pages); CLS is unitless.
pub const METRICS: [MetricSpec; 5] = [
    MetricSpec { name: "ttfb", max: 60_000.0 },
    MetricSpec { name: "fcp", max: 60_000.0 },
    MetricSpec { name: "lcp", max: 60_000.0 },
    MetricSpec { name: "cls", max: 10.0 },
    MetricSpec { name: "inp", max: 60_000.0 },
];

/// Every event key a beacon may count (spec §12). `name:value` for events that carry one:
/// instruments by pillar (S2 physics is its own), console-challenge stages 1–4, render tiers A–D.
pub const EVENTS: [&str; 23] = [
    "cta_start_project",
    "proof_open",
    "instrument_start:physics",
    "instrument_start:efficiency",
    "instrument_start:security",
    "instrument_start:highload",
    "instrument_start:design",
    "instrument_complete:physics",
    "instrument_complete:efficiency",
    "instrument_complete:security",
    "instrument_complete:highload",
    "instrument_complete:design",
    "terminal_open",
    "ctf_stage:1",
    "ctf_stage:2",
    "ctf_stage:3",
    "ctf_stage:4",
    "contact_submit",
    "lang_switch",
    "render_tier:A",
    "render_tier:B",
    "render_tier:C",
    "render_tier:D",
];

/// `POST /_rum` body (sent with `navigator.sendBeacon`, so usually `text/plain`).
///
/// ```json
/// { "route": "/en/", "view": true, "variant": "hero-title-hardware",
///   "metrics": { "ttfb": 92.4, "fcp": 640.2, "lcp": 812.6, "cls": 0.0012, "inp": 48 },
///   "events": { "proof_open": 1, "cta_start_project": 1, "render_tier:B": 1 },
///   "converted": ["cta_start_project"] }
/// ```
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct RumBeacon {
    /// A page route from the manifest, `/`, or `404` for the not-found page. Never a free path.
    pub route: String,
    /// `true` on the first beacon of a page view (the only one that may carry metrics).
    #[serde(default)]
    pub view: bool,
    #[serde(default)]
    pub metrics: RumMetrics,
    /// [`EVENTS`] key → times it happened since the previous beacon of this page view (1–20).
    #[serde(default)]
    pub events: BTreeMap<String, u32>,
    /// The experiment variant this page view shows (§17), on every beacon of a page view our
    /// server sent with one; the first (`view: true`) is the exposure.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub variant: Option<String>,
    /// The experiment's metrics this page view reached for the first time, each at most once per
    /// page view, each also among `events`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub converted: Vec<String>,
}

/// The page view's Web Vitals; absent when the browser couldn't measure one.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct RumMetrics {
    pub ttfb: Option<f64>,
    pub fcp: Option<f64>,
    pub lcp: Option<f64>,
    pub cls: Option<f64>,
    pub inp: Option<f64>,
}

impl RumMetrics {
    /// The values in [`METRICS`] order.
    pub fn values(&self) -> [Option<f64>; 5] {
        [self.ttfb, self.fcp, self.lcp, self.cls, self.inp]
    }
}

/// `GET /_rum`: what real visitors' browsers reported to this server process.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RumSummary {
    /// Unix seconds when this process started counting (it keeps nothing across restarts).
    pub since: u64,
    /// Unix seconds when this summary was computed.
    pub now: u64,
    /// [`RUM_MIN_SAMPLES`]: below this many samples a percentile is `null`.
    pub min_n: u64,
    /// Since the process started.
    pub start: RumWindow,
    /// The last 24 hours, in whole hours (the current hour plus the 23 before it).
    pub day: RumWindow,
    /// Beacons refused since the process started (not counted anywhere above).
    pub rejected: RumRejected,
    /// The A/B test of this build since the process started; `null` when none runs.
    pub experiment: Option<RumExperiment>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct RumWindow {
    /// Unix seconds where the window begins (never before `since`).
    pub from: u64,
    /// Page views: beacons with `view: true`.
    pub views: u64,
    /// Page views per route (only routes with at least one).
    pub routes: BTreeMap<String, u64>,
    /// [`METRICS`] name → sample count and 75th percentile.
    pub metrics: BTreeMap<String, RumPercentile>,
    /// [`EVENTS`] key → count (only keys with at least one).
    pub events: BTreeMap<String, u64>,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq)]
pub struct RumPercentile {
    pub n: u64,
    /// Milliseconds (unitless for CLS), rounded up to the histogram's resolution; `null` below
    /// [`RUM_MIN_SAMPLES`] samples.
    pub p75: Option<f64>,
}

/// `GET /_rum` → `experiment`: page views and conversions per variant, and — only once every
/// variant has reached the planned sample size — each variant against the control.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RumExperiment {
    pub id: String,
    /// The primary metric (an [`EVENTS`] key).
    pub metric: String,
    /// Counted, never tested.
    pub secondary: Vec<String>,
    pub plan: Plan,
    /// Control first, in the manifest's order.
    pub variants: Vec<RumVariant>,
    /// Each variant after the control against it, in the same order. `null` until every variant
    /// has `plan.sample_size` page views (no peeking); final from then on, because counting stops.
    pub result: Option<Vec<RumComparison>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RumVariant {
    pub id: String,
    /// Page views that showed this variant, up to `plan.sample_size`.
    pub views: u64,
    /// Of those, the page views that reached the primary metric at least once.
    pub conversions: u64,
    /// The same per secondary metric.
    pub secondary: BTreeMap<String, u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RumComparison {
    /// The variant compared with the control.
    pub variant: String,
    /// Two-proportion z statistic; positive when the variant converted more often.
    pub z: f64,
    /// Two-sided p-value.
    pub p: f64,
    /// `p` below `plan.alpha` split over the comparisons: not likely to be chance.
    pub significant: bool,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct RumRejected {
    /// Malformed, out of range, unknown names or routes, too large, cross-site.
    pub invalid: u64,
    /// Over the per-network limit.
    pub rate_limited: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn beacon_shape_is_strict() {
        let json = r#"{"route":"/en/","view":true,"metrics":{"lcp":812.6,"cls":0},"events":{"proof_open":1}}"#;
        let b: RumBeacon = serde_json::from_str(json).unwrap();
        assert_eq!(b.metrics.values(), [None, None, Some(812.6), Some(0.0), None]);
        assert_eq!(b.events["proof_open"], 1);
        assert!(serde_json::from_str::<RumBeacon>(r#"{"route":"/en/","ip":"1.2.3.4"}"#).is_err());
        assert!(serde_json::from_str::<RumBeacon>(r#"{"route":"/en/","metrics":{"fid":3}}"#).is_err());
        assert!(serde_json::from_str::<RumBeacon>(r#"{"route":"/en/","events":{"x":-1}}"#).is_err());
        assert!(serde_json::from_str::<RumBeacon>(r#"{"view":true}"#).is_err(), "route is required");
        let b: RumBeacon = serde_json::from_str(r#"{"route":"/en/","view":true,"variant":"hero-title","converted":["cta_start_project"]}"#).unwrap();
        assert_eq!((b.variant.as_deref(), b.converted.as_slice()), (Some("hero-title"), &["cta_start_project".to_string()][..]));
        assert!(serde_json::from_str::<RumBeacon>(r#"{"route":"/en/","variant":2}"#).is_err());
    }

    #[test]
    fn vocabularies_are_unique() {
        let mut keys = EVENTS.to_vec();
        keys.sort_unstable();
        keys.dedup();
        assert_eq!(keys.len(), EVENTS.len());
        assert_eq!(METRICS.map(|m| m.name), ["ttfb", "fcp", "lcp", "cls", "inp"]);
    }
}
