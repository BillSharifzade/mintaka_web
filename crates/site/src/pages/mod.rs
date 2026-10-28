//! Page templates (Maud: HTML checked at compile time).

pub mod home;
pub mod misc;
pub mod svg;

use crate::config::SiteConfig;
use crate::i18n::Strings;
use maud::{DOCTYPE, Markup, PreEscaped, html};

/// Everything a template needs.
pub struct Ctx<'a> {
    pub s: &'a Strings,
    pub cfg: &'a SiteConfig,
    pub lang: &'a str,
    pub release: bool,
    pub build: &'a str,
    pub year: i64,
    /// Minified page CSS, inlined in `<style>` (its hash is in the CSP).
    pub css: &'a str,
    pub main_js: &'a str,
    pub font_preloads: Vec<&'a str>,
    /// `<script type="application/json" id="mintaka-i18n">` payload (already HTML-safe).
    pub island_json: &'a str,
    /// `<script type="application/json" id="mintaka-assets">` payload (already HTML-safe).
    pub assets_json: &'a str,
    /// Pre-rendered brand SVGs: the symbol sprite (once per page), the logo `<use>`, the mark.
    pub sprite: &'a Markup,
    pub logo: &'a Markup,
    pub mark: &'a Markup,
    /// Date of the OpenStreetMap extract behind the routing race (`2026-10-28`), if built.
    pub osm_date: &'a str,
}

impl Ctx<'_> {
    /// Home page path for this language: `/en/`.
    pub fn home(&self) -> String {
        format!("/{}/", self.lang)
    }

    pub fn abs(&self, path: &str) -> String {
        format!("{}{}", self.cfg.origin, path)
    }
}

pub struct Page<'a> {
    pub path: &'a str,
    pub title: String,
    pub description: String,
    pub is_home: bool,
    pub body: Markup,
    /// Index the page (404 and result pages are noindex).
    pub index: bool,
}

/// The view-source greeting (spec §7.5): the first thing an engineer reading our HTML sees.
fn source_greeting(cfg: &SiteConfig) -> String {
    format!(
        "<!--\n      ●            Mintaka · software at the edge of physics\n       ╲   ·\n        ╲ ·  ●     You read page source. We'd like to meet you:\n    · ·  ●         {}\n        ╱    ╲\n       ●      ●    Press ~ on the page for a terminal.\n-->\n",
        cfg.contact.careers_email
    )
}

pub fn document(ctx: &Ctx, page: Page) -> Markup {
    let s = ctx.s;
    let og_locale = match ctx.lang {
        "ru" => "ru_RU",
        "tg" => "tg_TJ",
        _ => "en_US",
    };
    let nav_href = |id: &str| if page.is_home { format!("#{id}") } else { format!("{}#{id}", ctx.home()) };
    html! {
        (DOCTYPE)
        (PreEscaped(source_greeting(ctx.cfg)))
        html lang=(ctx.lang) dir="ltr" {
            head {
                meta charset="utf-8";
                meta name="viewport" content="width=device-width, initial-scale=1, viewport-fit=cover";
                title { (page.title) }
                meta name="description" content=(page.description);
                meta name="color-scheme" content="dark";
                meta name="theme-color" content="#0A0B0D";
                @if !page.index { meta name="robots" content="noindex"; }
                link rel="canonical" href=(ctx.abs(page.path));
                @if page.is_home {
                    @for l in &ctx.cfg.langs {
                        link rel="alternate" hreflang=(l) href=(ctx.abs(&format!("/{l}/")));
                    }
                    link rel="alternate" hreflang="x-default" href=(ctx.abs("/"));
                }
                @for url in &ctx.font_preloads {
                    link rel="preload" href=(url) as="font" type="font/woff2" crossorigin;
                }
                link rel="icon" href="/favicon.ico" sizes="any";
                link rel="icon" href="/favicon.svg" type="image/svg+xml";
                link rel="apple-touch-icon" href="/apple-touch-icon.png";
                link rel="manifest" href="/manifest.webmanifest";
                meta property="og:type" content="website";
                meta property="og:site_name" content="Mintaka";
                meta property="og:title" content=(page.title);
                meta property="og:description" content=(page.description);
                meta property="og:url" content=(ctx.abs(page.path));
                meta property="og:image" content=(ctx.abs("/og-image.png"));
                meta property="og:image:width" content="1200";
                meta property="og:image:height" content="630";
                meta property="og:locale" content=(og_locale);
                meta name="twitter:card" content="summary_large_image";
                style { (PreEscaped(ctx.css)) }
                script type="module" src=(ctx.main_js) {}
            }
            body {
                svg class="sprite" aria-hidden="true" focusable="false" { (ctx.sprite) }
                a class="skip-link" href="#main" { (s.t("skip-link")) }
                header class="site-header" {
                    div class="wrap site-header__inner" {
                        a class="site-header__logo" href=(ctx.home()) aria-label="Mintaka" { (ctx.logo) }
                        nav class="site-nav" aria-label=(s.t("nav-label")) {
                            ul role="list" {
                                li { a href=(nav_href("efficiency")) { (s.t("nav-efficiency")) } }
                                li { a href=(nav_href("security")) { (s.t("nav-security")) } }
                                li { a href=(nav_href("highload")) { (s.t("nav-highload")) } }
                                li { a href=(nav_href("design")) { (s.t("nav-design")) } }
                                li { a href=(nav_href("process")) { (s.t("nav-process")) } }
                                li { a href=(nav_href("team")) { (s.t("nav-team")) } }
                            }
                        }
                        (lang_switch(ctx))
                        a class="btn btn--primary btn--sm site-header__cta" href=(nav_href("contact")) { (s.t("nav-cta")) }
                        details class="menu" {
                            summary class="btn btn--ghost btn--sm menu__toggle" { (s.t("nav-menu")) }
                            nav class="menu__panel" aria-label=(s.t("nav-label")) {
                                ul role="list" {
                                    li { a href=(nav_href("efficiency")) { (s.t("nav-efficiency")) } }
                                    li { a href=(nav_href("security")) { (s.t("nav-security")) } }
                                    li { a href=(nav_href("highload")) { (s.t("nav-highload")) } }
                                    li { a href=(nav_href("design")) { (s.t("nav-design")) } }
                                    li { a href=(nav_href("process")) { (s.t("nav-process")) } }
                                    li { a href=(nav_href("team")) { (s.t("nav-team")) } }
                                    li { a href=(nav_href("contact")) { (s.t("nav-cta")) } }
                                }
                            }
                        }
                    }
                }
                @if page.is_home { (svg::orion_nav(s)) }
                @if page.is_home { canvas id="hero-canvas" aria-hidden="true" {} }
                main id="main" tabindex="-1" { (page.body) }
                (footer(ctx, page.path))
                script type="application/json" id="mintaka-i18n" { (PreEscaped(ctx.island_json)) }
                script type="application/json" id="mintaka-assets" { (PreEscaped(ctx.assets_json)) }
            }
        }
    }
}

