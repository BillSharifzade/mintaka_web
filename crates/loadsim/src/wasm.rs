//! The WebAssembly surface (wasm32 only). JavaScript owns the clock and the seed; Rust owns the
//! simulated time. Everything crosses as numbers or a Float64Array.

use crate::sim::{self, Config, Lab as Core};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct Lab(Core);

#[wasm_bindgen]
impl Lab {
    /// A new run from a seed (JavaScript passes e.g. `Math.random() * 2 ** 32`).
    #[wasm_bindgen(constructor)]
    pub fn new(seed: f64) -> Lab {
        Lab(Core::new(seed as u64))
    }

    /// New requests per second, from now on.
    #[wasm_bindgen(js_name = setRate)]
    pub fn set_rate(&mut self, per_s: f64) {
        self.0.set_rate(per_s);
    }

    /// Advance simulated time by `ms`; every closed window, `STRIDE` numbers each.
    pub fn step(&mut self, ms: f64) -> Vec<f64> {
        self.0.step(ms)
    }

    /// Simulated milliseconds since the start.
    pub fn now(&self) -> f64 {
        self.0.now()
    }
}

/// The model's constants, so the page shows the real parameters:
/// `[STRIDE, FIELDS, WINDOW_MS, BASE_RATE, mean service ms, lognormal σ, timeout ms, retries,
///   retry jitter ms, queue cap, naive workers, our min workers, our max workers, boot ms,
///   target utilisation, idle ms before scale-in, shed target ms, shed interval ms,
///   queue limit per worker]`.
#[wasm_bindgen]
pub fn params() -> Vec<f64> {
    let (n, o) = (Config::naive(), Config::ours());
    let sigma = match n.service {
        sim::ServiceTime::LogNormal { sigma } => sigma,
        sim::ServiceTime::Exponential => 1.0,
    };
    let (a, s) = (o.autoscale.expect("ours autoscales"), o.shedding.expect("ours sheds"));
    vec![
        sim::STRIDE as f64,
        sim::FIELDS as f64,
        sim::WINDOW_MS,
        sim::BASE_RATE,
        n.mean_service_ms,
        sigma,
        n.timeout_ms,
        f64::from(n.retries),
        n.retry_jitter_ms,
        n.queue_cap as f64,
        f64::from(n.workers),
        f64::from(o.workers),
        f64::from(a.max_workers),
        a.start_ms,
        a.target_util,
        a.idle_ms,
        s.target_ms,
        s.interval_ms,
        f64::from(s.queue_per_worker),
    ]
}
