//! Contact form fields as types (docs/CONTRACTS.md §8.2). Parse, don't validate: a value of one
//! of these types is valid by construction, so the storage code cannot see a bad brief.

use proof::{Budget, ContactRequest, ErrorCode, Field, FieldError, Kind, Timeline};

/// Max characters (Unicode scalar values) per field.
const NAME_MAX: usize = 120;
const COMPANY_MAX: usize = 120;
const MESSAGE_MAX: usize = 5000;
const EMAIL_MAX: usize = 254;
const EMAIL_LOCAL_MAX: usize = 64;

/// A single-line value: runs of whitespace and control characters become one space, trimmed.
fn one_line(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    for word in raw.split(|c: char| c.is_whitespace() || c.is_control()).filter(|w| !w.is_empty()) {
        if !out.is_empty() {
            out.push(' ');
        }
        out.push_str(word);
    }
    out
}

/// `None` for absent or blank input.
fn present(raw: Option<&str>) -> Option<&str> {
    raw.map(str::trim).filter(|s| !s.is_empty())
}

/// The person's name: 1–120 characters after trimming.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Name(String);

impl Name {
    pub fn parse(raw: Option<&str>) -> Result<Name, ErrorCode> {
        let name = one_line(raw.unwrap_or(""));
        match name.chars().count() {
            0 => Err(ErrorCode::NameRequired),
            n if n > NAME_MAX => Err(ErrorCode::NameTooLong),
            _ => Ok(Name(name)),
        }
    }

    pub fn into_inner(self) -> String {
        self.0
    }
}

/// A syntactically valid address of at most 254 characters: the WHATWG `type=email` grammar
/// (what the browser enforces on the no-JS form), plus a dot in the domain.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Email(String);

impl Email {
    pub fn parse(raw: &str) -> Result<Email, ErrorCode> {
        let email = raw.trim();
        let (local, domain) = email.rsplit_once('@').ok_or(ErrorCode::EmailInvalid)?;
        let local_ok = !local.is_empty()
            && local.len() <= EMAIL_LOCAL_MAX
            && local.bytes().all(|b| b.is_ascii_alphanumeric() || b".!#$%&'*+/=?^_`{|}~-".contains(&b));
        let labels: Vec<&str> = domain.split('.').collect();
        let domain_ok = labels.len() >= 2
            && labels.iter().all(|label| {
                (1..=63).contains(&label.len())
                    && label.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
                    && !label.starts_with('-')
                    && !label.ends_with('-')
            });
        if email.len() <= EMAIL_MAX && local_ok && domain_ok {
            Ok(Email(email.to_owned()))
        } else {
            Err(ErrorCode::EmailInvalid)
        }
    }

    pub fn into_inner(self) -> String {
        self.0
    }
}

/// A Telegram username, `@?[A-Za-z][A-Za-z0-9_]{4,31}`, stored without the `@`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TelegramHandle(String);

impl TelegramHandle {
    pub fn parse(raw: &str) -> Result<TelegramHandle, ErrorCode> {
        let handle = raw.trim();
        let handle = handle.strip_prefix('@').unwrap_or(handle);
        let mut bytes = handle.bytes();
        let first_ok = bytes.next().is_some_and(|b| b.is_ascii_alphabetic());
        let rest_ok = bytes.all(|b| b.is_ascii_alphanumeric() || b == b'_');
        if first_ok && rest_ok && (5..=32).contains(&handle.len()) {
            Ok(TelegramHandle(handle.to_owned()))
        } else {
            Err(ErrorCode::TelegramInvalid)
        }
    }

    pub fn into_inner(self) -> String {
        self.0
    }
}

/// The free-text message: at most 5000 characters. Line breaks are normalised to `\n` first
/// (browsers submit a textarea's newlines as CRLF but count them as one character for
/// `maxlength`); other control characters except tab are dropped.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Message(String);

