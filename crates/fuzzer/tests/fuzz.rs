//! Native tests for the S5 fuzzer: the bug is real, the fix holds, and guidance is what finds it.

use std::panic::{self, AssertUnwindSafe};

use fuzzer::engine::{Fuzzer, Mode, MAX_LEN};
use fuzzer::points::{Coverage, COUNT, P};
use fuzzer::rng::Rng;
use fuzzer::target::{parse, Build, ParseError};

/// The smallest chart that reaches the name section with a length byte larger than what follows.
const MINIMAL_CRASH: &[u8] = b"ORI\x02\x01\x00\x00\x00\x00N\x01";

fn silence_panics() {
    panic::set_hook(Box::new(|_| {}));
}

fn panic_message(payload: Box<dyn std::any::Any + Send>) -> String {
    if let Some(s) = payload.downcast_ref::<String>() {
        s.clone()
    } else if let Some(s) = payload.downcast_ref::<&str>() {
        (*s).to_string()
    } else {
        "<non-string panic>".to_string()
    }
}

/// Runs the fuzzer against `build` until it panics or `max_execs` inputs have run.
/// Returns (execution number, crashing input, panic message) for a crash.
fn fuzz(seed: u64, mode: Mode, build: Build, max_execs: u64) -> (Fuzzer, Option<(u64, Vec<u8>, String)>) {
    let mut fz = Fuzzer::new(seed, mode);
    while fz.execs() < max_execs {
        let input = fz.next_input().to_vec();
        let run = panic::catch_unwind(AssertUnwindSafe(|| {
            let mut cov = Coverage::default();
            let _ = parse(&input, build, &mut cov);
            cov.bits()
        }));
        match run {
            Ok(bits) => {
                fz.record(bits);
            }
            Err(payload) => {
                let crash = (fz.execs(), input, panic_message(payload));
                return (fz, Some(crash));
            }
        }
    }
    (fz, None)
}

/// A valid version-2 chart: Betelgeuse, Mintaka and a third star, two lines, named ORION.
fn orion() -> Vec<u8> {
    let mut v = b"ORI\x02\x03".to_vec();
    v.extend_from_slice(&[40, 30, 5, 0b1001]); // Betelgeuse: bright variable supergiant
    v.extend_from_slice(&[200, 120, 22, 0b0110]); // Mintaka: multiple giant, 2nd magnitude
    v.extend_from_slice(&[100, 220, 30, 0]); // a faint star
    v.extend_from_slice(b"C\x02\x00\x01\x01\x02"); // lines 0–1, 1–2
    v.extend_from_slice(b"N\x05ORION");
    v
}

#[test]
fn a_valid_chart_parses_in_both_builds() {
    let bytes = orion();
    for build in [Build::Buggy, Build::Fixed] {
        let mut cov = Coverage::default();
        let chart = parse(&bytes, build, &mut cov).expect("valid chart");
        assert_eq!(chart.stars, 3);
        assert_eq!(chart.lines, 2);
        assert_eq!(chart.name, Some(&b"ORION"[..]));
        for p in [P::MagicORI, P::Version2, P::Star3, P::RedSupergiant, P::BlueMultiple, P::LinesOk, P::NameUpper, P::End] {
            assert!(cov.contains(p), "{p:?} not reached");
        }
    }
}

#[test]
fn malformed_charts_are_rejected_cleanly() {
    let cases: &[(&[u8], ParseError)] = &[
        (b"", ParseError::BadMagic),
        (b"ORX", ParseError::BadMagic),
        (b"ORI", ParseError::Truncated),
        (b"ORI\x09", ParseError::BadVersion),
        (b"ORI\x02\x00", ParseError::NoStars),
        (b"ORI\x02\x08", ParseError::TooManyStars),
        (b"ORI\x02\x01\x00\x00", ParseError::Truncated),
        (b"ORI\x02\x01\x00\x00\x00\x10", ParseError::ReservedFlags),
        (b"ORI\x02\x01\x00\x00\x00\x00Z", ParseError::UnknownSection),
        (b"ORI\x02\x01\x00\x00\x00\x00C\x01\x00\x05", ParseError::BadLine),
    ];
    for (input, expected) in cases {
        for build in [Build::Buggy, Build::Fixed] {
            let mut cov = Coverage::default();
            assert_eq!(parse(input, build, &mut cov).err(), Some(*expected), "input {input:?}");
        }
    }
}

