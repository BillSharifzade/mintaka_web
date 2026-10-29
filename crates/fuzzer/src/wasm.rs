//! WebAssembly exports: a plain C ABI, no bindgen. The host reads results straight from memory.
//!
//! When the buggy parser panics, the panic hook copies Rust's own message into `PANIC_MSG`, then
//! the module aborts and the host catches the trap (`WebAssembly.RuntimeError`).
//!
//! Everything the host reads lives in atomics *outside* the fuzzer's `RefCell`: a panic aborts in
//! the middle of `fz_run`, with the cell still borrowed, and the host must still be able to read
//! the crashing input, the execution count, the message and every discovery made before it. The
//! getters below only load atomics, so they are safe to call after a trap. The host then discards
//! the instance and creates a fresh one for the next run.

use std::cell::RefCell;
use std::sync::atomic::{AtomicU8, AtomicU32, AtomicU64, Ordering::Relaxed};
use std::sync::Once;

use crate::engine::{Fuzzer, Mode, MAX_LEN};
use crate::points::{Coverage, COUNT};
use crate::target::{self, Build};

const MSG_CAP: usize = 512;
/// Every event reveals at least one new point, so there can never be more events than points.
const EVENT_CAP: usize = 64;

#[allow(clippy::declare_interior_mutable_const)]
const ZERO_INPUT: [AtomicU8; MAX_LEN] = [const { AtomicU8::new(0) }; MAX_LEN];

static CURRENT: [AtomicU8; MAX_LEN] = [const { AtomicU8::new(0) }; MAX_LEN];
static CURRENT_LEN: AtomicU32 = AtomicU32::new(0);
static EXECS: AtomicU64 = AtomicU64::new(0);
static COVERAGE: AtomicU64 = AtomicU64::new(0);
static CORPUS: AtomicU32 = AtomicU32::new(0);

static EVENT_COUNT: AtomicU32 = AtomicU32::new(0);
static EVENT_EXEC: [AtomicU64; EVENT_CAP] = [const { AtomicU64::new(0) }; EVENT_CAP];
static EVENT_BITS: [AtomicU64; EVENT_CAP] = [const { AtomicU64::new(0) }; EVENT_CAP];
static EVENT_LEN: [AtomicU32; EVENT_CAP] = [const { AtomicU32::new(0) }; EVENT_CAP];
static EVENT_INPUT: [[AtomicU8; MAX_LEN]; EVENT_CAP] = [ZERO_INPUT; EVENT_CAP];

static PANIC_MSG: [AtomicU8; MSG_CAP] = [const { AtomicU8::new(0) }; MSG_CAP];
static PANIC_LEN: AtomicU32 = AtomicU32::new(0);

static SCRATCH: [AtomicU8; MAX_LEN] = [const { AtomicU8::new(0) }; MAX_LEN];
static LAST_COV: AtomicU64 = AtomicU64::new(0);

thread_local! {
    static STATE: RefCell<Option<(Fuzzer, Build)>> = const { RefCell::new(None) };
}

fn install_panic_hook() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        std::panic::set_hook(Box::new(|info| {
            let text = info.to_string();
            let bytes = &text.as_bytes()[..text.len().min(MSG_CAP)];
            for (slot, b) in PANIC_MSG.iter().zip(bytes) {
                slot.store(*b, Relaxed);
            }
            PANIC_LEN.store(bytes.len() as u32, Relaxed);
        }));
    });
}

fn copy_into(dst: &[AtomicU8; MAX_LEN], src: &[u8]) {
    for (slot, b) in dst.iter().zip(src) {
        slot.store(*b, Relaxed);
    }
}

fn build_from(code: u32) -> Build {
    if code == 1 { Build::Fixed } else { Build::Buggy }
}

fn lo(v: u64) -> u32 {
    v as u32
}

fn hi(v: u64) -> u32 {
    (v >> 32) as u32
}

fn event(i: u32) -> Option<usize> {
    let i = i as usize;
    (i < (EVENT_COUNT.load(Relaxed) as usize).min(EVENT_CAP)).then_some(i)
}

/// Starts a run. `mode`: 0 guided, 1 blind. `build`: 0 buggy, 1 fixed.
#[unsafe(no_mangle)]
pub extern "C" fn fz_init(seed_lo: u32, seed_hi: u32, mode: u32, build: u32) {
    install_panic_hook();
    let seed = (u64::from(seed_hi) << 32) | u64::from(seed_lo);
    let mode = if mode == 1 { Mode::Blind } else { Mode::Guided };
    for a in [&EXECS, &COVERAGE, &LAST_COV] {
        a.store(0, Relaxed);
    }
    for a in [&CURRENT_LEN, &EVENT_COUNT, &PANIC_LEN] {
        a.store(0, Relaxed);
    }
    CORPUS.store(1, Relaxed);
    STATE.with(|cell| *cell.borrow_mut() = Some((Fuzzer::new(seed, mode), build_from(build))));
}

