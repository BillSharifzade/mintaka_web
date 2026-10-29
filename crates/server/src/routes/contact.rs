//! `GET /api/pow` and `POST /api/contact` (docs/CONTRACTS.md §8).
//!
//! Order of checks for a submission: method and size → parse → honeypot (answers exactly like
//! success, stores nothing) → field types → rate limit → proof of work → encrypt and store →
//! notify. JSON requests get JSON answers; form posts get `303` to `/{lang}/sent/[#e-{code}]`.

use std::sync::Arc;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use axum::body::Body;
use axum::extract::{Request, State};
use axum::response::Response;
use http::header::{CACHE_CONTROL, CONTENT_LENGTH, CONTENT_TYPE, LOCATION};
use http::{HeaderValue, StatusCode};
use http_body_util::{BodyExt, LengthLimitError, Limited};
use proof::{
    CONTACT_BODY_LIMIT, ContactRequest, ContactResponse, ErrorCode, Field, FieldError, StoredBrief,
};

use super::{GET_HEAD, RouteName, ctx, internal_error, is_read, json, mark, method_not_allowed};
use crate::app::AppState;
use crate::contact::{Brief, new_id};
use crate::pow::PowError;

fn unix_now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs())
}

/// `GET /api/pow`: a fresh challenge.
pub(crate) async fn pow(State(app): State<Arc<AppState>>, req: Request) -> Response {
    let routed = Instant::now();
    let response = if !is_read(req.method()) {
        method_not_allowed(GET_HEAD)
    } else {
        app.pow.issue(unix_now()).map_or_else(internal_error, |c| json(StatusCode::OK, &c))
    };
    mark(response, RouteName::Named("/api/pow"), routed)
}

/// How the client wants to be answered.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Flavor {
    Json,
    Form,
}

/// `POST /api/contact`.
pub(crate) async fn submit(State(app): State<Arc<AppState>>, req: Request) -> Response {
    let routed = Instant::now();
    let response = if req.method() == http::Method::POST {
        receive(&app, req).await
    } else {
        method_not_allowed("POST")
    };
    mark(response, RouteName::Named("/api/contact"), routed)
}

async fn receive(app: &AppState, req: Request) -> Response {
    let default_lang = app.site.default_lang.as_str();
    let Some(ctx) = ctx(&req) else { return internal_error() };
    let content_type = req.headers().get(CONTENT_TYPE).and_then(|v| v.to_str().ok()).unwrap_or("");
    let mime = content_type.split(';').next().unwrap_or("").trim().to_ascii_lowercase();
    let flavor = match mime.as_str() {
        "application/json" => Flavor::Json,
        "application/x-www-form-urlencoded" => Flavor::Form,
        _ => {
            let body = ContactResponse::errors(vec![FieldError::new(Field::Form, ErrorCode::ServerError)]);
            return json(StatusCode::UNSUPPORTED_MEDIA_TYPE, &body);
        }
    };
    let declared = req.headers().get(CONTENT_LENGTH).and_then(|v| v.to_str().ok()?.parse::<u64>().ok());
    if declared.is_some_and(|n| n > CONTACT_BODY_LIMIT as u64) {
        return refuse(flavor, default_lang, StatusCode::PAYLOAD_TOO_LARGE, Field::Form, ErrorCode::TooLarge);
    }
    let body = match Limited::new(req.into_body(), CONTACT_BODY_LIMIT).collect().await {
        Ok(collected) => collected.to_bytes(),
        Err(e) if e.downcast_ref::<LengthLimitError>().is_some() => {
            return refuse(flavor, default_lang, StatusCode::PAYLOAD_TOO_LARGE, Field::Form, ErrorCode::TooLarge);
        }
        Err(_) => return refuse(flavor, default_lang, StatusCode::BAD_REQUEST, Field::Form, ErrorCode::ServerError),
    };

    let (raw, pow_malformed) = match flavor {
        Flavor::Json => match serde_json::from_slice::<ContactRequest>(&body) {
            Ok(raw) => (raw, false),
            Err(_) => return refuse(flavor, default_lang, StatusCode::BAD_REQUEST, Field::Form, ErrorCode::ServerError),
        },
        Flavor::Form => parse_form(&body),
    };
    let lang = raw.lang.as_deref().and_then(|l| app.site.lang(l.trim())).unwrap_or(default_lang);

    // Honeypot: indistinguishable from success, nothing stored.
    if raw.website.as_deref().is_some_and(|w| !w.trim().is_empty()) {
        tracing::info!(target: "contact", "honeypot filled; answered as success, stored nothing");
        return accepted(flavor, lang);
    }

    let brief = match Brief::parse(&raw) {
        Ok(brief) => brief,
        Err(errors) => return rejected(flavor, lang, StatusCode::UNPROCESSABLE_ENTITY, errors),
    };

    let with_pow = pow_malformed || raw.pow_challenge.is_some() || raw.pow_counter.is_some();
    if !app.contact.limiter.admit(ctx.conn.client_net, with_pow, Instant::now()) {
        return refuse(flavor, lang, StatusCode::TOO_MANY_REQUESTS, Field::Form, ErrorCode::RateLimited);
    }
    if with_pow {
        let verdict = match (&raw.pow_challenge, raw.pow_counter) {
            (Some(challenge), Some(counter)) if !pow_malformed => app.pow.verify(challenge, counter, unix_now()),
            _ => Err(PowError::Invalid),
        };
        if let Err(e) = verdict {
            let code = if e == PowError::Expired { ErrorCode::PowExpired } else { ErrorCode::PowInvalid };
            return refuse(flavor, lang, StatusCode::UNPROCESSABLE_ENTITY, Field::Pow, code);
        }
    }

    // An application from someone who finished the console challenge. A bad pass is ignored, not
    // refused: the brief still reaches us, just without the mark.
    let ctf_solved = raw.ctf_pass.as_deref().is_some_and(|pass| app.ctf.is_pass(pass));
    let (id, received_at_unix_ms) = new_id();
    let kinds = brief.kinds.clone();
    let timeline = brief.timeline;
    let record = StoredBrief {
        id: id.clone(),
        received_at_unix_ms,
        lang: lang.to_owned(),
        kinds: brief.kinds,
        message: brief.message.map(|m| m.into_inner()),
        timeline: brief.timeline,
        budget: brief.budget,
        name: brief.name.into_inner(),
        email: brief.email.map(|e| e.into_inner()),
        telegram: brief.telegram.map(|t| t.into_inner()),
        company: brief.company.map(|c| c.into_inner()),
        proof_of_work: with_pow,
        ctf_solved,
    };
    let store = Arc::clone(&app.contact.store);
    let saved = tokio::task::spawn_blocking(move || store.save(&record)).await;
    match saved {
        Ok(Ok(_)) => {
            tracing::info!(target: "contact", id = %id, "brief stored");
            if let Some(notifier) = &app.contact.notifier {
                notifier.brief_received(&id, &kinds, timeline, ctf_solved);
            }
            accepted(flavor, lang)
        }
        Ok(Err(e)) => {
            tracing::error!(target: "contact", "storing a brief failed: {e}");
            refuse(flavor, lang, StatusCode::INTERNAL_SERVER_ERROR, Field::Form, ErrorCode::ServerError)
        }
        Err(e) => {
            tracing::error!(target: "contact", "storing a brief panicked: {e}");
            refuse(flavor, lang, StatusCode::INTERNAL_SERVER_ERROR, Field::Form, ErrorCode::ServerError)
        }
    }
}

