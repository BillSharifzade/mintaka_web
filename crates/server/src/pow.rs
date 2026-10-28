//! Proof of work for the contact form (docs/CONTRACTS.md §8.1).
//!
//! A challenge is `nonce[16] ‖ issued_at[8, BE] ‖ difficulty[1] ‖ mac[16]`, where `mac` is the
//! first 16 bytes of HMAC-SHA256(secret, nonce ‖ issued_at ‖ difficulty) and the secret is random
//! per process. The server keeps no state per issued challenge; it remembers *spent* nonces until
//! they expire, so each challenge is accepted once.

use std::collections::HashMap;
use std::sync::{Mutex, PoisonError};

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use hmac::{Hmac, Mac};
use proof::{POW_CHALLENGE_BYTES, POW_TTL_SECS, PowChallenge};
use rand::RngCore;
use sha2::{Digest, Sha256};

type HmacSha256 = Hmac<Sha256>;

const NONCE: usize = 16;
const SIGNED: usize = NONCE + 8 + 1;
/// Challenges dated this far in the future (clock steps) are still accepted.
const CLOCK_SKEW_SECS: u64 = 60;
/// Spent nonces are pruned when the set grows past this.
const PRUNE_AT: usize = 4096;

/// Why a solution was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PowError {
    /// Malformed, forged, insufficient work, or already spent.
    Invalid,
    /// Older than [`POW_TTL_SECS`].
    Expired,
}

/// Issues and verifies challenges.
pub(crate) struct Pow {
    secret: [u8; 32],
    difficulty: u8,
    /// nonce → unix second at which it expires.
    spent: Mutex<HashMap<[u8; NONCE], u64>>,
}

impl Pow {
    /// A fresh random secret: challenges from a previous process are invalid.
    pub fn new(difficulty: u8) -> Self {
        let mut secret = [0; 32];
        rand::rngs::OsRng.fill_bytes(&mut secret);
        Pow { secret, difficulty, spent: Mutex::new(HashMap::new()) }
    }

    /// First 16 bytes of HMAC-SHA256(secret, signed). `None` is unreachable in practice (HMAC
    /// takes keys of any length); callers fail closed on it.
    fn mac(&self, signed: &[u8]) -> Option<[u8; 16]> {
        let mut mac = <HmacSha256 as Mac>::new_from_slice(&self.secret).ok()?;
        mac.update(signed);
        let full = mac.finalize().into_bytes();
        let mut out = [0; 16];
        out.copy_from_slice(full.get(..16)?);
        Some(out)
    }

    /// A new challenge issued at `now` (unix seconds).
    pub fn issue(&self, now: u64) -> Option<PowChallenge> {
        let mut bytes = [0u8; POW_CHALLENGE_BYTES];
        rand::thread_rng().fill_bytes(&mut bytes[..NONCE]);
        bytes[NONCE..NONCE + 8].copy_from_slice(&now.to_be_bytes());
        bytes[NONCE + 8] = self.difficulty;
        let mac = self.mac(&bytes[..SIGNED])?;
        bytes[SIGNED..].copy_from_slice(&mac);
        Some(PowChallenge {
            challenge: URL_SAFE_NO_PAD.encode(bytes),
            difficulty: self.difficulty,
            expires_at: now + POW_TTL_SECS,
        })
    }

    /// Checks a solution and, if it is good, marks the challenge spent.
    pub fn verify(&self, challenge: &str, counter: u64, now: u64) -> Result<(), PowError> {
        let decoded = URL_SAFE_NO_PAD.decode(challenge.trim()).map_err(|_| PowError::Invalid)?;
        let bytes: [u8; POW_CHALLENGE_BYTES] = decoded.try_into().map_err(|_| PowError::Invalid)?;
        let expected = self.mac(&bytes[..SIGNED]).ok_or(PowError::Invalid)?;
        if !constant_time_eq(&expected, &bytes[SIGNED..]) {
            return Err(PowError::Invalid);
        }
        let mut issued = [0; 8];
        issued.copy_from_slice(&bytes[NONCE..NONCE + 8]);
        let issued_at = u64::from_be_bytes(issued);
        let difficulty = bytes[NONCE + 8];
        if issued_at > now + CLOCK_SKEW_SECS || difficulty != self.difficulty {
            return Err(PowError::Invalid);
        }
        let expires_at = issued_at.saturating_add(POW_TTL_SECS);
        if now >= expires_at {
            return Err(PowError::Expired);
        }
        if leading_zero_bits(&solution_digest(&bytes, counter)) < u32::from(difficulty) {
            return Err(PowError::Invalid);
        }
        let mut nonce = [0; NONCE];
        nonce.copy_from_slice(&bytes[..NONCE]);
        let mut spent = self.spent.lock().unwrap_or_else(PoisonError::into_inner);
        if spent.len() >= PRUNE_AT {
            spent.retain(|_, expiry| *expiry > now);
        }
        match spent.insert(nonce, expires_at) {
            Some(_) => Err(PowError::Invalid),
            None => Ok(()),
        }
    }
}

