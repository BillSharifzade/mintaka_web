//! The console challenge (spec §7.5, docs/CONTRACTS.md §8.3): four puzzles, one per pillar star.
//! The HTTP side is `routes::ctf`.
//!
//! Nothing is stored per player. A run is a random nonce plus the stage it has reached, carried
//! in a token the server MACs with its secret; each stage's puzzle and answer are derived from
//! the same secret and nonce. So a run replays identically, a forged token is refused, and the
//! public source shows how the puzzles are made but not one answer.

mod puzzles;

use std::collections::HashMap;
use std::fs::{DirBuilder, OpenOptions};
use std::io::{ErrorKind, Read as _, Write as _};
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt};
use std::path::Path;
use std::sync::{Mutex, PoisonError};
use std::time::Instant;

use anyhow::Context;
use base64::Engine as _;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use hmac::{Hmac, Mac};
use proof::{CTF_STAGES, CtfPuzzle};
use rand::RngCore;
use sha2::Sha256;

use crate::contact::ClientNet;

type HmacSha256 = Hmac<Sha256>;

const NONCE: usize = 16;
const TAG: usize = 16;
/// `nonce[16] ‖ stage[1] ‖ tag[16]`, base64url, behind a version prefix.
const TOKEN_PREFIX: &str = "r1.";

/// One player's run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Run {
    nonce: [u8; NONCE],
    /// 1–4, or 5 when every stage is solved.
    pub stage: u8,
}

impl Run {
    /// Every stage solved: the token is the application form's pass.
    pub fn finished(&self) -> bool {
        self.stage > CTF_STAGES
    }
}

/// The secret, and the rate limits of the three endpoints.
pub(crate) struct Ctf {
    secret: [u8; 32],
    pub starts: Limiter,
    pub reads: Limiter,
    pub flags: Limiter,
}

impl Ctf {
    /// Loads `{data_dir}/ctf.key`, creating it (32 random bytes, mode 0600) on first start.
    /// Deleting it invalidates every run in progress.
    pub fn open(data_dir: &Path) -> anyhow::Result<Ctf> {
        DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(data_dir)
            .with_context(|| format!("creating {}", data_dir.display()))?;
        let path = data_dir.join("ctf.key");
        let mut secret = [0u8; 32];
        match OpenOptions::new().read(true).open(&path) {
            Ok(mut file) => {
                let mut bytes = Vec::new();
                file.read_to_end(&mut bytes).with_context(|| format!("reading {}", path.display()))?;
                secret = bytes
                    .try_into()
                    .map_err(|_| anyhow::anyhow!("{} must hold exactly 32 bytes", path.display()))?;
            }
            Err(e) if e.kind() == ErrorKind::NotFound => {
                rand::rngs::OsRng.fill_bytes(&mut secret);
                let mut file = OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .mode(0o600)
                    .open(&path)
                    .with_context(|| format!("creating {}", path.display()))?;
                file.write_all(&secret).with_context(|| format!("writing {}", path.display()))?;
                file.sync_all()?;
            }
            Err(e) => return Err(e).with_context(|| format!("opening {}", path.display())),
        }
        Ok(Ctf::with_secret(secret))
    }

    fn with_secret(secret: [u8; 32]) -> Ctf {
        Ctf {
            secret,
            // A new run per network every few minutes is plenty; reading the puzzle again is
            // cheap; answers are where guessing would happen.
            starts: Limiter::new(20.0),
            reads: Limiter::new(240.0),
            flags: Limiter::new(60.0),
        }
    }

    /// A new run at stage one.
    pub fn start(&self) -> Run {
        let mut nonce = [0u8; NONCE];
        rand::rngs::OsRng.fill_bytes(&mut nonce);
        Run { nonce, stage: 1 }
    }