impl Message {
    pub fn parse(raw: &str) -> Result<Message, ErrorCode> {
        let normalised = raw.replace("\r\n", "\n").replace('\r', "\n");
        let message: String =
            normalised.trim().chars().filter(|&c| c == '\n' || c == '\t' || !c.is_control()).collect();
        if message.chars().count() > MESSAGE_MAX {
            Err(ErrorCode::MessageTooLong)
        } else {
            Ok(Message(message))
        }
    }

    pub fn into_inner(self) -> String {
        self.0
    }
}

/// The company: at most 120 characters, one line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Company(String);

impl Company {
    pub fn parse(raw: &str) -> Result<Company, ErrorCode> {
        let company = one_line(raw);
        if company.chars().count() > COMPANY_MAX {
            Err(ErrorCode::CompanyTooLong)
        } else {
            Ok(Company(company))
        }
    }

    pub fn into_inner(self) -> String {
        self.0
    }
}

/// A complete, valid brief.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Brief {
    pub kinds: Vec<Kind>,
    pub message: Option<Message>,
    pub timeline: Option<Timeline>,
    pub budget: Option<Budget>,
    pub name: Name,
    pub email: Option<Email>,
    pub telegram: Option<TelegramHandle>,
    pub company: Option<Company>,
}

impl Brief {
    /// All fields at once; errors come back in form order (the no-JS flow reports the first).
    pub fn parse(req: &ContactRequest) -> Result<Brief, Vec<FieldError>> {
        let mut errors = Vec::new();
        let e = &mut errors;

        let mut kinds = Vec::new();
        let mut kinds_ok = true;
        for raw in req.kind.iter().flatten() {
            match raw.trim().parse::<Kind>() {
                Ok(kind) if !kinds.contains(&kind) => kinds.push(kind),
                Ok(_) => {}
                Err(_) => kinds_ok = false,
            }
        }
        let kinds = check(e, Field::Kind, if kinds_ok { Ok(kinds) } else { Err(ErrorCode::ChoiceInvalid) });
        let message = present(req.message.as_deref()).and_then(|m| check(e, Field::Message, Message::parse(m)));
        let timeline = present(req.timeline.as_deref()).and_then(|t| check(e, Field::Timeline, choice(t)));
        let budget = present(req.budget.as_deref()).and_then(|b| check(e, Field::Budget, choice(b)));
        let name = check(e, Field::Name, Name::parse(req.name.as_deref()));
        let email_raw = present(req.email.as_deref());
        let email = email_raw.and_then(|v| check(e, Field::Email, Email::parse(v)));
        let telegram_raw = present(req.telegram.as_deref());
        let telegram = telegram_raw.and_then(|v| check(e, Field::Telegram, TelegramHandle::parse(v)));
        if email_raw.is_none() && telegram_raw.is_none() {
            e.push(FieldError::new(Field::Contact, ErrorCode::ContactRequired));
        }
        let company = present(req.company.as_deref())
            .and_then(|c| check(e, Field::Company, Company::parse(c)))
            .filter(|c| !c.0.is_empty());

        match (kinds, name) {
            (Some(kinds), Some(name)) if errors.is_empty() => {
                Ok(Brief { kinds, message, timeline, budget, name, email, telegram, company })
            }
            _ => Err(errors),
        }
    }
}

/// Records a field's error and turns it into `None`.
fn check<T>(errors: &mut Vec<FieldError>, field: Field, result: Result<T, ErrorCode>) -> Option<T> {
    result.map_err(|code| errors.push(FieldError::new(field, code))).ok()
}

