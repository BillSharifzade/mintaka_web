//! The console challenge (spec §7.5): `POST /api/ctf/start`, `/api/ctf/puzzle`, `/api/ctf/flag`
//! (docs/CONTRACTS.md §8.3).
//!
//! The server derives every run token, puzzle and answer from a secret only it holds, so the
//! public repository shows how the puzzles are made but not what their answers are.

use serde::{Deserialize, Serialize};

/// Largest accepted request body on `/api/ctf/*`.
pub const CTF_BODY_LIMIT: usize = 1024;

/// Stages, one per pillar star, in page order. A run at stage `CTF_STAGES + 1` has solved them all.
pub const CTF_STAGES: u8 = 4;

/// The sentence stage two's ciphertext `a` hides. Every page's source starts with it — the
/// view-source greeting prints this very constant — and that is the clue.
pub const CTF_CRIB: &str = "You read page source. We'd like to meet you:";

/// Body of `puzzle` and `flag` (`start` takes none).
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct CtfRequest {
    /// The run token from the last answer.
    pub run: String,
    /// `flag` only: the answer to check.
    pub flag: Option<String>,
}

/// The answer of all three endpoints: where the run stands.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CtfState {
    /// `flag` only: whether the answer was right.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub correct: Option<bool>,
    /// The run token to keep. It changes each time a stage is solved; at the last stage it is
    /// also the pass the application form sends (`ctf_pass`, §8.2).
    pub run: String,
    /// 1–4, or 5 once every stage is solved.
    pub stage: u8,
    /// The current stage's data; `None` once every stage is solved.
    pub puzzle: Option<CtfPuzzle>,
}

/// One stage's data. The words that explain it are page copy (`content/{lang}/islands/ctf.ftl`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "star", rename_all = "lowercase")]
pub enum CtfPuzzle {
    /// Efficiency: a square sky of digits 1–9, one row per line.
    Rigel { grid: String },
    /// Security: two messages under one one-time pad, as hex.
    Saiph { a: String, b: String },
    /// Highload: the bytes of one connection's `struct tcp_info`, as hex.
    Betelgeuse { tcp_info: String },
    /// Design: pointing trials, `[distance px, width px, movement time ms]`.
    Meissa { trials: Vec<[u32; 3]> },
}

/// Error body: `{"error":"bad-run"}`, `"rate-limited"`, `"too-large"`, `"bad-request"`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CtfError {
    pub error: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn puzzles_are_tagged_by_star() {
        let p = CtfPuzzle::Saiph { a: "00".into(), b: "ff".into() };
        assert_eq!(serde_json::to_string(&p).unwrap(), r#"{"star":"saiph","a":"00","b":"ff"}"#);
        let s = CtfState { correct: None, run: "r".into(), stage: 5, puzzle: None };
        assert_eq!(serde_json::to_string(&s).unwrap(), r#"{"run":"r","stage":5,"puzzle":null}"#);
    }
}