/// `application/x-www-form-urlencoded` → the same shape as the JSON body (`kind` repeated).
/// The second value is true if `pow_counter` was present but not a number.
fn parse_form(body: &[u8]) -> (ContactRequest, bool) {
    let mut raw = ContactRequest::default();
    let mut kinds = Vec::new();
    let mut pow_malformed = false;
    for (key, value) in form_urlencoded::parse(body) {
        let slot = match &*key {
            "kind" => {
                kinds.push(value.into_owned());
                continue;
            }
            "pow_counter" => {
                match value.trim().parse() {
                    Ok(n) => raw.pow_counter = Some(n),
                    Err(_) => pow_malformed = true,
                }
                continue;
            }
            "message" => &mut raw.message,
            "timeline" => &mut raw.timeline,
            "budget" => &mut raw.budget,
            "name" => &mut raw.name,
            "email" => &mut raw.email,
            "telegram" => &mut raw.telegram,
            "company" => &mut raw.company,
            "lang" => &mut raw.lang,
            "website" => &mut raw.website,
            "pow_challenge" => &mut raw.pow_challenge,
            "ctf_pass" => &mut raw.ctf_pass,
            _ => continue,
        };
        // First occurrence wins.
        slot.get_or_insert_with(|| value.into_owned());
    }
    raw.kind = Some(kinds);
    (raw, pow_malformed)
}

fn accepted(flavor: Flavor, lang: &str) -> Response {
    match flavor {
        Flavor::Json => json(StatusCode::OK, &ContactResponse::ok()),
        Flavor::Form => see_other(&format!("/{lang}/sent/")),
    }
}

fn refuse(flavor: Flavor, lang: &str, status: StatusCode, field: Field, code: ErrorCode) -> Response {
    rejected(flavor, lang, status, vec![FieldError::new(field, code)])
}

fn rejected(flavor: Flavor, lang: &str, status: StatusCode, errors: Vec<FieldError>) -> Response {
    match (flavor, errors.first()) {
        (Flavor::Form, Some(first)) => see_other(&format!("/{lang}/sent/#e-{}", first.code)),
        _ => json(status, &ContactResponse::errors(errors)),
    }
}

/// `303 See Other` (the no-JS flow). `location` is built from a configured language and a
/// fixed error code, never from request data.
fn see_other(location: &str) -> Response {
    let Ok(location) = HeaderValue::try_from(location) else { return internal_error() };
    let mut response = Response::new(Body::empty());
    *response.status_mut() = StatusCode::SEE_OTHER;
    response.headers_mut().insert(LOCATION, location);
    response.headers_mut().insert(CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn form_bodies() {
        let (raw, bad) = parse_form(b"kind=ai&kind=design&name=Bill+K&name=Second&email=b%40m.tj&website=&x=1");
        assert_eq!(raw.kind, Some(vec!["ai".to_string(), "design".to_string()]));
        assert_eq!(raw.name.as_deref(), Some("Bill K"));
        assert_eq!(raw.email.as_deref(), Some("b@m.tj"));
        assert_eq!(raw.website.as_deref(), Some(""));
        assert!(!bad);
        let (_, bad) = parse_form(b"pow_counter=12x");
        assert!(bad);
    }
}
