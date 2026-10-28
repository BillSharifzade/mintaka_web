//! The site in memory: `dist/manifest.json` and every file it lists, loaded once at startup
//! (docs/CONTRACTS.md §3). Nothing here touches the file system after [`Site::load`] returns.
//!
//! For every route and every precompressed variant the complete `200` and `304` header maps are
//! built up front, so serving a static file is a hash lookup, an `Accept-Encoding` parse and two
//! clones (header map, body handle).

use std::collections::HashMap;
use std::path::{Component, Path};

use anyhow::{Context, bail, ensure};
use bytes::Bytes;
use http::header::{
    CACHE_CONTROL, CONTENT_ENCODING, CONTENT_LANGUAGE, CONTENT_LENGTH, CONTENT_TYPE, ETAG, VARY,
};
use http::{HeaderMap, HeaderValue};
use proof::{CachePolicy, Csp, Encoding, MANIFEST_VERSION, Manifest, RouteKind};
use sha2::{Digest, Sha256};

use crate::headers::static_value;
use crate::negotiate::Coding;

/// Replaced in each 404 page by a link to the closest page route.
pub(crate) const SUGGEST_MARKER: &[u8] = b"<!--mintaka:suggest-->";

/// The page paths (after `/{lang}`) that curl mode answers with text.
pub(crate) const CURL_PAGES: [&str; 3] = ["/", "/pillars", "/contact"];

/// Longest prefix of a requested path that the 404 suggestion compares (bounds the work an
/// arbitrary URL can cause).
const SUGGEST_PROBE_LEN: usize = 256;

/// One way of sending a route's bytes.
pub(crate) struct Variant {
    pub body: Bytes,
    pub etag: HeaderValue,
    /// Headers of a `200`: type, length, cache policy, validator, `Vary`, encoding, language.
    pub headers: HeaderMap,
    /// Headers of a `304`: validator, cache policy, `Vary` (RFC 9110 §15.4.5).
    pub not_modified: HeaderMap,
}

/// The representations of one route: always identity, plus the precompressed siblings.
pub(crate) struct Variants {
    pub identity: Variant,
    pub br: Option<Variant>,
    pub gzip: Option<Variant>,
}

impl Variants {
    /// The variant for a negotiated coding (falls back to identity).
    pub fn pick(&self, coding: Coding) -> &Variant {
        match coding {
            Coding::Br => self.br.as_ref(),
            Coding::Gzip => self.gzip.as_ref(),
            Coding::Identity => None,
        }
        .unwrap_or(&self.identity)
    }
}

/// A route from `manifest.routes`.
pub(crate) struct StaticRoute {
    pub path: String,
    pub variants: Variants,
}

/// A 404 page, split at its suggestion marker.
pub(crate) struct NotFoundPage {
    pub lang: String,
    /// `Content-Language` value.
    pub lang_header: HeaderValue,
    head: Bytes,
    /// `None` when the page has no marker (served as is).
    tail: Option<Bytes>,
}

impl NotFoundPage {
    /// The page with the marker replaced by `suggestion` (or by nothing).
    pub fn render(&self, suggestion: Option<&PageLink>) -> Bytes {
        let Some(tail) = &self.tail else { return self.head.clone() };
        let link = suggestion.map_or(&b""[..], |s| &s.html[..]);
        let mut body = Vec::with_capacity(self.head.len() + link.len() + tail.len());
        body.extend_from_slice(&self.head);
        body.extend_from_slice(link);
        body.extend_from_slice(tail);
        Bytes::from(body)
    }
}

/// A page route as a 404 suggestion candidate.
pub(crate) struct PageLink {
    pub path: String,
    pub lang: Option<String>,
    /// `<a href="/en/">/en/</a>`, HTML-escaped.
    pub html: Bytes,
}

/// A curl-mode text file with its `{{placeholder}}` tokens pre-split. `{{build}}` is constant and
/// substituted at load time.
pub(crate) struct TextTemplate {
    parts: Vec<Part>,
    literal_len: usize,
}

enum Part {
    Literal(Bytes),
    ServerTime,
    Tls,
    Http,
}

