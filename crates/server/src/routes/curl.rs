//! curl mode (docs/CONTRACTS.md §4.6): command-line clients get plain text with 24-bit ANSI
//! colour instead of HTML, for `/`, `/{lang}/`, `/pillars`, `/contact` (from the manifest's text
//! files) and `/proof` (generated here from the same facts as `/_proof`).

use std::fmt::Write as _;
use std::time::Duration;

use axum::body::Body;
use axum::extract::Request;
use axum::response::Response;
use http::header::{ACCEPT_LANGUAGE, CACHE_CONTROL, CONTENT_LANGUAGE, CONTENT_TYPE, USER_AGENT, VARY};
use http::{HeaderMap, HeaderValue, StatusCode, Version};
use proof::Proof;

use super::{ctx, internal_error};
use crate::app::AppState;
use crate::assets::{CURL_PAGES, Site, TextTemplate};
use crate::conn::ReqCtx;
use crate::headers::{micros, push_ms};
use crate::negotiate;

const STARLIGHT: &str = "\x1b[38;2;244;245;247m";
const MIST: &str = "\x1b[38;2;201;206;216m";
const BLUE: &str = "\x1b[38;2;61;123;255m";
const RUST: &str = "\x1b[38;2;226;85;45m";
const SUCCESS: &str = "\x1b[38;2;47;191;113m";
const RESET: &str = "\x1b[0m";

/// `User-Agent` starts with `curl/`, `Wget/`, `HTTPie/` or `xh/`.
pub(crate) fn is_cli_agent(headers: &HeaderMap) -> bool {
    headers.get(USER_AGENT).is_some_and(|ua| {
        let ua = ua.as_bytes();
        [&b"curl/"[..], b"Wget/", b"HTTPie/", b"xh/"].iter().any(|prefix| ua.starts_with(prefix))
    })
}

/// What a curl-mode path resolves to.
pub(crate) enum Target<'a> {
    Text { lang: &'a str, template: &'a TextTemplate, negotiated: bool, key: &'static str },
    Proof,
}

impl Target<'_> {
    /// Access-log label.
    pub fn label(&self) -> &'static str {
        match self {
            Target::Proof => "curl /proof",
            Target::Text { key: "/", .. } => "curl /",
            Target::Text { key: "/pillars", .. } => "curl /pillars",
            Target::Text { .. } => "curl /contact",
        }
    }
}

/// `/pillars` → `/pillars`; `/{lang}/contact` → (`lang`, `/contact`); `/{lang}/` → (`lang`, `/`).
fn split<'a>(site: &'a Site, path: &str) -> Option<(Option<&'a str>, &'static str)> {
    let page = |tail: &str| CURL_PAGES.iter().copied().find(|p| *p == tail);
    if let Some(key) = page(path) {
        return Some((None, key));
    }
    let (lang, tail) = path.strip_prefix('/')?.split_once('/')?;
    let lang = site.lang(lang)?;
    let key = match tail {
        "" => "/",
        "pillars" => "/pillars",
        "contact" => "/contact",
        _ => return None,
    };
    Some((Some(lang), key))
}

/// Paths that answer differently to command-line clients (so their responses vary on
/// `User-Agent`).
pub(crate) fn is_curl_path(site: &Site, path: &str) -> bool {
    path == "/proof" || split(site, path).is_some()
}

/// The curl-mode target for a path, if this path has one.
pub(crate) fn target<'a>(site: &'a Site, path: &str, headers: &HeaderMap) -> Option<Target<'a>> {
    if path == "/proof" {
        return Some(Target::Proof);
    }
    let (explicit, key) = split(site, path)?;
    let lang = explicit.unwrap_or_else(|| {
        negotiate::language(headers.get_all(ACCEPT_LANGUAGE), &site.langs, &site.default_lang)
    });
    let (lang, template) = site.text(lang, key)?;
    Some(Target::Text { lang, template, negotiated: explicit.is_none(), key })
}

/// Renders a target.
pub(crate) fn respond(app: &AppState, req: &Request, target: &Target<'_>) -> Response {
    let Some(ctx) = ctx(req) else { return internal_error() };
    let (body, lang, vary) = match target {
        Target::Proof => {
            let facts = super::proof::collect(app, &ctx, req.version());
            (proof_text(&facts, &ctx).into_bytes(), None, "User-Agent")
        }
        Target::Text { lang, template, negotiated, .. } => {
            let tls = ctx.conn.tls.as_ref().map_or_else(|| "plain HTTP (development)".to_owned(), |t| t.describe());
            let server_time = server_time(ctx.t0.elapsed());
            let body = template.render(&server_time, &tls, http_short(req.version()));
            (body, Some(*lang), if *negotiated { "Accept-Language, User-Agent" } else { "User-Agent" })
        }
    };
    let mut response = Response::new(Body::from(body));
    *response.status_mut() = StatusCode::OK;
    let h = response.headers_mut();
    h.insert(CONTENT_TYPE, HeaderValue::from_static("text/plain; charset=utf-8"));
    h.insert(CACHE_CONTROL, HeaderValue::from_static("no-store"));
    h.insert(VARY, HeaderValue::from_static(vary));
    if let Some(lang) = lang.and_then(|l| HeaderValue::from_str(l).ok()) {
        h.insert(CONTENT_LANGUAGE, lang);
    }
    response
}

