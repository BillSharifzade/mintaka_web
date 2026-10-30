//! The A/B test's counts (docs/CONTRACTS.md §17): page views and conversions per variant, since
//! the process started — a test is judged on all of its data, so there is no 24-hour window.
//!
//! A page view is one exposure, counted from its first beacon; a conversion is a page view that
//! reached a metric at least once, reported by the browser the first time it happens. Each
//! variant stops counting at the planned sample size, and once every variant has, the variants
//! are compared with the control, once: the result is final, because nothing changes after it.

use anyhow::ensure;
use proof::experiment::z_test;
use proof::rum::EVENTS;
use proof::{Experiment, RumComparison, RumExperiment, RumVariant};

/// Page views and conversions of one variant.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct Tally {
    views: u64,
    /// Per metric of [`Test::metrics`] (the primary one first).
    conversions: Vec<u64>,
}

/// The running test.
pub(crate) struct Test {
    spec: Experiment,
    /// Indexes into the aggregator's route list: the pages that serve the variants.
    pages: Vec<usize>,
    tallies: Vec<Tally>,
    /// Set once every variant has its planned page views.
    result: Option<Vec<RumComparison>>,
}

impl Test {
    /// `pages`: the route indexes of the pages that serve a variant.
    pub fn new(spec: Experiment, pages: Vec<usize>) -> anyhow::Result<Test> {
        let metrics: Vec<&str> = std::iter::once(&spec.metric).chain(&spec.secondary).map(String::as_str).collect();
        for (i, m) in metrics.iter().enumerate() {
            ensure!(EVENTS.contains(m), "`experiment` {:?}: metric {m:?} is not an event key", spec.id);
            ensure!(!metrics[..i].contains(m), "`experiment` {:?}: metric {m:?} is listed twice", spec.id);
        }
        for (i, v) in spec.variants.iter().enumerate() {
            ensure!(!spec.variants[..i].contains(v), "`experiment` {:?}: variant {v:?} is listed twice", spec.id);
        }
        ensure!(spec.variants.len() >= 2, "`experiment` {:?} needs a control and another variant", spec.id);
        ensure!(spec.plan.sample_size >= 1, "`experiment` {:?}: sample_size must be at least 1", spec.id);
        let tally = Tally { views: 0, conversions: vec![0; metrics.len()] };
        Ok(Test { tallies: vec![tally; spec.variants.len()], spec, pages, result: None })
    }

    /// The index of a variant id.
    pub fn variant(&self, id: &str) -> Option<usize> {
        self.spec.variants.iter().position(|v| v == id)
    }

    /// The index of a metric among the primary and secondary ones, and its [`EVENTS`] index.
    pub fn metric(&self, key: &str) -> Option<(usize, usize)> {
        let i = std::iter::once(&self.spec.metric).chain(&self.spec.secondary).position(|m| m == key)?;
        Some((i, EVENTS.iter().position(|e| *e == key)?))
    }

    /// Does the page at this route index serve the variants?
    pub fn serves(&self, route: usize) -> bool {
        self.pages.contains(&route)
    }

    /// One beacon of a page view that showed `variant`: `view` on its first, `converted` the
    /// metrics (by [`Test::metric`] index) it reached for the first time. A variant that already
    /// has its planned page views counts nothing more — not even a conversion of one of those page
    /// views reported late, since a late report can't be told from a page view after the stop.
    pub fn record(&mut self, variant: usize, view: bool, converted: &[usize]) {
        let planned = self.spec.plan.sample_size;
        let Some(tally) = self.tallies.get_mut(variant) else { return };
        if self.result.is_some() || tally.views >= planned {
            return;
        }
        tally.views += u64::from(view);
        for &m in converted {
            if let Some(n) = tally.conversions.get_mut(m) {
                *n += 1;
            }
        }
        if self.tallies.iter().all(|t| t.views >= planned) {
            self.result = Some(self.compare());
        }
    }

    /// Every variant after the control against it, on the primary metric, each at its share of
    /// the family-wise false-positive rate.
    fn compare(&self) -> Vec<RumComparison> {
        let alpha = self.spec.plan.alpha_each(self.tallies.len());
        let control = &self.tallies[0];
        self.tallies[1..]
            .iter()
            .zip(&self.spec.variants[1..])
            .map(|(t, id)| {
                let test = z_test(control.views, control.conversions[0], t.views, t.conversions[0]);
                RumComparison { variant: id.clone(), z: test.z, p: test.p, significant: test.p < alpha }
            })
            .collect()
    }

