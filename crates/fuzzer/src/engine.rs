//! A small coverage-guided fuzzer, in the spirit of libFuzzer and AFL.
//!
//! * **Guided:** pick an input from the corpus, apply a few random mutations, run it, and keep
//!   it if it reached a branch nobody reached before. Each kept input is a stepping stone to the
//!   next — that is how the fuzzer learns the format with no dictionary and no hints.
//! * **Blind:** random bytes every time, no feedback. The control group.
//!
//! The engine never runs the target itself: callers take [`Fuzzer::next_input`], run the parser,
//! and hand back the coverage with [`Fuzzer::record`]. That keeps crash handling with the host
//! (a trap in WebAssembly, `catch_unwind` in native tests).

use crate::rng::Rng;

/// Longest input the fuzzer will produce.
pub const MAX_LEN: usize = 64;

/// The single starting input: four bytes that are not a star chart.
pub const SEED: &[u8] = b"star";

/// Byte values that sit on common boundaries.
const INTERESTING: [u8; 8] = [0x00, 0x01, 0x02, 0x07, 0x10, 0x7F, 0x80, 0xFF];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Guided,
    Blind,
}

/// The first time one or more coverage points were reached.
#[derive(Clone, Debug)]
pub struct Event {
    /// 1-based execution number.
    pub exec: u64,
    /// Points reached for the first time by this input.
    pub new_bits: u64,
    /// The input that reached them.
    pub input: Vec<u8>,
}

pub struct Fuzzer {
    rng: Rng,
    mode: Mode,
    corpus: Vec<Vec<u8>>,
    global: u64,
    events: Vec<Event>,
    execs: u64,
    input: Vec<u8>,
}

impl Fuzzer {
    pub fn new(seed: u64, mode: Mode) -> Self {
        Fuzzer {
            rng: Rng::new(seed),
            mode,
            corpus: vec![SEED.to_vec()],
            global: 0,
            events: Vec::new(),
            execs: 0,
            input: Vec::with_capacity(MAX_LEN),
        }
    }

    /// Produces the next input to execute and counts the execution.
    pub fn next_input(&mut self) -> &[u8] {
        self.execs += 1;
        match self.mode {
            Mode::Guided => self.mutate(),
            Mode::Blind => {
                self.input.clear();
                let len = 1 + self.rng.below(MAX_LEN);
                for _ in 0..len {
                    let b = self.rng.byte();
                    self.input.push(b);
                }
            }
        }
        &self.input
    }

    /// Records the coverage reached by the last input. Returns true if it found something new.
    pub fn record(&mut self, coverage: u64) -> bool {
        let new_bits = coverage & !self.global;
        if new_bits == 0 {
            return false;
        }
        self.global |= new_bits;
        self.events.push(Event { exec: self.execs, new_bits, input: self.input.clone() });
        if self.mode == Mode::Guided {
            self.corpus.push(self.input.clone());
        }
        true
    }

    pub fn execs(&self) -> u64 {
        self.execs
    }

    pub fn coverage(&self) -> u64 {
        self.global
    }

    pub fn corpus_len(&self) -> usize {
        self.corpus.len()
    }

    pub fn events(&self) -> &[Event] {
        &self.events
    }

    /// The input most recently returned by [`next_input`](Self::next_input).
    pub fn current(&self) -> &[u8] {
        &self.input
    }

    fn mutate(&mut self) {
        // Favour the newest discovery: it is the deepest stepping stone we have.
        let pick = if self.rng.chance(1, 2) {
            self.corpus.len() - 1
        } else {
            self.rng.below(self.corpus.len())
        };
        self.input.clear();
        self.input.extend_from_slice(&self.corpus[pick]);
        let rounds = 1usize << self.rng.below(3);
        for _ in 0..rounds {
            self.mutate_once();
        }
        if self.input.is_empty() {
            let b = self.rng.byte();
            self.input.push(b);
        }
    }

    fn mutate_once(&mut self) {
        let len = self.input.len();
        match self.rng.below(10) {
            // Flip one bit.
            0 if len > 0 => {
                let i = self.rng.below(len);
                self.input[i] ^= 1 << self.rng.below(8);
            }
            // Random byte.
            1 if len > 0 => {
                let i = self.rng.below(len);
                self.input[i] = self.rng.byte();
            }
            // Boundary value.
            2 if len > 0 => {
                let i = self.rng.below(len);
                self.input[i] = INTERESTING[self.rng.below(INTERESTING.len())];
            }
            // Small arithmetic.
            3 if len > 0 => {
                let i = self.rng.below(len);
                let d = 1 + self.rng.below(16) as u8;
                self.input[i] = if self.rng.chance(1, 2) {
                    self.input[i].wrapping_add(d)
                } else {
                    self.input[i].wrapping_sub(d)
                };
            }
            // Insert one random byte.
            4 if len < MAX_LEN => {
                let i = self.rng.below(len + 1);
                let b = self.rng.byte();
                self.input.insert(i, b);
            }
            // Insert a run of random bytes.
            5 if len < MAX_LEN => {
                let i = self.rng.below(len + 1);
                let k = (1 + self.rng.below(8)).min(MAX_LEN - len);
                for _ in 0..k {
                    let b = self.rng.byte();
                    self.input.insert(i, b);
                }
            }
            // Delete a few bytes.
            6 if len > 1 => {
                let i = self.rng.below(len);
                let k = 1 + self.rng.below((len - i).min(4));
                self.input.drain(i..i + k);
            }
            // Duplicate a chunk of itself.
            7 if len > 0 && len < MAX_LEN => {
                let i = self.rng.below(len);
                let k = (1 + self.rng.below((len - i).min(8))).min(MAX_LEN - len);
                let j = self.rng.below(len + 1);
                let chunk = self.input[i..i + k].to_vec();
                self.input.splice(j..j, chunk);
            }
            // Splice with another corpus entry.
            8 if self.corpus.len() > 1 => {
                let other = self.rng.below(self.corpus.len());
                let cut_a = self.rng.below(len + 1);
                let cut_b = self.rng.below(self.corpus[other].len() + 1);
                self.input.truncate(cut_a);
                let tail = &self.corpus[other][cut_b..];
                let room = MAX_LEN - self.input.len();
                self.input.extend_from_slice(&tail[..tail.len().min(room)]);
            }
            // Swap two bytes.
            9 if len > 1 => {
                let i = self.rng.below(len);
                let j = self.rng.below(len);
                self.input.swap(i, j);
            }
            _ => {}
        }
    }
}
