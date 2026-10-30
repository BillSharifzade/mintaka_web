//! Start codes (docs/CONTRACTS.md §19). `https://t.me/<bot>?start=<code>` opens a chat with the
//! bot that sends `/start <code>`. The code names one brief — its id and language — and carries a
//! MAC made with `{data-dir}/telegram.key`, so the bot recognises it without the server keeping
//! anything per brief, like the console challenge's run tokens. Telegram allows 1–64 characters of
//! `A–Z a–z 0–9 _ -`: base64url is exactly that alphabet.

use base64::Engine as _;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use hmac::{Hmac, Mac};
use rand::RngCore;
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

/// `b1` + base64url(`ms[6] ‖ random[8] ‖ len[1] ‖ lang[len] ‖ tag[12]`): the brief id's two parts
/// (`{ms}-{16 hex digits}`), its language, and the first 12 bytes of
/// HMAC-SHA256(key, label ‖ everything before the tag).
const PREFIX: &str = "b1";
const TAG: usize = 12;
const LABEL: &[u8] = b"mintaka-bot/start/v1";
/// A honeypot submission's link (§8.2: indistinguishable from success): shaped like any other,
/// MAC'd under another label, so the bot treats it like a forgery.
const DECOY: &[u8] = b"mintaka-bot/decoy/v1";
/// How long after the brief its link still connects a chat.
pub(crate) const VALID_MS: u64 = 7 * 24 * 3600 * 1000;
/// Language tags as the server accepts them: letters, digits, `-`.
const MAX_LANG: usize = 16;

/// The key, and what it makes.
pub(crate) struct Links {
    key: [u8; 32],
}

/// The brief a valid code names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Brief {
    pub id: String,
    pub lang: String,
}

/// What a `/start` payload turned out to be.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Code {
    Brief(Brief),
    /// Ours, but older than [`VALID_MS`]: the language still says how to answer.
    Expired { lang: String },
    /// Not ours: forged, damaged, from another key, or a decoy.
    Unknown,
}

impl Links {
    pub fn new(key: [u8; 32]) -> Links {
        Links { key }
    }

    fn tag(&self, label: &[u8], body: &[u8]) -> HmacSha256 {
        // The key is 32 bytes, which HMAC always accepts.
        let mut mac = HmacSha256::new_from_slice(&self.key).expect("HMAC takes any key length");
        mac.update(label);
        mac.update(body);
        mac
    }

    fn encode(&self, label: &[u8], ms: u64, random: &[u8; 8], lang: &str) -> Option<String> {
        let lang_ok = (1..=MAX_LANG).contains(&lang.len()) && lang.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-');
        if !lang_ok || ms >= 1 << 48 {
            return None;
        }
        let mut raw = ms.to_be_bytes()[2..].to_vec();
        raw.extend_from_slice(random);
        raw.push(lang.len() as u8);
        raw.extend_from_slice(lang.as_bytes());
        let tag = self.tag(label, &raw).finalize().into_bytes();
        raw.extend_from_slice(&tag[..TAG]);
        Some(format!("{PREFIX}{}", URL_SAFE_NO_PAD.encode(raw)))
    }

