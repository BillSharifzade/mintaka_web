//! The contact form wire format: `GET /api/pow` and `POST /api/contact` (docs/CONTRACTS.md §8).
//!
//! These are the *unparsed* shapes. The server turns a [`ContactRequest`] into validated newtypes
//! (`Name`, `Email`, `TelegramHandle`, …) or a list of [`FieldError`]s; the closed vocabularies
//! ([`Kind`], [`Timeline`], [`Budget`], [`ErrorCode`], [`Field`]) live here so the site generator
//! can render the same values (chip values, `id="e-{code}"` targets on the `/{lang}/sent/` page).

use serde::{Deserialize, Serialize};

/// Raw length of a decoded proof-of-work challenge:
/// `nonce[16] ‖ issued_at[8, BE] ‖ difficulty[1] ‖ mac[16]`.
pub const POW_CHALLENGE_BYTES: usize = 41;

/// How long a challenge stays valid after it was issued, in seconds.
pub const POW_TTL_SECS: u64 = 600;

/// Largest accepted `POST /api/contact` body.
pub const CONTACT_BODY_LIMIT: usize = 32 * 1024;

/// `GET /api/pow` response.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PowChallenge {
    /// base64url without padding of [`POW_CHALLENGE_BYTES`] bytes.
    pub challenge: String,
    /// Required leading zero bits of `SHA-256(challenge_bytes ‖ counter[8, LE])`.
    pub difficulty: u8,
    /// Unix seconds after which the server answers `pow-expired`.
    pub expires_at: u64,
}

/// `POST /api/contact` body as JSON (the enhanced form). The no-JS form posts the same field
/// names as `application/x-www-form-urlencoded`, with `kind` repeated.
///
/// Every field is optional at this level: presence and shape are checked by the server, which
/// answers with [`FieldError`]s rather than a parse failure.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct ContactRequest {
    /// Zero or more [`Kind`] values.
    pub kind: Option<Vec<String>>,
    pub message: Option<String>,
    pub timeline: Option<String>,
    pub budget: Option<String>,
    pub name: Option<String>,
    pub email: Option<String>,
    pub telegram: Option<String>,
    pub company: Option<String>,
    pub lang: Option<String>,
    /// Honeypot: must be empty.
    pub website: Option<String>,
    pub pow_challenge: Option<String>,
    pub pow_counter: Option<u64>,
}

/// `POST /api/contact` JSON response: `{"ok":true}` or `{"ok":false,"errors":[…]}`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ContactResponse {
    pub ok: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub errors: Vec<FieldError>,
}

impl ContactResponse {
    /// `{"ok":true}`.
    pub fn ok() -> Self {
        Self { ok: true, errors: Vec::new() }
    }

    /// `{"ok":false,"errors":[…]}`.
    pub fn errors(errors: Vec<FieldError>) -> Self {
        Self { ok: false, errors }
    }
}

/// One problem with a submission.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct FieldError {
    pub field: Field,
    pub code: ErrorCode,
}

impl FieldError {
    pub fn new(field: Field, code: ErrorCode) -> Self {
        Self { field, code }
    }
}

/// A string that is not one of a closed vocabulary's wire values ([`Kind`], [`ErrorCode`], …).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnknownChoice;

impl core::fmt::Display for UnknownChoice {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("not one of the allowed values")
    }
}

impl std::error::Error for UnknownChoice {}

