//! Content negotiation: `Accept-Language`, `Accept-Encoding` and `If-None-Match`.
//!
//! All parsers are total: malformed input degrades to the default choice, never to an error.

use http::HeaderValue;
use http::header::GetAll;

/// At most this many list items are looked at per header (cheap bound on hostile input).
const MAX_ITEMS: usize = 32;

/// A quality value in thousandths: `q=0.5` → 500. Missing `q` means 1000.
type Quality = u16;

/// One `token;q=…` list item.
fn items<'a>(values: GetAll<'a, HeaderValue>) -> impl Iterator<Item = (&'a str, Quality)> + 'a {
    values
        .into_iter()
        .filter_map(|v| v.to_str().ok())
        .flat_map(|v| v.split(','))
        .take(MAX_ITEMS)
        .filter_map(|item| {
            let mut parts = item.split(';');
            let token = parts.next()?.trim();
            if token.is_empty() {
                return None;
            }
            let mut q = 1000;
            for param in parts {
                let (name, value) = param.split_once('=')?;
                if name.trim().eq_ignore_ascii_case("q") {
                    q = parse_quality(value)?;
                }
            }
            Some((token, q))
        })
}

/// RFC 9110 `qvalue`: `0(.ddd)?` or `1(.000)?`.
fn parse_quality(s: &str) -> Option<Quality> {
    let s = s.trim();
    let (int, frac) = s.split_once('.').unwrap_or((s, ""));
    if frac.len() > 3 || !frac.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let mut q: Quality = match int {
        "0" => 0,
        "1" => 1000,
        _ => return None,
    };
    let mut scale = 100;
    for digit in frac.bytes() {
        q += Quality::from(digit - b'0') * scale;
        scale /= 10;
    }
    (q <= 1000).then_some(q)
}

/// The primary subtag of a language tag (`en-US` → `en`).
fn primary(tag: &str) -> &str {
    tag.split(['-', '_']).next().unwrap_or(tag)
}

/// Picks the language to serve from `Accept-Language`: the available language whose primary
/// subtag matches the highest-quality range (earlier ranges win ties). `*` stands for the default
/// language (or the first one not excluded with `q=0`). Nothing acceptable → `default`.
pub(crate) fn language<'a>(
    accept: GetAll<'_, HeaderValue>,
    available: &'a [String],
    default: &'a str,
) -> &'a str {
    let mut best: Option<(Quality, &'a str)> = None;
    let mut star: Option<Quality> = None;
    let mut excluded = [false; MAX_ITEMS];
    for (range, q) in items(accept) {
        if range == "*" {
            star = star.or(Some(q));
            continue;
        }
        let wanted = primary(range);
        let Some(index) = available.iter().position(|l| primary(l).eq_ignore_ascii_case(wanted)) else {
            continue;
        };
        if q == 0 {
            if let Some(slot) = excluded.get_mut(index) {
                *slot = true;
            }
        } else if best.is_none_or(|(best_q, _)| q > best_q) {
            best = Some((q, &available[index]));
        }
    }
    if let Some((_, lang)) = best {
        return lang;
    }
    if star.is_some_and(|q| q > 0) {
        let allowed = |i: &usize| !excluded.get(*i).copied().unwrap_or(false);
        let preferred = available.iter().position(|l| l == default).filter(allowed);
        if let Some(i) = preferred.or_else(|| (0..available.len()).find(allowed)) {
            return &available[i];
        }
    }
    default
}

/// A content coding the server has a precompressed file for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Coding {
    Identity,
    Br,
    Gzip,
}

/// Server preference br > gzip > identity, restricted to what the client accepts (`q=0`
/// excludes; an unlisted coding is acceptable only through `*`). Identity is the fallback even
/// when the client excluded it — a body it can't decode is no better than a 406.
pub(crate) fn coding(accept: GetAll<'_, HeaderValue>, has_br: bool, has_gzip: bool) -> Coding {
    if !has_br && !has_gzip {
        return Coding::Identity;
    }
    let (mut br, mut gzip, mut star) = (None, None, None);
    for (token, q) in items(accept) {
        if token.eq_ignore_ascii_case("br") {
            br = br.or(Some(q));
        } else if token.eq_ignore_ascii_case("gzip") || token.eq_ignore_ascii_case("x-gzip") {
            gzip = gzip.or(Some(q));
        } else if token == "*" {
            star = star.or(Some(q));
        }
    }
    let acceptable = |q: Option<Quality>| q.or(star).is_some_and(|q| q > 0);
    if has_br && acceptable(br) {
        Coding::Br
    } else if has_gzip && acceptable(gzip) {
        Coding::Gzip
    } else {
        Coding::Identity
    }
}

