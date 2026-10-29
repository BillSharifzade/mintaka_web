//! robots.txt, humans.txt, sitemap.xml, security.txt (RFC 9116), the web app manifest.

use crate::config::SiteConfig;
use crate::util::civil_from_days;

pub fn robots(cfg: &SiteConfig) -> String {
    format!(
        "#        ●            mintaka — software at the edge of physics\n#   ·  ·   ●\n#      ● ·\n\nUser-agent: *\nAllow: /\nDisallow: /api/\nDisallow: /_\n{}\nSitemap: {}/sitemap.xml\n",
        cfg.langs.iter().map(|l| format!("Disallow: /{l}/sent/")).collect::<Vec<_>>().join("\n"),
        cfg.origin
    )
}

/// `people`: one `(role, name, github, site)` per team member, in the default language.
pub fn humans(cfg: &SiteConfig, date: &str, people: &[(String, String, String, String)]) -> String {
    let mut team = String::new();
    for (role, name, github, site) in people {
        team += &format!("\n  {role}: {name}\n  GitHub: https://github.com/{github}\n");
        if !site.is_empty() {
            team += &format!("  Site: {site}\n");
        }
    }
    format!(
        "/* TEAM */\n  Studio: {name}\n  Location: {city}, {country}\n  Contact: {email}\n  Careers: {careers}\n{team}\n/* THANKS */\n  The IANA time zone database, OpenStreetMap contributors, the Rust project.\n\n/* SITE */\n  Last update: {date}\n  Language: {langs}\n  Standards: HTML, CSS, WCAG 2.2, RFC 9116\n  Components: Rust (axum, hyper, rustls, maud, fluent, lightningcss, resvg), TypeScript with no runtime dependencies\n  Fonts: Space Grotesk, Onest, IBM Plex Mono (SIL Open Font License 1.1)\n",
        name = cfg.company.name,
        city = cfg.company.city,
        country = cfg.company.country,
        email = cfg.contact.email,
        careers = cfg.contact.careers_email,
        langs = cfg.langs.join(", "),
    )
}

/// A page that exists in some languages only (the Lab): its path after `/{lang}`, the languages
/// that have it, and the date its content last changed.
pub struct SitemapUrl {
    pub path: String,
    pub langs: Vec<String>,
    pub lastmod: String,
}

/// `pages`: paths without language prefix, e.g. `/` and `/privacy/`, in every language and dated
/// by the build. `extra`: pages with their own languages and dates.
pub fn sitemap(cfg: &SiteConfig, pages: &[&str], extra: &[SitemapUrl], date: &str) -> String {
    let mut out = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\" xmlns:xhtml=\"http://www.w3.org/1999/xhtml\">\n",
    );
    for page in pages {
        for lang in &cfg.langs {
            out += &format!("  <url>\n    <loc>{}/{lang}{page}</loc>\n    <lastmod>{date}</lastmod>\n", cfg.origin);
            for alt in &cfg.langs {
                out += &format!("    <xhtml:link rel=\"alternate\" hreflang=\"{alt}\" href=\"{}/{alt}{page}\"/>\n", cfg.origin);
            }
            if *page == "/" {
                out += &format!("    <xhtml:link rel=\"alternate\" hreflang=\"x-default\" href=\"{}/\"/>\n", cfg.origin);
            }
            out += "  </url>\n";
        }
    }
    for url in extra {
        for lang in &url.langs {
            out += &format!("  <url>\n    <loc>{}/{lang}{}</loc>\n    <lastmod>{}</lastmod>\n", cfg.origin, url.path, url.lastmod);
            for alt in &url.langs {
                out += &format!("    <xhtml:link rel=\"alternate\" hreflang=\"{alt}\" href=\"{}/{alt}{}\"/>\n", cfg.origin, url.path);
            }
            out += "  </url>\n";
        }
    }
    out + "</urlset>\n"
}

/// RFC 9116. `Expires` is set 360 days after the build, so a forgotten deploy never serves an
/// expired file for long — rebuild at least yearly.
pub fn security_txt(cfg: &SiteConfig, days_since_epoch: i64) -> String {
    let (y, m, d) = civil_from_days(days_since_epoch + 360);
    let langs: Vec<&str> = ["en", "ru", "tg"].into_iter().collect();
    format!(
        "# Security contact for {origin}. We welcome reports and reply in {langs}.\nContact: mailto:{email}\nExpires: {y:04}-{m:02}-{d:02}T00:00:00.000Z\nPreferred-Languages: {langs}\nCanonical: {origin}/.well-known/security.txt\n",
        origin = cfg.origin,
        email = cfg.contact.security_email,
        langs = langs.join(", "),
    )
}

pub fn webmanifest(cfg: &SiteConfig) -> String {
    serde_json::json!({
        "name": cfg.company.name,
        "short_name": cfg.company.name,
        "start_url": "/",
        "display": "browser",
        "background_color": "#0A0B0D",
        "theme_color": "#0A0B0D",
        "icons": [
            { "src": "/icon-192.png", "sizes": "192x192", "type": "image/png" },
            { "src": "/icon-512.png", "sizes": "512x512", "type": "image/png" },
            { "src": "/icon-512.svg", "sizes": "any", "type": "image/svg+xml" }
        ]
    })
    .to_string()
}