impl TextTemplate {
    fn parse(src: &[u8], build: &str) -> TextTemplate {
        let mut parts = Vec::new();
        let mut literal = Vec::new();
        let mut rest = src;
        let flush = |literal: &mut Vec<u8>, parts: &mut Vec<Part>| {
            if !literal.is_empty() {
                parts.push(Part::Literal(Bytes::from(std::mem::take(literal))));
            }
        };
        while let Some(open) = find(rest, b"{{") {
            literal.extend_from_slice(&rest[..open]);
            let after = &rest[open + 2..];
            let Some(close) = find(after, b"}}") else {
                rest = &rest[open..];
                break;
            };
            match &after[..close] {
                b"build" => literal.extend_from_slice(build.as_bytes()),
                token @ (b"server_time" | b"tls" | b"http") => {
                    flush(&mut literal, &mut parts);
                    parts.push(match token {
                        b"server_time" => Part::ServerTime,
                        b"tls" => Part::Tls,
                        _ => Part::Http,
                    });
                }
                // Unknown tokens are left as written.
                _ => literal.extend_from_slice(&rest[open..open + close + 4]),
            }
            rest = &after[close + 2..];
        }
        literal.extend_from_slice(rest);
        flush(&mut literal, &mut parts);
        let literal_len = parts.iter().map(|p| if let Part::Literal(b) = p { b.len() } else { 0 }).sum();
        TextTemplate { parts, literal_len }
    }

    /// Fills the per-request tokens.
    pub fn render(&self, server_time: &str, tls: &str, http: &str) -> Vec<u8> {
        let mut out = Vec::with_capacity(self.literal_len + 128);
        for part in &self.parts {
            out.extend_from_slice(match part {
                Part::Literal(bytes) => bytes,
                Part::ServerTime => server_time.as_bytes(),
                Part::Tls => tls.as_bytes(),
                Part::Http => http.as_bytes(),
            });
        }
        out
    }
}

fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|w| w == needle)
}

/// The loaded site.
pub(crate) struct Site {
    pub build: String,
    pub origin: String,
    pub default_lang: String,
    pub langs: Vec<String>,
    pub csp: Csp,
    routes: Vec<StaticRoute>,
    index: HashMap<String, usize>,
    /// `GET /` for each language: the `/{lang}/` bytes with `Vary: Accept-Language`.
    roots: HashMap<String, Variants>,
    pages: Vec<PageLink>,
    not_found: HashMap<String, NotFoundPage>,
    text: HashMap<String, HashMap<String, TextTemplate>>,
    /// Bytes held in memory for bodies (all variants).
    pub memory_bytes: usize,
}

impl Site {
    /// Reads and checks the manifest, then loads every listed file. Fails with a message naming
    /// the offending file or field on the first problem.
    pub fn load(dist: &Path) -> anyhow::Result<Site> {
        let manifest_path = dist.join("manifest.json");
        let raw = std::fs::read(&manifest_path)
            .with_context(|| format!("cannot read {} (is --dist right?)", manifest_path.display()))?;
        let manifest: Manifest = serde_json::from_slice(&raw)
            .with_context(|| format!("{} is not a valid manifest", manifest_path.display()))?;
        ensure!(
            manifest.version == MANIFEST_VERSION,
            "{}: manifest version {} but this server reads version {MANIFEST_VERSION}; rebuild dist/ or the server",
            manifest_path.display(),
            manifest.version
        );
        Self::from_manifest(dist, manifest).with_context(|| format!("loading {}", manifest_path.display()))
    }

