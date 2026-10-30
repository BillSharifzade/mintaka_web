//! The A/B test on the landing page itself (spec §12, docs/CONTRACTS.md §17): what a build
//! declares (`manifest.experiment`) and the statistics both ends use — the generator to fix the
//! sample size before the test starts, the server to judge the result once it is reached. One
//! copy of each formula, so the two can never disagree. What `GET /_rum` reports about the test
//! is in [`crate::rum`].
//!
//! The method: every variant is compared with the control by a two-sided two-proportion z-test
//! (pooled variance under the null hypothesis), and the family-wise false-positive rate `alpha` is
//! split evenly over those comparisons (Bonferroni). The sample size per variant gives each
//! comparison `power` to detect a lift of `mde` over `baseline` — the normal approximation without
//! continuity correction, the same equation R's `power.prop.test` solves.

use serde::{Deserialize, Serialize};

/// `manifest.experiment`: the one test a build runs.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Experiment {
    /// Names the test, e.g. `hero-headline`.
    pub id: String,
    /// Message ids of the page strings, control first. The control's id is the message that
    /// varies: a page rendered as variant `v` reads message `v` wherever it reads the control.
    pub variants: Vec<String>,
    /// The primary metric, an event key of [`crate::rum::EVENTS`]: a page view converts when it
    /// happens at least once.
    pub metric: String,
    /// Event keys counted per variant the same way, for context. Never tested: the sample size is
    /// planned for `metric` alone.
    #[serde(default)]
    pub secondary: Vec<String>,
    pub plan: Plan,
}

/// The planning assumptions, fixed before the test starts, and the sample size they imply.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct Plan {
    /// Share of page views expected to convert with the control.
    pub baseline: f64,
    /// Minimum detectable effect, relative to `baseline`: 0.3 means 5 % → 6.5 %.
    pub mde: f64,
    /// Family-wise false-positive rate, two-sided, split evenly over the comparisons.
    pub alpha: f64,
    /// Chance that one comparison detects a true lift of `mde`.
    pub power: f64,
    /// Page views per variant, [`Plan::required`] of the above: the test's stopping point.
    pub sample_size: u64,
}

impl Plan {
    /// The false-positive rate of each comparison with the control: `alpha` over their number.
    pub fn alpha_each(&self, variants: usize) -> f64 {
        self.alpha / variants.saturating_sub(1).max(1) as f64
    }

    /// Page views per variant that these assumptions need for `variants` variants, rounded up.
    pub fn required(&self, variants: usize) -> u64 {
        let lifted = self.baseline * (1.0 + self.mde);
        sample_size(self.baseline, lifted, self.alpha_each(variants), self.power).ceil() as u64
    }
}

/// Page views per group for a two-sided test at level `alpha` to tell conversion rates `p1` and
/// `p2` apart with probability `power`. Unrounded.
pub fn sample_size(p1: f64, p2: f64, alpha: f64, power: f64) -> f64 {
    let pooled = (p1 + p2) / 2.0;
    let null = normal_quantile(1.0 - alpha / 2.0) * (2.0 * pooled * (1.0 - pooled)).sqrt();
    let alternative = normal_quantile(power) * (p1 * (1.0 - p1) + p2 * (1.0 - p2)).sqrt();
    ((null + alternative) / (p2 - p1)).powi(2)
}

/// A two-sided two-proportion z-test.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ZTest {
    /// Positive when the second rate is the higher one.
    pub z: f64,
    pub p: f64,
}

/// Tests `x2` conversions in `n2` page views against `x1` in `n1` (pooled variance). Without any
/// variation (an empty group, no conversions at all, or nothing else) there is no evidence either
/// way: z = 0, p = 1. Counts above their page views (a conversion whose page view's beacon was
/// lost) are read as all of them.
pub fn z_test(n1: u64, x1: u64, n2: u64, x2: u64) -> ZTest {
    if n1 == 0 || n2 == 0 {
        return ZTest { z: 0.0, p: 1.0 };
    }
    let (n1, n2) = (n1 as f64, n2 as f64);
    let (x1, x2) = ((x1 as f64).min(n1), (x2 as f64).min(n2));
    let pooled = (x1 + x2) / (n1 + n2);
    let se = (pooled * (1.0 - pooled) * (1.0 / n1 + 1.0 / n2)).sqrt();
    if se.is_nan() || se <= 0.0 {
        return ZTest { z: 0.0, p: 1.0 };
    }
    let z = (x2 / n2 - x1 / n1) / se;
    ZTest { z, p: erfc(z.abs() / std::f64::consts::SQRT_2) }
}

/// The standard normal distribution function Φ.
pub fn normal_cdf(x: f64) -> f64 {
    0.5 * erfc(-x / std::f64::consts::SQRT_2)
}

/// Φ⁻¹(p) for 0 < p < 1, by bisection on [`normal_cdf`] down to the last bit: slow by the
/// standards of a library, instant for the two quantiles a build needs, and as exact as Φ.
pub fn normal_quantile(p: f64) -> f64 {
    assert!(p > 0.0 && p < 1.0, "normal_quantile({p}): p must lie strictly between 0 and 1");
    // Through the lower tail, where Φ keeps its relative precision.
    if p > 0.5 {
        return -normal_quantile(1.0 - p);
    }
    let (mut low, mut high) = (-40.0_f64, 0.0_f64);
    loop {
        let mid = 0.5 * (low + high);
        if mid <= low || mid >= high {
            return mid;
        }
        if normal_cdf(mid) < p { low = mid } else { high = mid }
    }
}