/// Executes up to `budget` inputs. A crash in the buggy build traps out of this call.
#[unsafe(no_mangle)]
pub extern "C" fn fz_run(budget: u32) -> u32 {
    STATE.with(|cell| {
        let mut state = cell.borrow_mut();
        let Some((fz, build)) = state.as_mut() else { return 0 };
        let build = *build;
        for _ in 0..budget {
            let input = fz.next_input();
            copy_into(&CURRENT, input);
            CURRENT_LEN.store(input.len() as u32, Relaxed);
            EXECS.fetch_add(1, Relaxed);
            let mut cov = Coverage::default();
            let _ = target::parse(input, build, &mut cov);
            if fz.record(cov.bits()) {
                let e = fz.events().last().expect("record just pushed an event");
                let n = EVENT_COUNT.load(Relaxed) as usize;
                if n < EVENT_CAP {
                    EVENT_EXEC[n].store(e.exec, Relaxed);
                    EVENT_BITS[n].store(e.new_bits, Relaxed);
                    EVENT_LEN[n].store(e.input.len() as u32, Relaxed);
                    copy_into(&EVENT_INPUT[n], &e.input);
                    EVENT_COUNT.store(n as u32 + 1, Relaxed);
                }
                COVERAGE.store(fz.coverage(), Relaxed);
                CORPUS.store(fz.corpus_len() as u32, Relaxed);
            }
        }
        budget
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn fz_execs() -> f64 {
    EXECS.load(Relaxed) as f64
}

#[unsafe(no_mangle)]
pub extern "C" fn fz_cov_lo() -> u32 {
    lo(COVERAGE.load(Relaxed))
}

#[unsafe(no_mangle)]
pub extern "C" fn fz_cov_hi() -> u32 {
    hi(COVERAGE.load(Relaxed))
}

#[unsafe(no_mangle)]
pub extern "C" fn fz_corpus_len() -> u32 {
    CORPUS.load(Relaxed)
}

#[unsafe(no_mangle)]
pub extern "C" fn fz_event_count() -> u32 {
    EVENT_COUNT.load(Relaxed).min(EVENT_CAP as u32)
}

#[unsafe(no_mangle)]
pub extern "C" fn fz_event_exec(i: u32) -> f64 {
    event(i).map_or(0.0, |i| EVENT_EXEC[i].load(Relaxed) as f64)
}

#[unsafe(no_mangle)]
pub extern "C" fn fz_event_bits_lo(i: u32) -> u32 {
    event(i).map_or(0, |i| lo(EVENT_BITS[i].load(Relaxed)))
}

#[unsafe(no_mangle)]
pub extern "C" fn fz_event_bits_hi(i: u32) -> u32 {
    event(i).map_or(0, |i| hi(EVENT_BITS[i].load(Relaxed)))
}

#[unsafe(no_mangle)]
pub extern "C" fn fz_event_input(i: u32) -> *const u8 {
    event(i).map_or(std::ptr::null(), |i| EVENT_INPUT[i].as_ptr().cast())
}

#[unsafe(no_mangle)]
pub extern "C" fn fz_event_input_len(i: u32) -> u32 {
    event(i).map_or(0, |i| EVENT_LEN[i].load(Relaxed))
}

/// The input being executed — after a trap, the one that crashed.
#[unsafe(no_mangle)]
pub extern "C" fn fz_current() -> *const u8 {
    CURRENT.as_ptr().cast()
}

#[unsafe(no_mangle)]
pub extern "C" fn fz_current_len() -> u32 {
    CURRENT_LEN.load(Relaxed)
}

/// Rust's panic message, captured by the hook just before the abort.
#[unsafe(no_mangle)]
pub extern "C" fn fz_panic() -> *const u8 {
    PANIC_MSG.as_ptr().cast()
}

#[unsafe(no_mangle)]
pub extern "C" fn fz_panic_len() -> u32 {
    PANIC_LEN.load(Relaxed)
}

/// A `MAX_LEN`-byte buffer the host writes a single input into, for [`fz_exec`].
#[unsafe(no_mangle)]
pub extern "C" fn fz_scratch() -> *mut u8 {
    SCRATCH.as_ptr().cast_mut().cast()
}

/// Runs the parser once on the first `len` scratch bytes. Returns 0 if the chart parsed,
/// otherwise the `ParseError` code. A crash in the buggy build traps out of this call.
#[unsafe(no_mangle)]
pub extern "C" fn fz_exec(len: u32, build: u32) -> u32 {
    install_panic_hook();
    let len = (len as usize).min(MAX_LEN);
    let mut input = [0u8; MAX_LEN];
    for (b, slot) in input.iter_mut().zip(&SCRATCH).take(len) {
        *b = slot.load(Relaxed);
    }
    let input = &input[..len];
    copy_into(&CURRENT, input);
    CURRENT_LEN.store(len as u32, Relaxed);
    LAST_COV.store(0, Relaxed);
    let mut cov = Coverage::default();
    let result = target::parse(input, build_from(build), &mut cov);
    LAST_COV.store(cov.bits(), Relaxed);
    match result {
        Ok(_) => 0,
        Err(e) => e as u32,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn fz_last_cov_lo() -> u32 {
    lo(LAST_COV.load(Relaxed))
}

#[unsafe(no_mangle)]
pub extern "C" fn fz_last_cov_hi() -> u32 {
    hi(LAST_COV.load(Relaxed))
}

#[unsafe(no_mangle)]
pub extern "C" fn fz_points() -> u32 {
    COUNT as u32
}

#[unsafe(no_mangle)]
pub extern "C" fn fz_max_len() -> u32 {
    MAX_LEN as u32
}