    fn from_manifest(dist: &Path, m: Manifest) -> anyhow::Result<Site> {
        ensure!(!m.langs.is_empty(), "`langs` is empty");
        for lang in &m.langs {
            ensure!(is_lang_tag(lang), "`langs`: {lang:?} is not a language tag");
        }
        ensure!(m.langs.contains(&m.default_lang), "`default_lang` {:?} is not in `langs`", m.default_lang);

        let mut memory_bytes = 0;
        let mut routes = Vec::with_capacity(m.routes.len());
        let mut index = HashMap::with_capacity(m.routes.len());
        let mut pages = Vec::new();
        let mut loaded: HashMap<String, Files> = HashMap::new();
        for route in &m.routes {
            ensure!(is_route_path(&route.path), "route {:?}: not an absolute URL path", route.path);
            if route.path == "/" {
                tracing::warn!("manifest lists `/`; ignored — `/` is served by Accept-Language negotiation");
                continue;
            }
            ensure!(!index.contains_key(&route.path), "route {:?} is listed twice", route.path);
            let files = Files::read(dist, &route.file, &route.encodings)
                .with_context(|| format!("route {:?}", route.path))?;
            memory_bytes += files.len();
            let mut vary = Vec::new();
            if !route.encodings.is_empty() {
                vary.push("Accept-Encoding");
            }
            if is_curl_page(&m.langs, &route.path) {
                vary.push("User-Agent");
            }
            let variants = files
                .variants(&route.content_type, route.cache, route.lang.as_deref(), &vary)
                .with_context(|| format!("route {:?}", route.path))?;
            if route.kind == RouteKind::Page {
                pages.push(PageLink {
                    path: route.path.clone(),
                    lang: route.lang.clone(),
                    html: Bytes::from(format!("<a href=\"{0}\">{0}</a>", escape_html(&route.path))),
                });
            }
            index.insert(route.path.clone(), routes.len());
            routes.push(StaticRoute { path: route.path.clone(), variants });
            loaded.insert(route.path.clone(), files);
        }

        // `GET /` serves `/{lang}/` directly, so every language needs that route.
        let mut roots = HashMap::new();
        for lang in &m.langs {
            let path = format!("/{lang}/");
            let route = m.routes.iter().find(|r| r.path == path);
            let (Some(route), Some(files)) = (route, loaded.get(&path)) else {
                bail!("language {lang:?} has no {path:?} route (needed to serve `/`)");
            };
            let mut vary = vec!["Accept-Language", "User-Agent"];
            if !route.encodings.is_empty() {
                vary.insert(0, "Accept-Encoding");
            }
            let lang_header = route.lang.as_deref().unwrap_or(lang);
            roots.insert(lang.clone(), files.variants(&route.content_type, route.cache, Some(lang_header), &vary)?);
        }

        let mut not_found = HashMap::new();
        ensure!(
            m.not_found.contains_key(&m.default_lang),
            "`not_found` has no page for the default language {:?}",
            m.default_lang
        );
        for (lang, file) in &m.not_found {
            let body = read_file(dist, file).with_context(|| format!("not_found[{lang:?}]"))?;
            memory_bytes += body.len();
            let (head, tail) = match find(&body, SUGGEST_MARKER) {
                Some(at) => (
                    Bytes::copy_from_slice(&body[..at]),
                    Some(Bytes::copy_from_slice(&body[at + SUGGEST_MARKER.len()..])),
                ),
                None => {
                    tracing::warn!(file = %file, "404 page has no <!--mintaka:suggest--> marker; served without a suggestion");
                    (Bytes::from(body), None)
                }
            };
            ensure!(is_lang_tag(lang), "`not_found`: {lang:?} is not a language tag");
            let lang_header = static_value(lang.clone())?;
            not_found.insert(lang.clone(), NotFoundPage { lang: lang.clone(), lang_header, head, tail });
        }

        let mut text = HashMap::new();
        for (lang, files) in &m.text {
            let mut templates = HashMap::new();
            for (key, file) in files {
                ensure!(key.starts_with('/'), "text[{lang:?}]: key {key:?} must start with `/`");
                let body = read_file(dist, file).with_context(|| format!("text[{lang:?}][{key:?}]"))?;
                memory_bytes += body.len();
                templates.insert(key.clone(), TextTemplate::parse(&body, &m.build));
            }
            text.insert(lang.clone(), templates);
        }

        for lang in &m.langs {
            if !index.contains_key(&format!("/{lang}/sent/")) {
                tracing::warn!(lang = %lang, "no /{{lang}}/sent/ route: the no-JS contact form will redirect to a 404");
            }
        }

        Ok(Site {
            build: m.build,
            origin: m.origin,
            default_lang: m.default_lang,
            langs: m.langs,
            csp: m.csp,
            routes,
            index,
            roots,
            pages,
            not_found,
            text,
            memory_bytes,
        })
    }

    /// Number of static routes.
    pub fn route_count(&self) -> usize {
        self.routes.len()
    }

    /// Exact static route lookup.
    pub fn route(&self, path: &str) -> Option<(usize, &StaticRoute)> {
        let i = *self.index.get(path)?;
        Some((i, self.routes.get(i)?))
    }

    /// A route by index (for log labels).
    pub fn route_at(&self, i: usize) -> Option<&StaticRoute> {
        self.routes.get(i)
    }

    /// The representations of `/` for a language.
    pub fn root(&self, lang: &str) -> Option<&Variants> {
        self.roots.get(lang)
    }

    /// The configured language equal to `lang`, if any.
    pub fn lang(&self, lang: &str) -> Option<&str> {
        self.langs.iter().find(|l| *l == lang).map(String::as_str)
    }

    /// The language a path is under (`/ru/anything` → `ru`).
    pub fn lang_of_path(&self, path: &str) -> Option<&str> {
        let first = path.strip_prefix('/')?.split('/').next()?;
        self.lang(first)
    }

