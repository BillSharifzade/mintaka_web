//! # S5 · Secure by attack — a fuzzer, attacking our own code
//!
//! A small coverage-guided fuzzer (in the spirit of libFuzzer and AFL) attacks a small parser
//! for a binary star-chart format. The parser ships in two builds:
//!
//! * [`Build::Buggy`] trusts a length byte from its input — one line, one real bug.
//! * [`Build::Fixed`] is the same code with that line fixed.
//!
//! On the page the visitor watches the fuzzer discover the format byte by byte, light up the
//! code paths it reaches, find the bug, and then fail to break the fixed build.
//!
//! Everything runs inside the visitor's browser and only ever touches this parser. Nothing is
//! sent anywhere and no other system is involved: this is the defensive practice of attacking
//! your own code before release, made visible.
//!
//! [`Build::Buggy`]: target::Build::Buggy
//! [`Build::Fixed`]: target::Build::Fixed

pub mod engine;
pub mod points;
pub mod rng;
pub mod target;

#[cfg(target_arch = "wasm32")]
mod wasm;