/// The complementary error function, to about 1e-14 relative: below 2.5 from the series for erf
/// (A&S 7.1.6, every term positive, so nothing cancels), above it from the continued fraction
/// (A&S 7.1.14), evaluated from its tail.
pub fn erfc(x: f64) -> f64 {
    const FRAC_1_SQRT_PI: f64 = std::f64::consts::FRAC_2_SQRT_PI / 2.0;
    if x < 0.0 {
        return 2.0 - erfc(-x);
    }
    if x < 2.5 {
        // erf x = 2/√π · e^(−x²) · Σ 2ⁿ x^(2n+1) / (1 · 3 · … · (2n+1))
        let (mut term, mut sum, mut n) = (x, x, 0.0);
        while term > sum * f64::EPSILON / 4.0 {
            n += 1.0;
            term *= 2.0 * x * x / (2.0 * n + 1.0);
            sum += term;
        }
        1.0 - 2.0 * FRAC_1_SQRT_PI * (-x * x).exp() * sum
    } else {
        // erfc x = e^(−x²)/√π · 1/(x + ½/(x + 1/(x + 3⁄2/(x + 2/(x + …)))))
        let mut fraction = x;
        for k in (1..=60).rev() {
            fraction = x + f64::from(k) / 2.0 / fraction;
        }
        FRAC_1_SQRT_PI * (-x * x).exp() / fraction
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(got: f64, want: f64, relative: f64) -> bool {
        (got - want).abs() <= relative * want.abs().max(f64::MIN_POSITIVE)
    }

    #[test]
    fn erfc_matches_reference_values() {
        // Python 3 `math.erfc` (correctly rounded to the last digit or two).
        for (x, want) in [
            (0.0, 1.0),
            (0.1, 0.887537083981715),
            (0.5, 0.4795001221869535),
            (1.0, 0.15729920705028513),
            (2.0, 0.004677734981047266),
            (2.4, 0.0006885138966450789),
            (2.5, 0.0004069520174449589),
            (2.6, 0.00023603441652934908),
            (4.0, 1.541725790028002e-08),
            (6.0, 2.1519736712498913e-17),
            (10.0, 2.088487583762545e-45),
            (-1.0, 1.8427007929497148),
        ] {
            assert!(close(erfc(x), want, 1e-13), "erfc({x}) = {} ≠ {want}", erfc(x));
        }
    }

    #[test]
    fn quantiles_match_reference_values() {
        // Python 3 `statistics.NormalDist().inv_cdf` (Wichura's AS 241).
        for (p, want) in [
            (0.975, 1.9599639845400536),
            (0.9875, 2.2414027276049464),
            (0.8, 0.8416212335729144),
            (0.9, 1.2815515655446008),
            (0.025, -1.9599639845400538),
            (0.001, -3.090232306167813),
        ] {
            assert!(close(normal_quantile(p), want, 1e-12), "Φ⁻¹({p}) = {} ≠ {want}", normal_quantile(p));
        }
        assert!(normal_quantile(0.5).abs() < 1e-15);
        assert!(close(normal_cdf(normal_quantile(0.3)), 0.3, 1e-14));
    }

    #[test]
    fn sample_size_matches_r() {
        // R's documentation of `power.prop.test`:
        //   power.prop.test(p1 = .50, p2 = .75, power = .90)  # =>  n = 76.7
        let n = sample_size(0.50, 0.75, 0.05, 0.90);
        assert!((n - 76.7069).abs() < 1e-3, "{n}");
        // Smaller effects need more: four times the sample for half the difference, roughly.
        assert!(sample_size(0.50, 0.625, 0.05, 0.90) > 3.9 * n);
    }

    #[test]
    fn the_plan_splits_alpha_over_the_comparisons() {
        let plan = Plan { baseline: 0.05, mde: 0.3, alpha: 0.05, power: 0.8, sample_size: 0 };
        assert_eq!(plan.alpha_each(2), 0.05, "one comparison keeps all of alpha");
        assert_eq!(plan.alpha_each(3), 0.025);
        // 5 % → 6.5 %, alpha 0.025 two-sided, power 0.8: 4577.49 page views per variant.
        assert_eq!(plan.required(3), 4578);
        assert_eq!(plan.required(2), 3780, "at alpha 0.05: 3779.80");
    }

    #[test]
    fn z_test_reads_a_difference() {
        // 50/1000 against 75/1000: pooled rate 0.0625, standard error 0.0108253, difference 0.025,
        // so z = 2.3094 and p = 2 · (1 − Φ(2.3094)) = 0.020921.
        let t = z_test(1000, 50, 1000, 75);
        assert!((t.z - 2.3094).abs() < 1e-3, "{t:?}");
        assert!((t.p - 0.020921).abs() < 1e-5, "{t:?}");
        assert_eq!(z_test(1000, 75, 1000, 50).z, -t.z, "symmetric, signed");
        assert_eq!(z_test(500, 0, 500, 0), ZTest { z: 0.0, p: 1.0 }, "no conversions, no evidence");
        assert_eq!(z_test(10, 10, 10, 10), ZTest { z: 0.0, p: 1.0 });
        assert_eq!(z_test(0, 0, 10, 3), ZTest { z: 0.0, p: 1.0 }, "an empty group");
        let same = z_test(4578, 229, 4578, 229);
        assert_eq!((same.z, same.p), (0.0, 1.0));
        assert!(z_test(3, 9, 3, 0).z.is_finite(), "counts above their page views are capped");
    }
}