    /// `true` if `path` + `/` is a page (`/en` → `/en/`, `/en/sent` → `/en/sent/`).
    pub fn needs_slash(&self, path: &str) -> bool {
        !path.ends_with('/')
            && self.pages.iter().any(|p| p.path.strip_suffix('/') == Some(path))
    }

    /// The 404 page for a language (default language if it has none).
    pub fn not_found(&self, lang: &str) -> Option<&NotFoundPage> {
        self.not_found.get(lang).or_else(|| self.not_found.get(&self.default_lang))
    }

    /// The page route closest to `path` by edit distance (ASCII case-insensitive); ties prefer
    /// pages in `lang`, then manifest order. Nothing if even the closest page differs in more
    /// than half of the characters — a suggestion should be a plausible typo fix, not a guess.
    pub fn suggest(&self, path: &str, lang: &str) -> Option<&PageLink> {
        let probe: Vec<u8> = path.bytes().take(SUGGEST_PROBE_LEN).map(|b| b.to_ascii_lowercase()).collect();
        let mut row = Vec::new();
        self.pages
            .iter()
            .enumerate()
            .map(|(i, page)| (edit_distance(&probe, page.path.as_bytes(), &mut row), i, page))
            .filter(|(distance, _, page)| distance * 2 <= probe.len().max(page.path.len()))
            .min_by_key(|(distance, i, page)| (*distance, page.lang.as_deref() != Some(lang), *i))
            .map(|(_, _, page)| page)
    }

    /// The curl-mode text for `key` in `lang`, falling back to the default language.
    pub fn text(&self, lang: &str, key: &str) -> Option<(&str, &TextTemplate)> {
        [lang, self.default_lang.as_str()].into_iter().find_map(|l| {
            let (l, templates) = self.text.get_key_value(l)?;
            Some((l.as_str(), templates.get(key)?))
        })
    }
}

/// A route's files, read from disk.
struct Files {
    identity: Bytes,
    br: Option<Bytes>,
    gzip: Option<Bytes>,
    /// First 128 bits of SHA-256 of the identity bytes, hex.
    hash: String,
}

impl Files {
    fn read(dist: &Path, file: &str, encodings: &[Encoding]) -> anyhow::Result<Files> {
        let identity = read_file(dist, file)?;
        let digest = Sha256::digest(&identity);
        let hash = digest[..16].iter().map(|b| format!("{b:02x}")).collect();
        let (mut br, mut gzip) = (None, None);
        for encoding in encodings {
            let sibling = format!("{file}{}", encoding.suffix());
            let bytes = Some(leak(read_file(dist, &sibling)?));
            match encoding {
                Encoding::Br => br = bytes,
                Encoding::Gzip => gzip = bytes,
            }
        }
        Ok(Files { identity: leak(identity), br, gzip, hash })
    }

    fn len(&self) -> usize {
        self.identity.len() + self.br.as_ref().map_or(0, Bytes::len) + self.gzip.as_ref().map_or(0, Bytes::len)
    }

    fn variants(
        &self,
        content_type: &str,
        cache: CachePolicy,
        lang: Option<&str>,
        vary: &[&str],
    ) -> anyhow::Result<Variants> {
        let content_type = static_value(content_type.to_owned()).context("content_type")?;
        let cache_control = HeaderValue::from_static(match cache {
            CachePolicy::Document => "no-cache",
            CachePolicy::Immutable => "public, max-age=31536000, immutable",
            CachePolicy::Short => "public, max-age=3600",
        });
        let language = lang.map(|l| static_value(l.to_owned())).transpose().context("lang")?;
        let vary = (!vary.is_empty()).then(|| static_value(vary.join(", "))).transpose()?;
        let variant = |body: &Bytes, coding: Option<&'static str>| -> anyhow::Result<Variant> {
            // A strong validator must differ between representations (RFC 9110 §8.8.1): the hash
            // of the identity bytes, suffixed by the content coding.
            let suffix = match coding {
                Some("br") => "-br",
                Some(_) => "-gz",
                None => "",
            };
            let etag = static_value(format!("\"{}{suffix}\"", self.hash))?;
            let mut not_modified = HeaderMap::with_capacity(4);
            not_modified.insert(ETAG, etag.clone());
            not_modified.insert(CACHE_CONTROL, cache_control.clone());
            if let Some(vary) = &vary {
                not_modified.insert(VARY, vary.clone());
            }
            let mut headers = not_modified.clone();
            headers.insert(CONTENT_TYPE, content_type.clone());
            headers.insert(CONTENT_LENGTH, static_value(body.len().to_string())?);
            if let Some(coding) = coding {
                headers.insert(CONTENT_ENCODING, HeaderValue::from_static(coding));
            }
            if let Some(language) = &language {
                headers.insert(CONTENT_LANGUAGE, language.clone());
            }
            Ok(Variant { body: body.clone(), etag, headers, not_modified })
        };
        Ok(Variants {
            identity: variant(&self.identity, None)?,
            br: self.br.as_ref().map(|b| variant(b, Some("br"))).transpose()?,
            gzip: self.gzip.as_ref().map(|b| variant(b, Some("gzip"))).transpose()?,
        })
    }
}