#[test]
fn the_bug_is_real_and_the_fix_holds() {
    silence_panics();
    let crashed = panic::catch_unwind(|| {
        let mut cov = Coverage::default();
        let _ = parse(MINIMAL_CRASH, Build::Buggy, &mut cov);
    });
    let message = panic_message(crashed.expect_err("the buggy build must panic"));
    assert_eq!(message, "range end index 12 out of range for slice of length 11");

    let mut cov = Coverage::default();
    assert_eq!(parse(MINIMAL_CRASH, Build::Fixed, &mut cov), Err(ParseError::Truncated));
}

#[test]
fn the_fixed_build_never_panics() {
    // Random bytes, and mutations of a valid chart so most inputs get deep into the parser.
    let mut rng = Rng::new(42);
    let base = orion();
    for _ in 0..2_000_000 {
        let mut input: Vec<u8> = if rng.chance(1, 2) {
            (0..rng.below(MAX_LEN + 1)).map(|_| rng.byte()).collect()
        } else {
            base.clone()
        };
        for _ in 0..1 + rng.below(4) {
            if !input.is_empty() {
                let i = rng.below(input.len());
                input[i] = rng.byte();
            }
            if rng.chance(1, 4) {
                input.truncate(rng.below(input.len() + 1));
            }
        }
        let mut cov = Coverage::default();
        let _ = parse(&input, Build::Fixed, &mut cov);
    }
}

#[test]
fn guided_fuzzing_finds_the_bug_from_four_bytes_of_nothing() {
    silence_panics();
    let mut found = Vec::new();
    for seed in 1..=20 {
        let (fz, crash) = fuzz(seed, Mode::Guided, Build::Buggy, 5_000_000);
        let (exec, input, message) = crash.unwrap_or_else(|| panic!("seed {seed}: no crash in 5M inputs"));
        assert!(message.contains("out of range for slice"), "seed {seed}: {message}");
        assert!(input.starts_with(b"ORI\x02"), "seed {seed}: crash input {input:?}");
        assert_eq!(fz.execs(), exec);
        found.push(exec);
    }
    found.sort_unstable();
    println!("execs to first crash over 20 seeds: min {} · median {} · max {}", found[0], found[10], found[19]);
}

#[test]
fn guided_fuzzing_cannot_break_the_fixed_build() {
    let (fz, crash) = fuzz(7, Mode::Guided, Build::Fixed, 3_000_000);
    assert!(crash.is_none());
    let reached = fz.coverage().count_ones();
    println!("fixed build: {reached} of {COUNT} points in 3M inputs, corpus {}", fz.corpus_len());
    assert!(reached >= 30, "only {reached} points reached");
    for p in [P::SectionName, P::NameOk, P::LinesOk] {
        assert!(fz.coverage() & (1 << p as u8) != 0, "{p:?} never reached");
    }
}

#[test]
fn blind_fuzzing_never_gets_past_the_header() {
    silence_panics();
    for seed in 1..=5 {
        let (fz, crash) = fuzz(seed, Mode::Blind, Build::Buggy, 2_000_000);
        assert!(crash.is_none(), "seed {seed}: blind fuzzing should not find the bug");
        assert!(fz.coverage() & (1 << P::CountOk as u8) == 0, "seed {seed}: blind fuzzing got deep");
        println!("blind seed {seed}: {} of {COUNT} points in 2M inputs", fz.coverage().count_ones());
    }
}

#[test]
fn runs_replay_exactly_from_their_seed() {
    silence_panics();
    let (_, a) = fuzz(99, Mode::Guided, Build::Buggy, 5_000_000);
    let (_, b) = fuzz(99, Mode::Guided, Build::Buggy, 5_000_000);
    assert_eq!(a.map(|c| (c.0, c.1)), b.map(|c| (c.0, c.1)));
}