    fn mac(&self, label: &[u8], nonce: &[u8; NONCE], stage: u8, extra: &[u8]) -> [u8; 32] {
        // The key is 32 bytes, which HMAC always accepts.
        let mut mac = HmacSha256::new_from_slice(&self.secret).expect("HMAC takes any key length");
        mac.update(label);
        mac.update(nonce);
        mac.update(&[stage]);
        mac.update(extra);
        mac.finalize().into_bytes().into()
    }

    /// The token that carries `run`.
    pub fn token(&self, run: &Run) -> String {
        let mut raw = [0u8; NONCE + 1 + TAG];
        raw[..NONCE].copy_from_slice(&run.nonce);
        raw[NONCE] = run.stage;
        raw[NONCE + 1..].copy_from_slice(&self.mac(b"mintaka-ctf/run/v1", &run.nonce, run.stage, &[])[..TAG]);
        format!("{TOKEN_PREFIX}{}", URL_SAFE_NO_PAD.encode(raw))
    }

    /// The run a token carries, if this server issued it.
    pub fn parse(&self, token: &str) -> Option<Run> {
        let raw = URL_SAFE_NO_PAD.decode(token.trim().strip_prefix(TOKEN_PREFIX)?).ok()?;
        let raw: [u8; NONCE + 1 + TAG] = raw.try_into().ok()?;
        let nonce: [u8; NONCE] = raw[..NONCE].try_into().ok()?;
        let stage = raw[NONCE];
        if !(1..=CTF_STAGES + 1).contains(&stage) {
            return None;
        }
        let mut mac = HmacSha256::new_from_slice(&self.secret).expect("HMAC takes any key length");
        mac.update(b"mintaka-ctf/run/v1");
        mac.update(&nonce);
        mac.update(&[stage]);
        // Constant time, on the truncated tag.
        mac.verify_truncated_left(&raw[NONCE + 1..]).ok()?;
        Some(Run { nonce, stage })
    }

    /// Whether `token` is the pass of a finished run.
    pub fn is_pass(&self, token: &str) -> bool {
        self.parse(token).is_some_and(|run| run.finished())
    }

    /// The current stage's puzzle and its answer; `None` once every stage is solved.
    fn instance(&self, run: &Run) -> Option<(CtfPuzzle, String)> {
        if run.finished() {
            return None;
        }
        let seed = |attempt: u32| self.mac(b"mintaka-ctf/puzzle/v1", &run.nonce, run.stage, &attempt.to_le_bytes());
        Some(puzzles::generate(run.stage, seed))
    }

    /// The current stage's data.
    pub fn puzzle(&self, run: &Run) -> Option<CtfPuzzle> {
        self.instance(run).map(|(puzzle, _)| puzzle)
    }

    /// Checks an answer; a right one moves the run to the next stage.
    pub fn check(&self, run: &Run, flag: &str) -> Option<Run> {
        let (_, answer) = self.instance(run)?;
        puzzles::accepts(run.stage, &answer, flag).then_some(Run { nonce: run.nonce, stage: run.stage + 1 })
    }
}

/// A token bucket per client network (IPv4 /24, IPv6 /48), refilled continuously. In memory only.
pub(crate) struct Limiter {
    per_hour: f64,
    nets: Mutex<HashMap<ClientNet, (f64, Instant)>>,
}

/// Beyond this many active networks, idle (full) buckets are dropped; past twice that, new
/// networks are refused (fail closed under a flood).
const PRUNE_AT: usize = 10_000;

impl Limiter {
    fn new(per_hour: f64) -> Limiter {
        Limiter { per_hour, nets: Mutex::new(HashMap::new()) }
    }