/// `{{server_time}}`: µs below 1 ms, else ms with one decimal.
pub(crate) fn server_time(elapsed: Duration) -> String {
    let us = micros(elapsed);
    if us < 1000 { format!("{us} µs") } else { format!("{:.1} ms", us as f64 / 1000.0) }
}

/// `{{http}}`: `HTTP/2`, `HTTP/1.1`.
fn http_short(version: Version) -> &'static str {
    match version {
        Version::HTTP_09 => "HTTP/0.9",
        Version::HTTP_10 => "HTTP/1.0",
        Version::HTTP_11 => "HTTP/1.1",
        Version::HTTP_2 => "HTTP/2",
        Version::HTTP_3 => "HTTP/3",
        _ => "HTTP",
    }
}

fn ms(us: u64) -> String {
    let mut s = String::new();
    push_ms(&mut s, us);
    s.push_str(" ms");
    s
}

/// `/proof` for terminals: the `/_proof` facts as aligned, coloured lines.
fn proof_text(p: &Proof, ctx: &ReqCtx) -> String {
    let mut out = String::with_capacity(2048);
    let mut line = |label: &str, colour: &str, value: &str| {
        let _ = writeln!(out, "  {MIST}{label:<12}{RESET}{colour}{value}{RESET}");
    };
    let s = &p.server;
    let place = s.location.as_ref().map_or_else(
        || "location not configured".to_owned(),
        |l| format!("{}, {} ({}, {})", l.city, l.country, l.lat, l.lon),
    );
    line("server", STARLIGHT, &format!("{} · {place}", s.region));
    line("build", STARLIGHT, &s.build);
    line("software", STARLIGHT, &s.software);
    line("uptime", STARLIGHT, &format!("{} s · {} cores", s.uptime_s, s.cores));
    match &p.tls {
        Some(t) => {
            let alpn = t.alpn.as_deref().map_or_else(String::new, |a| format!(" · ALPN {a}"));
            let cipher = t.cipher.as_deref().unwrap_or("cipher not reported");
            line("tls", STARLIGHT, &format!("{} · {cipher}{alpn}", t.version));
            match (&t.group, t.post_quantum) {
                (Some(group), true) => line("key share", SUCCESS, &format!("{group} · post-quantum")),
                (Some(group), false) => line("key share", RUST, &format!("{group} · not post-quantum")),
                (None, _) => line("key share", MIST, "not reported"),
            }
            if let Some(us) = t.handshake_us {
                line("handshake", STARLIGHT, &format!("{} (server side, includes one round trip)", ms(us)));
            }
        }
        None => line("tls", RUST, "plain HTTP (development)"),
    }
    line("http", STARLIGHT, &p.http.version);
    let c = &p.connection;
    line("connection", STARLIGHT, &format!("#{} · request {} on it", c.id, c.requests));
    let rtt = match (c.kernel_rtt_us, c.kernel_rttvar_us) {
        (Some(rtt), Some(var)) => format!("{} ± {} (TCP_INFO)", ms(rtt.into()), ms(var.into())),
        _ if c.quic_rtt_us.is_some() => "not available (QUIC: no TCP socket)".to_owned(),
        _ => "not available".to_owned(),
    };
    line("kernel rtt", STARLIGHT, &rtt);
    if let Some(rtt) = c.quic_rtt_us {
        line("quic rtt", STARLIGHT, &format!("{} (QUIC's own estimate)", ms(rtt.into())));
    }
    let v = &p.privacy;
    let yes_no = |b: bool, what: &str| if b { format!("IP {what}") } else { format!("IP not {what}") };
    line(
        "privacy",
        SUCCESS,
        &format!(
            "{} cookies · {} third-party origins · {} · {}",
            v.cookies_set,
            v.third_party_origins,
            yes_no(v.ip_stored, "stored"),
            yes_no(v.ip_logged, "logged")
        ),
    );
    for (i, header) in p.headers.iter().enumerate() {
        line(if i == 0 { "headers" } else { "" }, STARLIGHT, header);
    }
    line("served in", BLUE, &format!("{} (server time so far)", server_time(ctx.t0.elapsed())));
    format!("\n  {BLUE}mintaka{RESET} {MIST}— proof of this request{RESET}\n\n{out}\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn agents() {
        let ua = |v: &str| {
            let mut h = HeaderMap::new();
            h.insert(USER_AGENT, HeaderValue::from_str(v).unwrap());
            is_cli_agent(&h)
        };
        assert!(ua("curl/8.23.0"));
        assert!(ua("Wget/1.24.5"));
        assert!(ua("HTTPie/3.2.4"));
        assert!(ua("xh/0.23.0"));
        assert!(!ua("Mozilla/5.0 (X11; Linux x86_64) curl/8"));
        assert!(!ua("CURL/8"));
        assert!(!is_cli_agent(&HeaderMap::new()));
    }

    #[test]
    fn server_time_format() {
        assert_eq!(server_time(Duration::from_micros(21)), "21 µs");
        assert_eq!(server_time(Duration::from_micros(999)), "999 µs");
        assert_eq!(server_time(Duration::from_micros(1234)), "1.2 ms");
    }
}
