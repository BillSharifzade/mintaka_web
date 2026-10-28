//! Response headers sent on every response (docs/CONTRACTS.md §4.2) and `Server-Timing` (§4.5).

use std::fmt::Write as _;
use std::time::Duration;

use anyhow::ensure;
use http::header::{
    CONTENT_SECURITY_POLICY, HeaderName, REFERRER_POLICY, STRICT_TRANSPORT_SECURITY,
    X_CONTENT_TYPE_OPTIONS,
};
use http::{HeaderMap, HeaderValue};
use proof::Csp;

use crate::sys::TcpRtt;

pub(crate) const SERVER_TIMING: HeaderName = HeaderName::from_static("server-timing");
const COOP: HeaderName = HeaderName::from_static("cross-origin-opener-policy");
const COEP: HeaderName = HeaderName::from_static("cross-origin-embedder-policy");
const CORP: HeaderName = HeaderName::from_static("cross-origin-resource-policy");
const PERMISSIONS_POLICY: HeaderName = HeaderName::from_static("permissions-policy");

const HSTS: &str = "max-age=63072000; includeSubDomains; preload";
const PERMISSIONS: &str = "camera=(), microphone=(), geolocation=(), payment=(), usb=(), browsing-topics=()";

/// A header value that lives as long as the process. Built once at startup; cloning it per
/// request copies a pointer instead of touching a shared reference count.
pub(crate) fn static_value(value: String) -> anyhow::Result<HeaderValue> {
    // `from_static` accepts visible ASCII only (it panics otherwise); anything else goes through
    // the checked, reference-counted constructor.
    if value.bytes().all(|b| b == b'\t' || (0x20..0x7f).contains(&b)) {
        Ok(HeaderValue::from_static(value.leak()))
    } else {
        Ok(HeaderValue::from_str(&value)?)
    }
}

/// The security header set, pre-built for plain HTTP and for TLS.
pub(crate) struct SecurityHeaders {
    plain: Vec<(HeaderName, HeaderValue)>,
    tls: Vec<(HeaderName, HeaderValue)>,
    third_party_origins: u32,
}

impl SecurityHeaders {
    /// Assembles the CSP from the manifest's hash sources (rejecting anything that is not a
    /// well-formed `sha256-…`/`sha384-…`/`sha512-…` source, which could otherwise inject
    /// directives).
    pub fn new(csp: &Csp) -> anyhow::Result<Self> {
        for source in csp.style_src.iter().chain(&csp.script_src) {
            ensure!(is_hash_source(source), "manifest csp: {source:?} is not a sha256-/sha384-/sha512- hash source");
        }
        let plain_csp = content_security_policy(csp, false);
        let tls_csp = content_security_policy(csp, true);
        let third_party_origins = count_third_party_origins(&tls_csp);
        let common = |csp: String| -> anyhow::Result<Vec<(HeaderName, HeaderValue)>> {
            Ok(vec![
                (CONTENT_SECURITY_POLICY, static_value(csp)?),
                (COOP, HeaderValue::from_static("same-origin")),
                (COEP, HeaderValue::from_static("require-corp")),
                (CORP, HeaderValue::from_static("same-origin")),
                (PERMISSIONS_POLICY, HeaderValue::from_static(PERMISSIONS)),
                (REFERRER_POLICY, HeaderValue::from_static("no-referrer")),
                (X_CONTENT_TYPE_OPTIONS, HeaderValue::from_static("nosniff")),
            ])
        };
        let plain = common(plain_csp)?;
        let mut tls = common(tls_csp)?;
        tls.push((STRICT_TRANSPORT_SECURITY, HeaderValue::from_static(HSTS)));
        Ok(Self { plain, tls, third_party_origins })
    }

    fn set(&self, tls: bool) -> &[(HeaderName, HeaderValue)] {
        if tls { &self.tls } else { &self.plain }
    }

    /// Inserts (overwriting) the full set into a response.
    pub fn apply(&self, headers: &mut HeaderMap, tls: bool) {
        for (name, value) in self.set(tls) {
            headers.insert(name.clone(), value.clone());
        }
    }