/// `SHA-256(challenge_bytes ‖ counter[8, LE])` — a single 49-byte block.
pub(crate) fn solution_digest(challenge: &[u8; POW_CHALLENGE_BYTES], counter: u64) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(challenge);
    hasher.update(counter.to_le_bytes());
    hasher.finalize().into()
}

pub(crate) fn leading_zero_bits(digest: &[u8]) -> u32 {
    let mut bits = 0;
    for &byte in digest {
        if byte == 0 {
            bits += 8;
        } else {
            return bits + byte.leading_zeros();
        }
    }
    bits
}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    a.len() == b.len() && a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn solve(challenge: &str) -> u64 {
        let bytes: [u8; POW_CHALLENGE_BYTES] = URL_SAFE_NO_PAD.decode(challenge).unwrap().try_into().unwrap();
        let difficulty = u32::from(bytes[NONCE + 8]);
        (0..).find(|&c| leading_zero_bits(&solution_digest(&bytes, c)) >= difficulty).unwrap()
    }

    #[test]
    fn issue_solve_verify_once() {
        let pow = Pow::new(8);
        let c = pow.issue(1_000).unwrap();
        assert_eq!(URL_SAFE_NO_PAD.decode(&c.challenge).unwrap().len(), 41);
        assert_eq!(c.expires_at, 1_600);
        let counter = solve(&c.challenge);
        assert_eq!(pow.verify(&c.challenge, counter, 1_001), Ok(()));
        assert_eq!(pow.verify(&c.challenge, counter, 1_002), Err(PowError::Invalid), "replay");
    }

    #[test]
    fn expiry_and_forgery() {
        let pow = Pow::new(4);
        let c = pow.issue(1_000).unwrap();
        let counter = solve(&c.challenge);
        assert_eq!(pow.verify(&c.challenge, counter, 1_600), Err(PowError::Expired));
        // A challenge from another process (another secret) is forged as far as we know.
        let other = Pow::new(4).issue(1_000).unwrap();
        assert_eq!(pow.verify(&other.challenge, solve(&other.challenge), 1_001), Err(PowError::Invalid));
        // Tampering with the issue time breaks the MAC.
        let mut bytes = URL_SAFE_NO_PAD.decode(&c.challenge).unwrap();
        bytes[NONCE + 7] ^= 1;
        assert_eq!(pow.verify(&URL_SAFE_NO_PAD.encode(&bytes), counter, 1_001), Err(PowError::Invalid));
        assert_eq!(pow.verify("not base64!", 0, 1_001), Err(PowError::Invalid));
        assert_eq!(pow.verify("AAAA", 0, 1_001), Err(PowError::Invalid));
    }

    #[test]
    fn insufficient_work_is_invalid() {
        let pow = Pow::new(20);
        let c = pow.issue(1_000).unwrap();
        let bytes: [u8; POW_CHALLENGE_BYTES] = URL_SAFE_NO_PAD.decode(&c.challenge).unwrap().try_into().unwrap();
        let weak = (0..).find(|&n| leading_zero_bits(&solution_digest(&bytes, n)) < 20).unwrap();
        assert_eq!(pow.verify(&c.challenge, weak, 1_001), Err(PowError::Invalid));
    }

    #[test]
    fn zero_bits() {
        assert_eq!(leading_zero_bits(&[0, 0, 0x10]), 19);
        assert_eq!(leading_zero_bits(&[0x80]), 0);
        assert_eq!(leading_zero_bits(&[0, 0]), 16);
    }
}