/// A closed vocabulary: an enum with a fixed wire string per variant, `ALL`, `as_str`, `FromStr`.
macro_rules! choice {
    ($(#[$meta:meta])* $name:ident { $($(#[$vmeta:meta])* $variant:ident = $wire:literal),+ $(,)? }) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
        pub enum $name {
            $($(#[$vmeta])* #[serde(rename = $wire)] $variant),+
        }

        impl $name {
            /// Every value, in declaration order.
            pub const ALL: &'static [$name] = &[$($name::$variant),+];

            /// The wire string.
            pub const fn as_str(self) -> &'static str {
                match self {
                    $($name::$variant => $wire),+
                }
            }
        }

        impl core::str::FromStr for $name {
            type Err = UnknownChoice;

            fn from_str(s: &str) -> Result<Self, UnknownChoice> {
                match s {
                    $($wire => Ok($name::$variant),)+
                    _ => Err(UnknownChoice),
                }
            }
        }

        impl core::fmt::Display for $name {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                f.write_str(self.as_str())
            }
        }
    };
}

choice! {
    /// "What are you building?" chips.
    Kind {
        NewProduct = "new-product",
        Performance = "performance",
        SecurityAudit = "security-audit",
        Design = "design",
        Ai = "ai",
        NotSure = "not-sure",
    }
}

choice! {
    /// Timeline chips.
    Timeline {
        Asap = "asap",
        OneToThreeMonths = "1-3-months",
        Later = "later",
        Exploring = "exploring",
    }
}

choice! {
    /// Budget chips (labels are page copy; ranges still to be set by Bill).
    Budget {
        Undecided = "undecided",
        Below10k = "lt-10k",
        From10To50k = "10-50k",
        From50To150k = "50-150k",
        Above150k = "gt-150k",
    }
}

choice! {
    /// Which part of the form an error belongs to. `form` means the submission as a whole
    /// (status line); `contact` means "email or Telegram".
    Field {
        Form = "form",
        Kind = "kind",
        Message = "message",
        Timeline = "timeline",
        Budget = "budget",
        Name = "name",
        Email = "email",
        Telegram = "telegram",
        Contact = "contact",
        Company = "company",
        Pow = "pow",
    }
}

choice! {
    /// Error codes, kebab-case. The no-JS flow redirects to `/{lang}/sent/#e-{code}`, so that page
    /// needs an element with `id="e-{code}"` for every value in [`ErrorCode::ALL`].
    ErrorCode {
        NameRequired = "name-required",
        NameTooLong = "name-too-long",
        EmailInvalid = "email-invalid",
        TelegramInvalid = "telegram-invalid",
        ContactRequired = "contact-required",
        MessageTooLong = "message-too-long",
        CompanyTooLong = "company-too-long",
        ChoiceInvalid = "choice-invalid",
        PowInvalid = "pow-invalid",
        PowExpired = "pow-expired",
        RateLimited = "rate-limited",
        TooLarge = "too-large",
        ServerError = "server-error",
    }
}

/// One accepted brief, as stored (encrypted with `age`) in `{data-dir}/contact/{id}.json.age`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StoredBrief {
    /// `{unix-ms}-{random hex}`, also the file name stem.
    pub id: String,
    pub received_at_unix_ms: u64,
    pub lang: String,
    pub kinds: Vec<Kind>,
    pub message: Option<String>,
    pub timeline: Option<Timeline>,
    pub budget: Option<Budget>,
    pub name: String,
    pub email: Option<String>,
    /// Without the leading `@`.
    pub telegram: Option<String>,
    pub company: Option<String>,
    /// Whether the submission came with a valid proof of work (JS path) or not (no-JS path).
    pub proof_of_work: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wire_strings_are_kebab_case() {
        assert_eq!(serde_json::to_string(&Timeline::OneToThreeMonths).unwrap(), r#""1-3-months""#);
        assert_eq!(serde_json::to_string(&Budget::Below10k).unwrap(), r#""lt-10k""#);
        assert_eq!("security-audit".parse::<Kind>(), Ok(Kind::SecurityAudit));
        assert_eq!("Security-Audit".parse::<Kind>(), Err(UnknownChoice));
        assert_eq!(ErrorCode::ALL.len(), 13);
        for code in ErrorCode::ALL {
            assert_eq!(code.as_str().parse::<ErrorCode>(), Ok(*code));
        }
    }

    #[test]
    fn responses_match_the_contract() {
        assert_eq!(serde_json::to_string(&ContactResponse::ok()).unwrap(), r#"{"ok":true}"#);
        let err = ContactResponse::errors(vec![FieldError::new(Field::Email, ErrorCode::EmailInvalid)]);
        assert_eq!(
            serde_json::to_string(&err).unwrap(),
            r#"{"ok":false,"errors":[{"field":"email","code":"email-invalid"}]}"#
        );
    }

    #[test]
    fn request_fields_are_all_optional() {
        let r: ContactRequest = serde_json::from_str(r#"{"name":"Bill","kind":["ai"],"budget":null}"#).unwrap();
        assert_eq!(r.name.as_deref(), Some("Bill"));
        assert_eq!(r.kind, Some(vec!["ai".to_string()]));
        assert_eq!(r.budget, None);
        assert_eq!(r.pow_counter, None);
    }
}
