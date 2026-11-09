//! Page templates (Maud: HTML checked at compile time).

pub mod home;
pub mod lab;
pub mod landing;
pub mod misc;
pub mod svg;

use crate::config::SiteConfig;
use crate::i18n::Strings;
use maud::{DOCTYPE, Markup, PreEscaped, html};
use std::collections::BTreeMap;

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
    /// Team portraits by member id.
    pub portraits: &'a BTreeMap<String, Portrait>,
    /// This language's Lab posts, newest first (spec §S11).
    pub lab: &'a [crate::lab::Post],
    /// For a language without posts of its own: the default language and its posts, which the
    /// Lab section and index list instead, marked with their language.
    pub lab_fallback: Option<(&'a str, &'a [crate::lab::Post])>,
}

/// Hashed URLs of one team portrait.
pub struct Portrait {
    pub avif: String,
    pub webp: String,
}

impl Ctx<'_> {
    /// Link to this language's home page: `/en/` (`/mintaka/en/` under a URL prefix).
    pub fn home(&self) -> String {
        self.url(&format!("/{}/", self.lang))
    }

    /// A path of this site (`/en/privacy/`) as a link, with the site's URL prefix.
    pub fn url(&self, path: &str) -> String {
        self.cfg.url(path)
    }

    /// The same link, absolute (canonical URLs, `hreflang`, Open Graph).
    pub fn abs(&self, path: &str) -> String {
        self.cfg.abs(path)
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
    /// Set for long-form pages (the Lab): see [`LongForm`]. `None` everywhere else.
    pub long_form: Option<LongForm<'a>>,
}

/// What a long-form page (spec §S11) changes in the document. It ships **no JavaScript at all**
/// (spec §9.5): no boot script, no island data, and no footer controls that need a script. It
/// inlines the long-form stylesheet instead of the page CSS (its hash is in the CSP too).
pub struct LongForm<'a> {
    /// Minified CSS for the page's one `<style>`.
    pub css: &'a str,
    /// `website` or `article`.
    pub og_type: &'static str,
    /// Extra `<head>` elements (article metadata).
    pub head: Markup,
    /// The same page in each language that has it: `(lang, path)`. The language switch and the
    /// `hreflang` alternates link there; a language without it links to its home page.
    pub alternates: Vec<(String, String)>,
}

/// The view-source greeting (spec §7.5): the first thing an engineer reading our HTML sees.
/// Its second line is also the clue to the console challenge's second stage (`proof::CTF_CRIB`).
fn source_greeting(cfg: &SiteConfig, scripts: bool) -> String {
    let last = if scripts { "Press ~ on the page for a terminal." } else { "This page runs no JavaScript at all." };
    format!(
        "<!--\n      ●            Mintaka · software at the edge of physics\n       ╲   ·\n        ╲ ·  ●     {}\n    · ·  ●         {}\n        ╱    ╲\n       ●      ●    {last}\n-->\n",
        proof::CTF_CRIB,
        cfg.contact.careers().1
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
    let long = page.long_form.as_ref();
    let scripts = long.is_none();
    let alternates = long.map_or(&[][..], |l| l.alternates.as_slice());
    // A page rendered as one variant of the A/B test says which (docs/CONTRACTS.md §17), and which
    // events its beacons report as conversions.
    let variant = ctx.s.variant();
    let converts = variant.as_ref().and(ctx.cfg.experiment.as_ref()).map(|e| e.metrics().collect::<Vec<_>>().join(" "));
    html! {
        (DOCTYPE)
        (PreEscaped(source_greeting(ctx.cfg, scripts)))
        html lang=(ctx.lang) dir="ltr" data-variant=[variant] data-converts=[converts] {
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
                } @else if alternates.len() > 1 {
                    @for (l, path) in alternates {
                        link rel="alternate" hreflang=(l) href=(ctx.abs(path));
                    }
                }
                @if !ctx.lab.is_empty() {
                    link rel="alternate" type="application/atom+xml" title=(s.t("lab-feed-title")) href=(ctx.url(&crate::lab::feed_path(ctx.lang)));
                }
                @for url in &ctx.font_preloads {
                    link rel="preload" href=(url) as="font" type="font/woff2" crossorigin;
                }
                link rel="icon" href=(ctx.url("/favicon.ico")) sizes="any";
                link rel="icon" href=(ctx.url("/favicon.svg")) type="image/svg+xml";
                link rel="apple-touch-icon" href=(ctx.url("/apple-touch-icon.png"));
                link rel="manifest" href=(ctx.url("/manifest.webmanifest"));
                meta property="og:type" content=(long.map_or("website", |l| l.og_type));
                meta property="og:site_name" content="Mintaka";
                meta property="og:title" content=(page.title);
                meta property="og:description" content=(page.description);
                meta property="og:url" content=(ctx.abs(page.path));
                meta property="og:image" content=(ctx.abs("/og-image.png"));
                meta property="og:image:width" content="1200";
                meta property="og:image:height" content="630";
                meta property="og:locale" content=(og_locale);
                meta name="twitter:card" content="summary_large_image";
                @if let Some(l) = long { (l.head) }
                style { (PreEscaped(long.map_or(ctx.css, |l| l.css))) }
                @if scripts { script type="module" src=(ctx.main_js) {} }
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
                        (lang_switch(ctx, alternates, true))
                        a class="btn btn--primary btn--sm site-header__cta" href=(nav_href("contact")) { (s.t("nav-cta")) }
                        details class="menu" {
                            summary class="btn btn--ghost btn--sm menu__toggle" { (s.t("nav-menu")) }
                            // Small screens: the sections and, below them, the language switch,
                            // which has no room in the header there.
                            // Named after the menu, and its language switch a group, not a second
                            // landmark: the header's own navigation and switch keep their names.
                            div class="menu__panel" {
                                nav aria-label=(s.t("nav-menu")) {
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
                                (lang_switch(ctx, alternates, false))
                            }
                        }
                    }
                }
                @if page.is_home { (svg::orion_nav(s)) }
                @if page.is_home { canvas id="hero-canvas" aria-hidden="true" {} }
                main id="main" tabindex="-1" { (page.body) }
                (footer(ctx, page.path, scripts))
                @if scripts {
                    script type="application/json" id="mintaka-i18n" { (PreEscaped(ctx.island_json)) }
                    script type="application/json" id="mintaka-assets" { (PreEscaped(ctx.assets_json)) }
                }
            }
        }
    }
}