    /// `GET /_rum` → `experiment`.
    pub fn summary(&self) -> RumExperiment {
        let variants = self.spec.variants.iter().zip(&self.tallies).map(|(id, t)| RumVariant {
            id: id.clone(),
            views: t.views,
            conversions: t.conversions[0],
            secondary: self.spec.secondary.iter().cloned().zip(t.conversions[1..].iter().copied()).collect(),
        });
        RumExperiment {
            id: self.spec.id.clone(),
            metric: self.spec.metric.clone(),
            secondary: self.spec.secondary.clone(),
            plan: self.spec.plan,
            variants: variants.collect(),
            result: self.result.clone(),
        }
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use proof::Plan;

    pub(crate) fn spec(sample_size: u64) -> Experiment {
        Experiment {
            id: "hero-headline".into(),
            variants: ["hero-title", "hero-title-hardware", "hero-title-cycle"].map(String::from).to_vec(),
            metric: "cta_start_project".into(),
            secondary: vec!["contact_submit".into()],
            plan: Plan { baseline: 0.05, mde: 0.3, alpha: 0.05, power: 0.8, sample_size },
        }
    }

    #[test]
    fn counts_views_and_first_conversions_per_variant() {
        let mut t = Test::new(spec(10), vec![1]).unwrap();
        assert_eq!(t.metric("cta_start_project").map(|m| m.0), Some(0));
        assert_eq!(t.metric("contact_submit").map(|m| m.0), Some(1));
        assert_eq!(t.metric("proof_open"), None, "not a metric of this test");
        assert_eq!(t.variant("hero-title-cycle"), Some(2));
        assert!(t.serves(1) && !t.serves(0));
        t.record(0, true, &[]);
        t.record(0, true, &[0]);
        t.record(0, false, &[1]); // a later beacon of that page view: it sent the form too
        t.record(2, true, &[0, 1]);
        let s = t.summary();
        assert_eq!((s.variants[0].views, s.variants[0].conversions, s.variants[0].secondary["contact_submit"]), (2, 1, 1));
        assert_eq!((s.variants[1].views, s.variants[1].conversions), (0, 0));
        assert_eq!((s.variants[2].views, s.variants[2].conversions, s.variants[2].secondary["contact_submit"]), (1, 1, 1));
        assert_eq!(s.result, None, "no verdict before the planned size");
        assert_eq!(s.plan.sample_size, 10);
    }

    #[test]
    fn stops_at_the_planned_size_then_compares_once() {
        let mut t = Test::new(spec(4), vec![1]).unwrap();
        // The control converts 1 page view in 4, the others 3 in 4 and 1 in 4.
        for (variant, converting) in [(0, 1), (1, 3), (2, 1)] {
            for i in 0..4 {
                t.record(variant, true, if i < converting { &[0] } else { &[] });
            }
            if variant < 2 {
                assert_eq!(t.summary().result, None, "variant {variant}: the others aren't there yet");
            }
        }
        // Over the plan: counted nowhere.
        t.record(1, true, &[0]);
        t.record(0, false, &[0]);
        let s = t.summary();
        assert_eq!(s.variants.iter().map(|v| (v.views, v.conversions)).collect::<Vec<_>>(), [(4, 1), (4, 3), (4, 1)]);
        let result = s.result.expect("every variant has its 4 page views");
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].variant, "hero-title-hardware");
        // 1/4 against 3/4: pooled 1/2, standard error 0.35355, z = 1.4142, p = 0.1573.
        assert!((result[0].z - std::f64::consts::SQRT_2).abs() < 1e-9 && (result[0].p - 0.157299).abs() < 1e-5, "{result:?}");
        assert!(!result[0].significant, "p 0.157 is not below 0.025");
        assert_eq!((result[1].z, result[1].p, result[1].significant), (0.0, 1.0, false));
    }

    #[test]
    fn a_clear_difference_is_significant() {
        let mut t = Test::new(spec(1000), vec![1]).unwrap();
        for (variant, converting) in [(0, 50), (1, 90), (2, 52)] {
            for i in 0..1000 {
                t.record(variant, true, if i < converting { &[0] } else { &[] });
            }
        }
        let result = t.summary().result.unwrap();
        assert!(result[0].significant && result[0].z > 0.0, "{result:?}");
        assert!(!result[1].significant, "{result:?}");
    }

    #[test]
    fn refuses_a_malformed_test() {
        let mut bad = spec(10);
        bad.secondary = vec!["cta_start_project".into()];
        assert!(Test::new(bad, vec![]).is_err(), "a metric twice");
        let mut bad = spec(10);
        bad.metric = "page_view".into();
        assert!(Test::new(bad, vec![]).is_err(), "not an event");
        let mut bad = spec(10);
        bad.variants.truncate(1);
        assert!(Test::new(bad, vec![]).is_err(), "nothing to compare");
        assert!(Test::new(spec(0), vec![]).is_err(), "no stopping point");
    }
}