/// `If-None-Match` against our (strong, quoted) ETag, with the weak comparison RFC 9110 §13.1.2
/// prescribes: `W/"x"` matches `"x"`; `*` matches any current representation.
pub(crate) fn none_match(values: GetAll<'_, HeaderValue>, etag: &HeaderValue) -> bool {
    let etag = etag.as_bytes();
    values.into_iter().any(|value| {
        value.as_bytes().split(|&b| b == b',').any(|tag| {
            let tag = tag.trim_ascii();
            tag == b"*" || tag.strip_prefix(b"W/").unwrap_or(tag) == etag
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use http::HeaderMap;
    use http::header::{ACCEPT_ENCODING, ACCEPT_LANGUAGE, IF_NONE_MATCH};

    fn map(name: http::HeaderName, values: &[&str]) -> HeaderMap {
        let mut m = HeaderMap::new();
        for v in values {
            m.append(name.clone(), HeaderValue::from_str(v).unwrap());
        }
        m
    }

    fn lang(values: &[&str]) -> String {
        let langs = ["en".to_string(), "ru".to_string(), "tg".to_string()];
        let m = map(ACCEPT_LANGUAGE, values);
        language(m.get_all(ACCEPT_LANGUAGE), &langs, "en").to_owned()
    }

    #[test]
    fn languages() {
        assert_eq!(lang(&[]), "en");
        assert_eq!(lang(&["ru-RU,ru;q=0.9,en-US;q=0.8,en;q=0.7"]), "ru");
        assert_eq!(lang(&["de-DE,de;q=0.9,tg;q=0.5,ru;q=0.4"]), "tg");
        assert_eq!(lang(&["fr"]), "en");
        assert_eq!(lang(&["en;q=0.5, ru;q=0.6"]), "ru");
        assert_eq!(lang(&["ru;q=0.5, tg;q=0.5"]), "ru", "ties go to the earlier range");
        assert_eq!(lang(&["TG-tj"]), "tg", "case-insensitive");
        assert_eq!(lang(&["en;q=0, *"]), "ru", "* never picks an excluded language");
        assert_eq!(lang(&["*"]), "en");
        assert_eq!(lang(&["ru;q=0"]), "en", "nothing acceptable: default");
        assert_eq!(lang(&["ru;q=2"]), "en", "invalid q ignores the item");
        assert_eq!(lang(&["fr", "ru"]), "ru", "multiple header lines");
    }

    fn enc(values: &[&str], br: bool, gz: bool) -> Coding {
        let m = map(ACCEPT_ENCODING, values);
        coding(m.get_all(ACCEPT_ENCODING), br, gz)
    }

    #[test]
    fn encodings() {
        use Coding::*;
        assert_eq!(enc(&[], true, true), Identity);
        assert_eq!(enc(&["gzip, deflate, br, zstd"], true, true), Br);
        assert_eq!(enc(&["gzip;q=1, br;q=0.1"], true, true), Br, "server preference");
        assert_eq!(enc(&["gzip, br;q=0"], true, true), Gzip);
        assert_eq!(enc(&["br"], false, true), Identity);
        assert_eq!(enc(&["*"], true, true), Br);
        assert_eq!(enc(&["*;q=0, gzip"], true, true), Gzip);
        assert_eq!(enc(&["x-gzip"], true, true), Gzip);
        assert_eq!(enc(&["BR;Q=0.5"], true, false), Br);
        assert_eq!(enc(&[""], true, true), Identity);
        assert_eq!(enc(&["identity;q=0"], false, false), Identity);
    }

    #[test]
    fn etags() {
        let etag = HeaderValue::from_static("\"abc\"");
        let check = |v: &[&str]| {
            let m = map(IF_NONE_MATCH, v);
            none_match(m.get_all(IF_NONE_MATCH), &etag)
        };
        assert!(check(&["\"abc\""]));
        assert!(check(&["W/\"abc\""]));
        assert!(check(&["\"x\", \"abc\""]));
        assert!(check(&["*"]));
        assert!(!check(&["\"abcd\""]));
        assert!(!check(&[]));
    }

    #[test]
    fn qualities() {
        assert_eq!(parse_quality("0"), Some(0));
        assert_eq!(parse_quality("0.5"), Some(500));
        assert_eq!(parse_quality("0.001"), Some(1));
        assert_eq!(parse_quality("1.000"), Some(1000));
        assert_eq!(parse_quality("1.001"), None);
        assert_eq!(parse_quality("0.0001"), None);
        assert_eq!(parse_quality("-1"), None);
    }
}