/// Process-lifetime bytes: clones copy a pointer, with no shared reference count to contend on.
fn leak(bytes: Vec<u8>) -> Bytes {
    Bytes::from_static(bytes.leak())
}

/// Reads `dist/{relative}`, refusing absolute paths and `..`.
fn read_file(dist: &Path, relative: &str) -> anyhow::Result<Vec<u8>> {
    let rel = Path::new(relative);
    ensure!(
        !relative.is_empty() && rel.components().all(|c| matches!(c, Component::Normal(_))),
        "file {relative:?} must be a plain path relative to dist/"
    );
    let path = dist.join(rel);
    std::fs::read(&path).with_context(|| format!("cannot read {}", path.display()))
}

/// `en`, `ru`, `tg`, `pt-BR`: letters, digits and dashes (it ends up in paths and headers).
fn is_lang_tag(s: &str) -> bool {
    (1..=16).contains(&s.len())
        && s.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
        && !s.starts_with('-')
}

/// `/`-rooted, printable ASCII, no query or fragment.
fn is_route_path(s: &str) -> bool {
    s.starts_with('/') && s.bytes().all(|b| b.is_ascii_graphic() && b != b'?' && b != b'#')
}

/// `/{lang}/`, `/{lang}/pillars`, `/{lang}/contact`: answered as text in curl mode.
fn is_curl_page(langs: &[String], path: &str) -> bool {
    langs.iter().any(|lang| {
        path.strip_prefix('/')
            .and_then(|p| p.strip_prefix(lang.as_str()))
            .is_some_and(|rest| CURL_PAGES.contains(&rest))
    })
}

fn escape_html(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            c => out.push(c),
        }
    }
    out
}

/// Levenshtein distance with one reusable row.
fn edit_distance(a: &[u8], b: &[u8], row: &mut Vec<usize>) -> usize {
    row.clear();
    row.extend(0..=b.len());
    for (i, &ca) in a.iter().enumerate() {
        let mut diagonal = row[0];
        row[0] = i + 1;
        for (j, &cb) in b.iter().enumerate() {
            let above = row[j + 1];
            row[j + 1] = if ca == cb { diagonal } else { 1 + diagonal.min(above).min(row[j]) };
            diagonal = above;
        }
    }
    row[b.len()]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn levenshtein() {
        let mut row = Vec::new();
        assert_eq!(edit_distance(b"kitten", b"sitting", &mut row), 3);
        assert_eq!(edit_distance(b"", b"/en/", &mut row), 4);
        assert_eq!(edit_distance(b"/en/snt/", b"/en/sent/", &mut row), 1);
        assert_eq!(edit_distance(b"/en/", b"/en/", &mut row), 0);
    }

    #[test]
    fn templates_substitute_tokens() {
        let t = TextTemplate::parse(b"a {{server_time}} b {{tls}} c {{http}} d {{build}} {{nope}} {{open", "B1");
        let out = t.render("21 µs", "plain HTTP (development)", "HTTP/1.1");
        assert_eq!(
            String::from_utf8(out).unwrap(),
            "a 21 µs b plain HTTP (development) c HTTP/1.1 d B1 {{nope}} {{open"
        );
    }

    #[test]
    fn curl_pages() {
        let langs = ["en".to_string(), "ru".to_string()];
        assert!(is_curl_page(&langs, "/en/"));
        assert!(is_curl_page(&langs, "/ru/pillars"));
        assert!(!is_curl_page(&langs, "/en/sent/"));
        assert!(!is_curl_page(&langs, "/enx/"));
    }

    #[test]
    fn path_checks() {
        assert!(is_route_path("/.well-known/security.txt"));
        assert!(!is_route_path("en/"));
        assert!(!is_route_path("/a b"));
        assert!(!is_route_path("/a?b"));
        assert!(is_lang_tag("pt-BR"));
        assert!(!is_lang_tag("../x"));
        assert!(read_file(Path::new("."), "../etc/passwd").is_err());
        assert!(read_file(Path::new("."), "/etc/passwd").is_err());
    }
}
