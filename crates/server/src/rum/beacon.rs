//! A `POST /_rum` body → a [`Beacon`], or a [`Refusal`]. Parse, don't validate: once a `Beacon`
//! exists, its route is one of ours, every metric is in range and every event is a known key with
//! a sane count. Nothing in it can name a path, a person or a browser.

use proof::RumBeacon;
use proof::rum::{EVENTS, METRICS, RUM_MAX_EVENT_COUNT};

use super::stats::SCALE;

/// One accepted beacon, in the aggregator's own units.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Beacon {
    /// Index into the aggregator's route list.
    pub route: usize,
    /// First beacon of a page view: counts as a view and may carry metrics.
    pub view: bool,
    /// Per [`METRICS`] entry: the value in histogram units ([`SCALE`]), if measured.
    pub metrics: [Option<u64>; METRICS.len()],
    /// Per [`EVENTS`] entry: how many times it happened (0 = not reported).
    pub events: [u32; EVENTS.len()],
}

/// Why a beacon was refused. The reason goes back to the sender as fixed text; nothing from the
/// request is ever echoed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Refusal {
    /// Not JSON, or not the beacon shape (unknown fields included).
    Shape,
    /// Not `/`, `404` or a page route of the manifest.
    Route,
    /// A metric out of range, not finite, or on a beacon that isn't the page view's first.
    Metric,
    /// An unknown event key, or a count outside 1–20.
    Event,
    /// Neither a page view nor any event: nothing to count.
    Empty,
}

impl Refusal {
    pub fn reason(self) -> &'static str {
        match self {
            Refusal::Shape => "beacon refused: not the documented JSON shape\n",
            Refusal::Route => "beacon refused: unknown route\n",
            Refusal::Metric => "beacon refused: metric out of range\n",
            Refusal::Event => "beacon refused: unknown event or count\n",
            Refusal::Empty => "beacon refused: nothing to count\n",
        }
    }
}

/// Parses `body` against the routes the aggregator knows.
pub(crate) fn parse(body: &[u8], routes: &[String]) -> Result<Beacon, Refusal> {
    let raw: RumBeacon = serde_json::from_slice(body).map_err(|_| Refusal::Shape)?;
    let route = routes.iter().position(|r| *r == raw.route).ok_or(Refusal::Route)?;

    let mut metrics = [None; METRICS.len()];
    for (i, value) in raw.metrics.values().into_iter().enumerate() {
        let Some(v) = value else { continue };
        if !raw.view || !v.is_finite() || !(0.0..=METRICS[i].max).contains(&v) {
            return Err(Refusal::Metric);
        }
        metrics[i] = Some((v * SCALE[i]).round() as u64);
    }

    let mut events = [0; EVENTS.len()];
    for (key, &n) in &raw.events {
        let i = EVENTS.iter().position(|e| e == key).ok_or(Refusal::Event)?;
        if !(1..=RUM_MAX_EVENT_COUNT).contains(&n) {
            return Err(Refusal::Event);
        }
        events[i] = n;
    }

    if !raw.view && events.iter().all(|&n| n == 0) {
        return Err(Refusal::Empty);
    }
    Ok(Beacon { route, view: raw.view, metrics, events })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn routes() -> Vec<String> {
        ["/", "/en/", "/en/privacy/", "404"].map(String::from).to_vec()
    }

    fn ok(json: &str) -> Beacon {
        parse(json.as_bytes(), &routes()).unwrap_or_else(|e| panic!("{json}: {e:?}"))
    }

    fn refused(json: &str) -> Refusal {
        parse(json.as_bytes(), &routes()).expect_err(json)
    }

    #[test]
    fn a_full_page_view() {
        let b = ok(r#"{"route":"/en/","view":true,
            "metrics":{"ttfb":92.44,"fcp":640.2,"lcp":812.6,"cls":0.0012,"inp":48},
            "events":{"proof_open":1,"instrument_start:highload":2,"render_tier:B":1}}"#);
        assert_eq!(b.route, 1);
        assert!(b.view);
        // Times in tenths of a millisecond, CLS in ten-thousandths.
        assert_eq!(b.metrics, [Some(924), Some(6402), Some(8126), Some(12), Some(480)]);
        let i = |k: &str| EVENTS.iter().position(|e| *e == k).unwrap();
        assert_eq!(b.events[i("proof_open")], 1);
        assert_eq!(b.events[i("instrument_start:highload")], 2);
        assert_eq!(b.events[i("render_tier:B")], 1);
        assert_eq!(b.events.iter().sum::<u32>(), 4);
    }

    #[test]
    fn edges_that_are_allowed() {
        assert_eq!(ok(r#"{"route":"/","view":true,"metrics":{"cls":0,"ttfb":0}}"#).metrics[3], Some(0));
        assert_eq!(ok(r#"{"route":"404","view":true}"#).route, 3, "a view without metrics");
        assert_eq!(ok(r#"{"route":"/en/","view":true,"metrics":{"lcp":60000}}"#).metrics[2], Some(600_000));
        assert!(!ok(r#"{"route":"/en/","events":{"contact_submit":1}}"#).view, "a later, events-only beacon");
        assert_eq!(ok(r#"{"route":"/en/","events":{"ctf_stage:4":20}}"#).events.iter().sum::<u32>(), 20);
    }

    #[test]
    fn refusals() {
        assert_eq!(refused("not json"), Refusal::Shape);
        assert_eq!(refused(r#"{"route":"/en/","view":true,"ua":"Firefox"}"#), Refusal::Shape);
        assert_eq!(refused(r#"{"route":"/en/","view":"yes"}"#), Refusal::Shape);
        assert_eq!(refused(r#"{"route":"/en/","events":{"proof_open":1.5}}"#), Refusal::Shape);
        // Free-form paths never get in, not even near misses.
        for route in ["/en", "/en/?q=1", "/admin/", "/EN/", "", "/en/privacy"] {
            assert_eq!(refused(&format!(r#"{{"route":"{route}","view":true}}"#)), Refusal::Route, "{route}");
        }
        assert_eq!(refused(r#"{"route":"/en/","view":true,"metrics":{"lcp":-1}}"#), Refusal::Metric);
        assert_eq!(refused(r#"{"route":"/en/","view":true,"metrics":{"lcp":60000.1}}"#), Refusal::Metric);
        assert_eq!(refused(r#"{"route":"/en/","view":true,"metrics":{"cls":10.5}}"#), Refusal::Metric);
        assert_eq!(refused(r#"{"route":"/en/","metrics":{"lcp":800},"events":{"lang_switch":1}}"#), Refusal::Metric);
        assert_eq!(refused(r#"{"route":"/en/","events":{"proof_opened":1}}"#), Refusal::Event);
        assert_eq!(refused(r#"{"route":"/en/","events":{"ctf_stage:5":1}}"#), Refusal::Event);
        assert_eq!(refused(r#"{"route":"/en/","events":{"render_tier:E":1}}"#), Refusal::Event);
        assert_eq!(refused(r#"{"route":"/en/","events":{"proof_open":0}}"#), Refusal::Event);
        assert_eq!(refused(r#"{"route":"/en/","events":{"proof_open":21}}"#), Refusal::Event);
        assert_eq!(refused(r#"{"route":"/en/"}"#), Refusal::Empty);
        assert_eq!(refused(r#"{"route":"/en/","view":false,"events":{}}"#), Refusal::Empty);
    }
}