    /// Names of the headers [`Self::apply`] sends, in contract order.
    pub fn names(&self, tls: bool) -> impl Iterator<Item = &str> {
        self.set(tls).iter().map(|(name, _)| name.as_str())
    }

    /// The CSP value sent over plain HTTP or TLS.
    #[cfg(test)]
    pub fn csp(&self, tls: bool) -> &HeaderValue {
        &self.set(tls)[0].1
    }

    /// Origins other than `'self'` that the CSP lets the page contact (0 by construction today).
    pub fn third_party_origins(&self) -> u32 {
        self.third_party_origins
    }
}

/// `sha256-<base64>` and friends (CSP3 `hash-source`, without quotes).
fn is_hash_source(s: &str) -> bool {
    let Some((alg, b64)) = s.split_once('-') else { return false };
    matches!(alg, "sha256" | "sha384" | "sha512")
        && !b64.is_empty()
        && b64.trim_end_matches('=').bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'+' | b'/' | b'-' | b'_'))
        && b64.len() - b64.trim_end_matches('=').len() <= 2
}

/// The policy of §4.2, with `upgrade-insecure-requests` only over TLS.
fn content_security_policy(csp: &Csp, tls: bool) -> String {
    let mut out = String::from("default-src 'none'; script-src 'self' 'wasm-unsafe-eval'");
    for hash in &csp.script_src {
        let _ = write!(out, " '{hash}'");
    }
    out.push_str("; style-src 'self'");
    for hash in &csp.style_src {
        let _ = write!(out, " '{hash}'");
    }
    out.push_str(
        "; img-src 'self'; font-src 'self'; connect-src 'self'; worker-src 'self'; \
         manifest-src 'self'; form-action 'self'; base-uri 'none'; frame-ancestors 'none'; \
         require-trusted-types-for 'script'; trusted-types mintaka",
    );
    if tls {
        out.push_str("; upgrade-insecure-requests");
    }
    out
}

/// Counts distinct host- or scheme-sources in fetch directives: anything that is not a quoted
/// keyword/hash. `trusted-types` names policies, not origins, so it is skipped.
fn count_third_party_origins(policy: &str) -> u32 {
    let mut origins: Vec<&str> = Vec::new();
    for directive in policy.split(';') {
        let mut words = directive.split_ascii_whitespace();
        let Some(name) = words.next() else { continue };
        if matches!(name, "trusted-types" | "require-trusted-types-for" | "upgrade-insecure-requests") {
            continue;
        }
        for source in words.filter(|w| !w.starts_with('\'')) {
            if !origins.contains(&source) {
                origins.push(source);
            }
        }
    }
    u32::try_from(origins.len()).unwrap_or(u32::MAX)
}

/// One `Server-Timing` value (§4.5): `app`, `route`, `render` always; `tls` on the first request
/// of a TLS connection; `krtt` when the kernel has an RTT sample.
pub(crate) fn server_timing(
    app: Duration,
    route: Duration,
    render: Duration,
    tls_handshake: Option<Duration>,
    rtt: Option<TcpRtt>,
) -> Option<HeaderValue> {
    let mut s = String::with_capacity(128);
    s.push_str("app;dur=");
    push_ms(&mut s, micros(app));
    s.push_str(", route;dur=");
    push_ms(&mut s, micros(route));
    s.push_str(", render;dur=");
    push_ms(&mut s, micros(render));
    if let Some(handshake) = tls_handshake {
        s.push_str(", tls;dur=");
        push_ms(&mut s, micros(handshake));
        s.push_str(";desc=\"TLS handshake\"");
    }
    if let Some(rtt) = rtt {
        s.push_str(", krtt;dur=");
        push_ms(&mut s, u64::from(rtt.rtt_us));
        s.push_str(";desc=\"kernel RTT\"");
    }
    HeaderValue::try_from(s).ok()
}

