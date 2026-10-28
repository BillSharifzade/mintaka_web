//! S6 "Overload lab": a seeded discrete-event simulation of two services under the same load —
//! a naive one (one unbounded queue, a fixed pool, clients that time out and retry) and one
//! built our way (CoDel-style shedding, backpressure, autoscaling). Pure Rust, tested natively
//! (`cargo test -p loadsim`); `wasm.rs` is the WebAssembly surface used by
//! `web/instruments/loadsim/`.
//!
//! Output layout: [`sim::Lab::step`] returns `STRIDE` numbers per closed 250 ms window —
//! the window's end (ms of simulated time), then `FIELDS` numbers for the naive system, then
//! `FIELDS` for ours, in the order documented on [`sim::FIELDS`].

pub mod rng;
pub mod sim;

#[cfg(target_arch = "wasm32")]
mod wasm;