/// Links to this page in each language (`alternates`), else to that language's home page. The
/// header's copy is a navigation landmark; the menu's copy (`landmark: false`) a named group.
fn lang_switch(ctx: &Ctx, alternates: &[(String, String)], landmark: bool) -> Markup {
    if ctx.cfg.langs.len() < 2 {
        return html! {};
    }
    let href = |l: &str| ctx.url(&alternates.iter().find(|(a, _)| a == l).map_or_else(|| format!("/{l}/"), |(_, p)| p.clone()));
    let links = html! {
        @for l in &ctx.cfg.langs {
            a href=(href(l)) hreflang=(l) lang=(l) aria-current=[(l == ctx.lang).then_some("page")] {
                (ctx.s.t(&format!("lang-name-{l}")))
            }
        }
    };
    html! {
        @if landmark {
            nav class="lang" aria-label=(ctx.s.t("lang-switch")) { (links) }
        } @else {
            div class="lang" role="group" aria-label=(ctx.s.t("lang-switch")) { (links) }
        }
    }
}

/// The footer is drawn as an engineering drawing's title block. Without scripts (long-form
/// pages) the live cell says why nothing is measured, and the script-driven controls are left out.
fn footer(ctx: &Ctx, path: &str, scripts: bool) -> Markup {
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
                        div { dt { (s.t("ft-sheet")) } dd class="mono" { (ctx.cfg.origin.trim_start_matches("https://")) (ctx.cfg.base) (path) } }
                        div { dt { (s.t("ft-build")) } dd class="mono" { (ctx.build) } }
                        div { dt { (s.t("ft-checked")) } dd { (s.t("ft-checked-value")) } }
                        div class="titleblock__live" {
                            dt { (s.t("ft-served")) }
                            @if scripts {
                                dd { p id="footer-stats" class="mono" aria-live="off" { (s.t("ft-served-fallback")) } }
                            } @else {
                                dd { p { (s.t("ft-served-static")) } }
                            }
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
                            li { a href=(ctx.url("/.well-known/security.txt")) { (s.t("ft-security")) } }
                            li { a href=(ctx.url("/humans.txt")) { (s.t("ft-humans")) } }
                        }
                        @if scripts {
                            p class="site-footer__tools" {
                                button class="btn btn--quiet" type="button" data-action="terminal" { (s.t("ft-terminal")) " " kbd { "~" } }
                                button class="btn btn--quiet" type="button" data-action="shortcuts" aria-pressed="true" {
                                    (s.t("ft-shortcuts"))
                                    span class="visually-hidden" { ": " (s.t("ft-shortcuts-keys")) }
                                }
                            }
                        }
                        p class="site-footer__copy" { (s.ta("ft-copyright", &[("year", ctx.year.into())])) }
                    }
                }
            }
        }
    }
}

/// An experiment's head on the bench, laid out like the top of a lab record: a rule, then the star
/// it is filed under with its catalogue position and the pillar's name; under it the claim, large,
/// and beside it the lead and the star's one-line story.
pub fn pillar_head(ctx: &Ctx, star: &str, id: &str, eyebrow_key: &str, title_key: &str, lead_key: &str) -> Markup {
    let s = ctx.s;
    let st = crate::orion::star(star);
    html! {
        header class="exp__head" {
            div class="exp__band" {
                (svg::star_mark(star))
                p class="exp__star" {
                    span class="exp__name" { (s.t(&format!("star-{star}"))) }
                    " "
                    span class="exp__xy" { (st.ra_label()) " · " (st.dec_label()) }
                }
                p class="exp__pillar" { (s.t(eyebrow_key)) }
            }
            h2 class="exp__title" id=(format!("{id}-title")) { (s.t(title_key)) }
            p class="lead exp__lead" { (s.t(lead_key)) }
        }
    }
}

/// An instrument frame (CONTRACTS §9) with a static fallback body. The frame is a tab stop until
/// its island has mounted (`web/main.ts`): an island mounts only when it scrolls near, so without
/// it Tab would jump past every instrument the visitor has not scrolled to yet.
pub fn instrument(island: &str, id_text: &str, title: &str, status: &str, body: Markup, foot: Option<Markup>) -> Markup {
    let title_id = format!("{island}-instrument-title");
    html! {
        div class="instrument" role="region" aria-labelledby=(title_id) data-island=(island) tabindex="0" {
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