/// Rounded to the nearest microsecond.
pub(crate) fn micros(d: Duration) -> u64 {
    u64::try_from((d.as_nanos() + 500) / 1000).unwrap_or(u64::MAX)
}

/// Microseconds as milliseconds with up to three decimals: 21 → `0.021`, 1830 → `1.83`, 2000 → `2`.
pub(crate) fn push_ms(s: &mut String, us: u64) {
    let _ = write!(s, "{}", us / 1000);
    let frac = us % 1000;
    if frac != 0 {
        let digits = [frac / 100, frac / 10 % 10, frac % 10];
        let len = if digits[2] != 0 { 3 } else if digits[1] != 0 { 2 } else { 1 };
        s.push('.');
        for d in &digits[..len] {
            s.push(char::from(b'0' + *d as u8));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn csp() -> Csp {
        Csp { style_src: vec!["sha256-AAAA+/9=".into(), "sha384-BBBB".into()], script_src: vec![] }
    }

    #[test]
    fn csp_matches_the_contract() {
        let h = SecurityHeaders::new(&csp()).unwrap();
        let expected = "default-src 'none'; script-src 'self' 'wasm-unsafe-eval'; style-src 'self' \
             'sha256-AAAA+/9=' 'sha384-BBBB'; img-src 'self'; font-src 'self'; connect-src 'self'; \
             worker-src 'self'; manifest-src 'self'; form-action 'self'; base-uri 'none'; \
             frame-ancestors 'none'; require-trusted-types-for 'script'; trusted-types mintaka";
        assert_eq!(h.csp(false), expected);
        assert_eq!(h.csp(true), &format!("{expected}; upgrade-insecure-requests"));
        assert_eq!(h.third_party_origins(), 0);
        assert_eq!(h.names(false).count(), 7);
        assert_eq!(h.names(true).last(), Some("strict-transport-security"));
    }

    #[test]
    fn script_hashes_are_appended() {
        let c = Csp { style_src: vec![], script_src: vec!["sha256-xyz=".into()] };
        let h = SecurityHeaders::new(&c).unwrap();
        assert!(h.csp(false).to_str().unwrap().contains("script-src 'self' 'wasm-unsafe-eval' 'sha256-xyz='; style-src 'self';"));
    }

    #[test]
    fn rejects_injection_through_hashes() {
        for bad in ["sha256-abc'; script-src *", "md5-abc", "sha256-", "'sha256-abc'", "sha256-a b"] {
            let c = Csp { style_src: vec![bad.into()], script_src: vec![] };
            assert!(SecurityHeaders::new(&c).is_err(), "{bad}");
        }
    }

    #[test]
    fn counts_third_party_sources() {
        assert_eq!(count_third_party_origins("img-src 'self' https://cdn.example; connect-src https://cdn.example wss://x"), 2);
        assert_eq!(count_third_party_origins("trusted-types mintaka"), 0);
    }

    #[test]
    fn milliseconds_with_up_to_three_decimals() {
        let f = |us| {
            let mut s = String::new();
            push_ms(&mut s, us);
            s
        };
        assert_eq!(f(21), "0.021");
        assert_eq!(f(1830), "1.83");
        assert_eq!(f(48210), "48.21");
        assert_eq!(f(2000), "2");
        assert_eq!(f(0), "0");
        assert_eq!(f(100), "0.1");
        assert_eq!(micros(Duration::from_nanos(1499)), 1);
        assert_eq!(micros(Duration::from_nanos(1500)), 2);
    }

    #[test]
    fn server_timing_value() {
        let v = server_timing(
            Duration::from_micros(21),
            Duration::from_micros(2),
            Duration::from_micros(3),
            Some(Duration::from_micros(1830)),
            Some(TcpRtt { rtt_us: 48210, rttvar_us: 1200 }),
        )
        .unwrap();
        assert_eq!(
            v,
            "app;dur=0.021, route;dur=0.002, render;dur=0.003, tls;dur=1.83;desc=\"TLS handshake\", krtt;dur=48.21;desc=\"kernel RTT\""
        );
    }
}