/// One value of a closed vocabulary.
fn choice<T: std::str::FromStr>(raw: &str) -> Result<T, ErrorCode> {
    raw.parse().map_err(|_| ErrorCode::ChoiceInvalid)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn codes(req: &ContactRequest) -> Vec<&'static str> {
        Brief::parse(req).err().unwrap_or_default().iter().map(|e| e.code.as_str()).collect()
    }

    fn valid() -> ContactRequest {
        ContactRequest {
            name: Some("  Bill  ".into()),
            email: Some("bill@mintaka.tj".into()),
            kind: Some(vec!["ai".into(), "ai".into(), "performance".into()]),
            timeline: Some("1-3-months".into()),
            ..Default::default()
        }
    }

    #[test]
    fn parses_a_valid_brief() {
        let brief = Brief::parse(&valid()).unwrap();
        assert_eq!(brief.name, Name("Bill".into()));
        assert_eq!(brief.kinds, [Kind::Ai, Kind::Performance]);
        assert_eq!(brief.timeline, Some(Timeline::OneToThreeMonths));
        assert_eq!(brief.budget, None);
    }

    #[test]
    fn reports_every_error_in_form_order() {
        let req = ContactRequest {
            kind: Some(vec!["rockets".into()]),
            message: Some("x".repeat(5001)),
            timeline: Some("tomorrow".into()),
            name: Some("x".repeat(121)),
            email: Some("not-an-email".into()),
            telegram: Some("@abc".into()),
            company: Some("c".repeat(121)),
            ..Default::default()
        };
        assert_eq!(
            codes(&req),
            [
                "choice-invalid",
                "message-too-long",
                "choice-invalid",
                "name-too-long",
                "email-invalid",
                "telegram-invalid",
                "company-too-long"
            ]
        );
        assert_eq!(codes(&ContactRequest::default()), ["name-required", "contact-required"]);
    }

    #[test]
    fn names() {
        assert_eq!(Name::parse(Some(" a\n\tb  ")), Ok(Name("a b".into())));
        assert_eq!(Name::parse(Some("   ")), Err(ErrorCode::NameRequired));
        assert!(Name::parse(Some(&"ж".repeat(120))).is_ok(), "characters, not bytes");
    }

    #[test]
    fn emails() {
        for ok in ["a@b.tj", "first.last+tag@sub.example.com", "x_y@a-b.io"] {
            assert!(Email::parse(ok).is_ok(), "{ok}");
        }
        for bad in ["", "a", "a@b", "@b.tj", "a@.tj", "a@b..tj", "a@-b.tj", "a b@c.tj", "a@b_c.tj", "ä@b.tj"] {
            assert_eq!(Email::parse(bad), Err(ErrorCode::EmailInvalid), "{bad}");
        }
        assert!(Email::parse(&format!("{}@{}.tj", "a".repeat(64), "b".repeat(63))).is_ok());
        assert!(Email::parse(&format!("{}@b.tj", "a".repeat(65))).is_err());
    }

    #[test]
    fn telegram_handles() {
        assert_eq!(TelegramHandle::parse("@mintaka_tj"), Ok(TelegramHandle("mintaka_tj".into())));
        assert!(TelegramHandle::parse("mintaka").is_ok());
        for bad in ["@abcd", "1mintaka", "@min-taka", &"a".repeat(33), "@"] {
            assert_eq!(TelegramHandle::parse(bad), Err(ErrorCode::TelegramInvalid), "{bad}");
        }
    }

    #[test]
    fn messages_count_crlf_as_one() {
        let text = "a\r\n".repeat(2500);
        let msg = Message::parse(&text).unwrap();
        assert_eq!(msg.0.chars().count(), 2 * 2500 - 1, "trimmed trailing newline");
        assert_eq!(Message::parse(&"é".repeat(5001)), Err(ErrorCode::MessageTooLong));
        assert_eq!(Message::parse("a\u{0}b"), Ok(Message("ab".into())));
    }

    #[test]
    fn telegram_alone_is_enough() {
        let req = ContactRequest { name: Some("Bill".into()), telegram: Some("@mintaka_tj".into()), ..Default::default() };
        assert!(Brief::parse(&req).is_ok());
    }
}
