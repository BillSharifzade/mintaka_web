//! `dist/manifest.json`: the contract between the site generator (writer) and the server (reader).
//!
//! The generator owns every byte the server sends for static routes. The server never touches the
//! file system per request: it loads every route listed here into memory at startup, including the
//! precompressed siblings named in [`Route::encodings`].

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Bumped whenever the shape below changes incompatibly.
pub const MANIFEST_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Manifest {
    pub version: u32,
    /// Build identifier, e.g. `2026-10-27.3f2a1b9c` (date + content hash prefix).
    pub build: String,
    /// Canonical origin, e.g. `https://mintaka.tj`. Used for absolute URLs only.
    pub origin: String,
    /// Language served at `/` when `Accept-Language` matches nothing we have.
    pub default_lang: String,
    /// Enabled languages, in display order. Each has `/{lang}/` and a 404 page.
    pub langs: Vec<String>,
    pub csp: Csp,
    pub routes: Vec<Route>,
    /// `lang -> file` (relative to `dist/`) for the 404 page of each language.
    pub not_found: BTreeMap<String, String>,
    /// curl mode: `lang -> (url path -> file)`. Files are UTF-8 text with ANSI colour escapes and
    /// `{{placeholder}}` tokens the server fills per request (see docs/CONTRACTS.md §curl).
    pub text: BTreeMap<String, BTreeMap<String, String>>,
}

/// Hash sources for the Content-Security-Policy header, without quotes: `sha256-<base64>`.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct Csp {
    pub style_src: Vec<String>,
    pub script_src: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Route {
    /// Exact URL path, e.g. `/en/`, `/assets/main.1a2b3c4d.js`, `/.well-known/security.txt`.
    pub path: String,
    /// File relative to `dist/`, e.g. `en/index.html`.
    pub file: String,
    /// Full `Content-Type` value, e.g. `text/html; charset=utf-8`.
    pub content_type: String,
    pub cache: CachePolicy,
    /// Precompressed siblings that exist next to `file` (`file.br`, `file.gz`).
    #[serde(default)]
    pub encodings: Vec<Encoding>,
    /// Set for pages: the page language (sent as `Content-Language`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lang: Option<String>,
    pub kind: RouteKind,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "kebab-case")]
pub enum CachePolicy {
    /// HTML: `Cache-Control: no-cache` + strong ETag, so every view revalidates (cheap 304s).
    Document,
    /// Content-hashed assets: `Cache-Control: public, max-age=31536000, immutable`.
    Immutable,
    /// Unhashed files (favicon, robots.txt…): `Cache-Control: public, max-age=3600`.
    Short,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "kebab-case")]
pub enum Encoding {
    /// `file.br`
    Br,
    /// `file.gz`
    Gzip,
}

impl Encoding {
    pub fn suffix(self) -> &'static str {
        match self {
            Encoding::Br => ".br",
            Encoding::Gzip => ".gz",
        }
    }
    pub fn content_encoding(self) -> &'static str {
        match self {
            Encoding::Br => "br",
            Encoding::Gzip => "gzip",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "kebab-case")]
pub enum RouteKind {
    /// An HTML page a person navigates to (candidate for 404 suggestions).
    Page,
    /// A hashed build asset under `/assets/`.
    Asset,
    /// Any other static file: icons, robots.txt, sitemap.xml, .well-known/*.
    File,
}

impl Manifest {
    pub fn route(&self, path: &str) -> Option<&Route> {
        self.routes.iter().find(|r| r.path == path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_through_json() {
        let m = Manifest {
            version: MANIFEST_VERSION,
            build: "2026-10-27.deadbeef".into(),
            origin: "https://mintaka.tj".into(),
            default_lang: "en".into(),
            langs: vec!["en".into()],
            csp: Csp { style_src: vec!["sha256-abc=".into()], script_src: vec![] },
            routes: vec![Route {
                path: "/en/".into(),
                file: "en/index.html".into(),
                content_type: "text/html; charset=utf-8".into(),
                cache: CachePolicy::Document,
                encodings: vec![Encoding::Br, Encoding::Gzip],
                lang: Some("en".into()),
                kind: RouteKind::Page,
            }],
            not_found: [("en".to_string(), "en/404.html".to_string())].into(),
            text: Default::default(),
        };
        let json = serde_json::to_string(&m).unwrap();
        assert!(json.contains(r#""cache":"document""#));
        assert!(json.contains(r#""encodings":["br","gzip"]"#));
        assert_eq!(serde_json::from_str::<Manifest>(&json).unwrap(), m);
    }
}