fn lang_switch(ctx: &Ctx) -> Markup {
    if ctx.cfg.langs.len() < 2 {
        return html! {};
    }
    html! {
        nav class="lang" aria-label=(ctx.s.t("lang-switch")) {
            @for l in &ctx.cfg.langs {
                a href=(format!("/{l}/")) hreflang=(l) lang=(l) aria-current=[(l == ctx.lang).then_some("page")] {
                    (ctx.s.t(&format!("lang-name-{l}")))
                }
            }
        }
    }
}

/// The footer is drawn as an engineering drawing's title block.
fn footer(ctx: &Ctx, path: &str) -> Markup {
    let s = ctx.s;
    html! {
        footer class="site-footer" {
            div class="wrap" {
                div class="titleblock" {
                    div class="titleblock__brand" {
                        a href=(ctx.home()) aria-label="Mintaka" { (ctx.logo) }
                        p class="titleblock__tagline" { (s.t("ft-tagline")) }
                        p class="titleblock__made" { (s.t("ft-made")) }
                    }
                    dl class="titleblock__cells" {
                        div { dt { (s.t("ft-sheet")) } dd class="mono" { (ctx.cfg.origin.trim_start_matches("https://")) (path) } }
                        div { dt { (s.t("ft-build")) } dd class="mono" { (ctx.build) } }
                        div { dt { (s.t("ft-checked")) } dd { (s.t("ft-checked-value")) } }
                        div class="titleblock__live" {
                            dt { (s.t("ft-served")) }
                            dd { p id="footer-stats" class="mono" aria-live="off" { (s.t("ft-served-fallback")) } }
                        }
                    }
                }
                div class="site-footer__cols" {
                    section class="promises" aria-labelledby="promises-title" {
                        h2 id="promises-title" class="site-footer__heading" { (s.t("ft-promises")) }
                        ul class="promises__list" role="list" {
                            @for i in 1..=5 { li { (s.t(&format!("ft-promise-{i}"))) } }
                        }
                    }
                    div class="site-footer__links" {
                        ul role="list" {
                            li { a href=(format!("{}privacy/", ctx.home())) { (s.t("ft-privacy")) } }
                            li { a href="/.well-known/security.txt" { (s.t("ft-security")) } }
                            li { a href="/humans.txt" { (s.t("ft-humans")) } }
                        }
                        p class="site-footer__tools" {
                            button class="btn btn--quiet" type="button" data-action="terminal" { (s.t("ft-terminal")) " " kbd { "~" } }
                            button class="btn btn--quiet" type="button" data-action="shortcuts" aria-pressed="true" {
                                (s.t("ft-shortcuts"))
                                span class="visually-hidden" { ": " (s.t("ft-shortcuts-keys")) }
                            }
                        }
                        p class="site-footer__copy" { (s.ta("ft-copyright", &[("year", ctx.year.into())])) }
                    }
                }
            }
        }
    }
}

/// A section heading block: star callout on the left, statement on the right.
pub fn pillar_head(ctx: &Ctx, star: &str, id: &str, eyebrow_key: &str, title_key: &str, lead_key: &str) -> Markup {
    let s = ctx.s;
    let st = crate::orion::star(star);
    html! {
        header class="sec__head sec__head--pillar" {
            div class="star-callout" {
                (svg::star_mark(star))
                p class="star-callout__name" { (s.t(&format!("star-{star}"))) }
                p class="star-callout__xy mono" { (st.ra_label()) br; (st.dec_label()) }
                p class="star-callout__fact" { (s.t(&format!("star-{star}-fact"))) }
            }
            div class="sec__statement" {
                p class="eyebrow" { (s.t(eyebrow_key)) }
                h2 class="sec__title" id=(format!("{id}-title")) { (s.t(title_key)) }
                p class="lead" { (s.t(lead_key)) }
            }
        }
    }
}

/// An instrument frame (CONTRACTS §9) with a static fallback body.
pub fn instrument(island: &str, id_text: &str, title: &str, status: &str, body: Markup, foot: Option<Markup>) -> Markup {
    let title_id = format!("{island}-instrument-title");
    html! {
        div class="instrument" role="region" aria-labelledby=(title_id) data-island=(island) {
            div class="instrument__bar" {
                span class="instrument__id" { (id_text) }
                h3 class="instrument__title" id=(title_id) { (title) }
                span class="instrument__status badge badge--plain" { (status) }
            }
            div class="instrument__body" { (body) }
            @if let Some(f) = foot { p class="instrument__foot" { (f) } }
        }
    }
}