    /// The code for brief `id` (`{unix-ms}-{16 hex digits}`, as `contact::new_id` makes them),
    /// written in `lang`.
    pub fn code(&self, id: &str, lang: &str) -> Option<String> {
        let (ms, hex) = id.split_once('-')?;
        let ms: u64 = ms.parse().ok()?;
        if hex.len() != 16 || !hex.bytes().all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase()) {
            return None;
        }
        let mut random = [0u8; 8];
        for (i, byte) in random.iter_mut().enumerate() {
            *byte = u8::from_str_radix(&hex[2 * i..2 * i + 2], 16).ok()?;
        }
        self.encode(LABEL, ms, &random, lang)
    }

    /// A code for a brief that doesn't exist, which no `/start` accepts.
    pub fn decoy(&self, lang: &str, now_ms: u64) -> Option<String> {
        let mut random = [0u8; 8];
        rand::thread_rng().fill_bytes(&mut random);
        self.encode(DECOY, now_ms, &random, lang)
    }

    /// Reads a `/start` payload.
    pub fn parse(&self, code: &str, now_ms: u64) -> Code {
        self.read(code, now_ms).unwrap_or(Code::Unknown)
    }

    fn read(&self, code: &str, now_ms: u64) -> Option<Code> {
        let raw = URL_SAFE_NO_PAD.decode(code.trim().strip_prefix(PREFIX)?).ok()?;
        let len = usize::from(*raw.get(14)?);
        if !(1..=MAX_LANG).contains(&len) || raw.len() != 15 + len + TAG {
            return None;
        }
        let (body, tag) = raw.split_at(15 + len);
        // Constant time, on the truncated tag.
        self.tag(LABEL, body).verify_truncated_left(tag).ok()?;
        let lang = std::str::from_utf8(&body[15..]).ok()?.to_owned();
        let mut ms = [0u8; 8];
        ms[2..].copy_from_slice(&body[..6]);
        let ms = u64::from_be_bytes(ms);
        if now_ms.saturating_sub(ms) > VALID_MS {
            return Some(Code::Expired { lang });
        }
        let hex: String = body[6..14].iter().map(|b| format!("{b:02x}")).collect();
        Some(Code::Brief(Brief { id: format!("{ms}-{hex}"), lang }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ID: &str = "1793102400123-a1b2c3d4e5f6a7b8";
    const MS: u64 = 1_793_102_400_123;

    fn links() -> Links {
        Links::new([7; 32])
    }

    #[test]
    fn a_code_names_its_brief_and_fits_telegram() {
        let code = links().code(ID, "en").unwrap();
        assert!(code.starts_with("b1") && code.len() <= 64, "{code}");
        assert!(code.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-'), "{code}");
        assert_eq!(links().parse(&code, MS + 1000), Code::Brief(Brief { id: ID.into(), lang: "en".into() }));
        // The longest language tag still fits.
        let long = links().code(ID, "pt-BR-x-mintaka1").unwrap();
        assert!(long.len() <= 64, "{} characters", long.len());
        assert_eq!(links().parse(&long, MS), Code::Brief(Brief { id: ID.into(), lang: "pt-BR-x-mintaka1".into() }));
    }

    #[test]
    fn only_this_key_and_these_bytes() {
        let code = links().code(ID, "ru").unwrap();
        assert_eq!(Links::new([8; 32]).parse(&code, MS), Code::Unknown, "another key");
        // Every single changed character breaks it.
        for i in 2..code.len() {
            let mut bytes = code.clone().into_bytes();
            bytes[i] = if bytes[i] == b'A' { b'B' } else { b'A' };
            let changed = String::from_utf8(bytes).unwrap();
            assert!(matches!(links().parse(&changed, MS), Code::Unknown), "{changed}");
        }
        for junk in ["", "b1", "b1!!!", "start", &code[..code.len() - 1], &format!("{code}A"), &code[2..]] {
            assert_eq!(links().parse(junk, MS), Code::Unknown, "{junk}");
        }
    }

    #[test]
    fn a_week_then_expired() {
        let code = links().code(ID, "tg").unwrap();
        assert!(matches!(links().parse(&code, MS + VALID_MS), Code::Brief(_)));
        assert_eq!(links().parse(&code, MS + VALID_MS + 1), Code::Expired { lang: "tg".into() });
        assert!(matches!(links().parse(&code, MS - 60_000), Code::Brief(_)), "a clock a little behind");
    }

    #[test]
    fn decoys_look_alike_and_open_nothing() {
        let decoy = links().decoy("en", MS).unwrap();
        let real = links().code(ID, "en").unwrap();
        assert_eq!(decoy.len(), real.len());
        assert_eq!(links().parse(&decoy, MS), Code::Unknown);
    }

    #[test]
    fn ids_and_languages_it_cannot_carry() {
        for id in ["1793102400123", "x-a1b2c3d4e5f6a7b8", "1793102400123-a1b2", "1793102400123-A1B2C3D4E5F6A7B8", "281474976710656-a1b2c3d4e5f6a7b8"] {
            assert_eq!(links().code(id, "en"), None, "{id}");
        }
        for lang in ["", "e n", "日本", "a-very-long-language-tag"] {
            assert_eq!(links().code(ID, lang), None, "{lang:?}");
        }
    }
}