    /// Takes one token for `net`; `false` means "rate limited".
    pub fn admit(&self, net: ClientNet, now: Instant) -> bool {
        let capacity = self.per_hour;
        let refill = |(tokens, at): &mut (f64, Instant)| {
            let hours = now.saturating_duration_since(*at).as_secs_f64() / 3600.0;
            *tokens = (*tokens + hours * capacity).min(capacity);
            *at = now;
        };
        let mut nets = self.nets.lock().unwrap_or_else(PoisonError::into_inner);
        if nets.len() >= PRUNE_AT && !nets.contains_key(&net) {
            nets.retain(|_, bucket| {
                refill(bucket);
                bucket.0 < capacity
            });
            if nets.len() >= 2 * PRUNE_AT {
                return false;
            }
        }
        let bucket = nets.entry(net).or_insert((capacity, now));
        refill(bucket);
        if bucket.0 >= 1.0 {
            bucket.0 -= 1.0;
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn ctf() -> Ctf {
        Ctf::with_secret([7; 32])
    }

    #[test]
    fn tokens_round_trip_and_forgeries_fail() {
        let c = ctf();
        let run = c.start();
        let token = c.token(&run);
        assert!(token.starts_with("r1."));
        assert_eq!(c.parse(&token), Some(run));
        assert_eq!(c.parse(&format!(" {token}\n")), Some(run));
        // Another secret, a raised stage, a flipped bit, garbage: all refused.
        assert_eq!(Ctf::with_secret([8; 32]).parse(&token), None);
        let mut raw = URL_SAFE_NO_PAD.decode(&token[3..]).unwrap();
        raw[NONCE] = 5;
        assert_eq!(c.parse(&format!("r1.{}", URL_SAFE_NO_PAD.encode(&raw))), None);
        raw[NONCE] = 1;
        raw[0] ^= 1;
        assert_eq!(c.parse(&format!("r1.{}", URL_SAFE_NO_PAD.encode(&raw))), None);
        for bad in ["", "r1.", "r2.AAAA", "r1.!!!!", &token[..token.len() - 2]] {
            assert_eq!(c.parse(bad), None, "{bad:?}");
        }
    }

    #[test]
    fn a_run_replays_and_runs_differ() {
        let c = ctf();
        let a = c.start();
        let b = c.start();
        assert_eq!(c.puzzle(&a), c.puzzle(&a));
        assert_ne!(c.puzzle(&a), c.puzzle(&b));
    }

    #[test]
    fn the_four_stages_lead_to_a_pass() {
        let c = ctf();
        let mut run = c.start();
        assert!(!c.is_pass(&c.token(&run)));
        for stage in 1..=CTF_STAGES {
            assert_eq!(run.stage, stage);
            let (_, answer) = c.instance(&run).unwrap();
            assert_eq!(c.check(&run, "0"), None, "stage {stage} took a wrong answer");
            run = c.check(&run, &answer).unwrap_or_else(|| panic!("stage {stage} refused its own answer {answer:?}"));
        }
        assert!(run.finished());
        assert_eq!(c.puzzle(&run), None);
        assert_eq!(c.check(&run, "anything"), None);
        assert!(c.is_pass(&c.token(&run)));
    }

    #[test]
    fn the_key_is_created_once_and_private() {
        let dir = std::env::temp_dir().join(format!("mintaka-ctf-{}-{}", std::process::id(), rand::random::<u32>()));
        let first = Ctf::open(&dir).unwrap();
        let mode = std::os::unix::fs::PermissionsExt::mode(&std::fs::metadata(dir.join("ctf.key")).unwrap().permissions());
        assert_eq!(mode & 0o777, 0o600);
        let again = Ctf::open(&dir).unwrap();
        let run = first.start();
        assert_eq!(again.parse(&first.token(&run)), Some(run), "same key after a restart");
        std::fs::write(dir.join("ctf.key"), b"short").unwrap();
        assert!(Ctf::open(&dir).is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn limiter_refills_per_network() {
        let l = Limiter::new(60.0);
        let net = ClientNet::of("192.0.2.1".parse().unwrap());
        let t0 = Instant::now();
        assert!((0..60).all(|_| l.admit(net, t0)));
        assert!(!l.admit(net, t0));
        assert!(l.admit(ClientNet::of("192.0.3.1".parse().unwrap()), t0), "other networks unaffected");
        assert!(l.admit(net, t0 + Duration::from_secs(61)), "one token a minute");
        assert!(!l.admit(net, t0 + Duration::from_secs(62)));
    }
}
