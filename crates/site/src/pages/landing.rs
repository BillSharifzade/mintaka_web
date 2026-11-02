//! The studio landing page (`/{lang}/studio/`): the company's front door. A distinct, lean page
//! — its own small stylesheet, no islands and no JavaScript at all (like the Lab, spec §9.5) —
//! that links to the lab for the proof. Signature: Orion's Belt, drawn from the real star
//! coordinates (`crate::orion`), with the company's star, Mintaka, picked out.

use maud::{DOCTYPE, Markup, PreEscaped, html};

use crate::config::SiteConfig;
use crate::i18n::Strings;
use crate::orion;

/// Everything the landing template needs. Deliberately small: no island data, no main script.
pub struct Landing<'a> {
    pub cfg: &'a SiteConfig,
    pub s: &'a Strings,
    pub lang: &'a str,
    pub build: &'a str,
    pub year: i64,
    /// The brand logo `<use>` markup (built once in the generator).
    pub logo: &'a Markup,
    /// The `<symbol>` sprite the logo `<use>` resolves against (rendered once, hidden).
    pub sprite: &'a Markup,
    /// Preloaded font URLs for this language.
    pub font_preloads: Vec<&'a str>,
    /// Minified landing CSS, inlined in the one `<style>` (its hash is in the CSP).
    pub css: &'a str,
}

impl Landing<'_> {
    fn url(&self, path: &str) -> String {
        self.cfg.url(path)
    }
    fn abs(&self, path: &str) -> String {
        self.cfg.abs(path)
    }
    /// This page, in a given language: `/{lang}/studio/`.
    fn studio_in(&self, lang: &str) -> String {
        self.url(&format!("/{lang}/studio/"))
    }
    /// The lab home (and its anchors) in this language.
    fn lab(&self) -> String {
        self.url(&format!("/{}/", self.lang))
    }
    fn lab_at(&self, anchor: &str) -> String {
        self.url(&format!("/{}/{anchor}", self.lang))
    }
}

/// A small north-east arrow for links that leave for the lab. Inline SVG, not a font glyph, so it
/// never depends on font coverage.
fn arrow() -> Markup {
    html! {
        svg class="arrow" viewBox="0 0 16 16" aria-hidden="true" focusable="false" {
            path d="M4.5 11.5 L11.5 4.5 M6 4.5 H11.5 V10" {}
        }
    }
}

pub fn page(l: &Landing) -> Markup {
    let s = l.s;
    let og_locale = match l.lang {
        "ru" => "ru_RU",
        "tg" => "tg_TJ",
        _ => "en_US",
    };
    let title = s.t("landing-meta-title");
    let description = s.t("landing-meta-description");
    let canonical = l.abs(&format!("/{}/studio/", l.lang));
    // The view-source greeting an engineer reading our HTML meets first (no-JavaScript variant).
    let greeting = format!(
        "<!--\n      ●            Mintaka · software at the edge of physics\n       ╲   ·\n        ╲ ·  ●     A studio that measures its own claims.\n    · ·  ●         This page runs no JavaScript at all.\n        ╱    ╲\n       ●      ●    The lab proves the rest: {}\n-->\n",
        l.lab()
    );

    html! {
        (DOCTYPE)
        (PreEscaped(greeting))
        html lang=(l.lang) dir="ltr" {
            head {
                meta charset="utf-8";
                meta name="viewport" content="width=device-width, initial-scale=1, viewport-fit=cover";
                title { (title) }
                meta name="description" content=(description);
                meta name="color-scheme" content="dark";
                meta name="theme-color" content="#0A0B0D";
                link rel="canonical" href=(canonical);
                @for al in &l.cfg.langs {
                    link rel="alternate" hreflang=(al) href=(l.abs(&format!("/{al}/studio/")));
                }
                link rel="alternate" hreflang="x-default" href=(l.abs(&format!("/{}/studio/", l.cfg.default_lang)));
                @for url in &l.font_preloads {
                    link rel="preload" href=(url) as="font" type="font/woff2" crossorigin;
                }
                link rel="icon" href=(l.url("/favicon.ico")) sizes="any";
                link rel="icon" href=(l.url("/favicon.svg")) type="image/svg+xml";
                link rel="apple-touch-icon" href=(l.url("/apple-touch-icon.png"));
                link rel="manifest" href=(l.url("/manifest.webmanifest"));
                meta property="og:type" content="website";
                meta property="og:site_name" content="Mintaka";
                meta property="og:title" content=(title);
                meta property="og:description" content=(description);
                meta property="og:url" content=(canonical);
                meta property="og:image" content=(l.abs("/og-image.png"));
                meta property="og:image:width" content="1200";
                meta property="og:image:height" content="630";
                meta property="og:locale" content=(og_locale);
                meta name="twitter:card" content="summary_large_image";
                style { (PreEscaped(l.css)) }
            }
            body {
                svg class="lp-sprite" aria-hidden="true" focusable="false" { (l.sprite) }
                a class="lp-skip" href="#main" { (s.t("skip-link")) }
                (header(l))
                main id="main" tabindex="-1" {
                    (hero(l))
                    (pillars(l))
                    (method(l))
                    (lab_callout(l))
                    (contact(l))
                }
                (footer(l))
            }
        }
    }
}

fn header(l: &Landing) -> Markup {
    let s = l.s;
    html! {
        header class="lp-header" {
            div class="lp-wrap lp-header__inner" {
                a class="lp-logo" href=(l.studio_in(l.lang)) aria-label="Mintaka" { (l.logo) }
                nav class="lp-nav" aria-label=(s.t("nav-label")) {
                    a href=(l.lab()) { (s.t("landing-to-lab")) }
                    @if l.cfg.langs.len() > 1 {
                        span class="lp-lang" {
                            @for al in &l.cfg.langs {
                                a href=(l.studio_in(al)) hreflang=(al) lang=(al)
                                  aria-current=[(al == l.lang).then_some("page")] {
                                    (s.t(&format!("lang-name-{al}")))
                                }
                            }
                        }
                    }
                }
                a class="btn btn--cta btn--sm" href="#start" { (s.t("nav-cta")) }
            }
        }
    }
}

fn hero(l: &Landing) -> Markup {
    let s = l.s;
    let mintaka = orion::star("mintaka");
    html! {
        section class="lp-hero" {
            div class="lp-wrap lp-hero__grid" {
                div {
                    p class="lp-eyebrow lp-reveal lp-reveal--1" { (s.t("hero-eyebrow")) }
                    h1 class="lp-h1 lp-reveal lp-reveal--2" { (s.t("hero-title")) }
                    p class="lp-hero__lead lp-reveal lp-reveal--3" { (s.t("landing-hero-lead")) }
                    div class="lp-cta-row lp-reveal lp-reveal--4" {
                        a class="btn btn--cta btn--lg" href="#start" { (s.t("nav-cta")) }
                        a class="btn btn--ghost btn--lg" href=(l.lab()) {
                            (s.t("landing-hero-lab")) (arrow())
                        }
                    }
                }
                figure class="belt lp-reveal lp-reveal--3" {
                    svg class="belt__fig" viewBox="0 0 320 180" role="img" aria-labelledby="belt-title" {
                        title id="belt-title" { "Orion's Belt: Alnitak, Alnilam and Mintaka, from their real sky coordinates." }
                        polyline class="belt__line" points="46,128 160,92 274,56" {}
                        g class="belt__star belt__star--1" {
                            circle class="belt__dot" cx="46" cy="128" r="5" {}
                            text class="belt__label" x="46" y="152" { "Alnitak" }
                        }
                        g class="belt__star belt__star--2" {
                            circle class="belt__dot" cx="160" cy="92" r="5" {}
                            text class="belt__label" x="160" y="116" { "Alnilam" }
                        }
                        g class="belt__star belt__star--3" {
                            circle class="belt__halo" cx="274" cy="56" r="22" {}
                            circle class="belt__dot belt__dot--mintaka" cx="274" cy="56" r="7" {}
                            text class="belt__label belt__label--mintaka" x="274" y="34" { "Mintaka" }
                        }
                    }
                    figcaption class="belt__caption" {
                        b { "Orion's Belt" }
                        p { (mintaka.ra_label()) " · " (mintaka.dec_label()) }
                    }
                }
            }
        }
    }
}

fn pillars(l: &Landing) -> Markup {
    let s = l.s;
    // (nav string id for the name, description id, proof id, lab anchor it is proven at).
    let rows = [
        ("nav-efficiency", "landing-eff-desc", "landing-eff-proof", "#efficiency"),
        ("nav-security", "landing-sec-desc", "landing-sec-proof", "#security"),
        ("nav-highload", "landing-high-desc", "landing-high-proof", "#highload"),
        ("nav-design", "landing-des-desc", "landing-des-proof", "#design"),
    ];
    html! {
        section class="lp-section" aria-labelledby="pillars-title" {
            div class="lp-wrap" {
                p class="lp-eyebrow" { (s.t("landing-pillars-eyebrow")) }
                h2 class="lp-h2" id="pillars-title" { (s.t("landing-pillars-title")) }
                p class="lp-lead" { (s.t("landing-pillars-lead")) }
                div class="pillars" {
                    @for (name_id, desc_id, proof_id, anchor) in rows {
                        @let name = s.t(name_id);
                        div class="pillar" {
                            h3 class="pillar__name" { (name) }
                            div {
                                p class="pillar__desc" { (s.t(desc_id)) }
                                span class="pillar__proof" { (s.t(proof_id)) }
                            }
                            a class="pillar__link" href=(l.lab_at(anchor))
                              aria-label=(format!("{}: {name}", s.t("landing-pillar-link"))) {
                                (s.t("landing-pillar-link")) (arrow())
                            }
                        }
                    }
                }
            }
        }
    }
}

fn method(l: &Landing) -> Markup {
    let s = l.s;
    // The three belt stars, west to east as the eye reads them, each a step.
    let steps = [
        ("alnitak", "landing-measure-title", "landing-measure-desc", false),
        ("alnilam", "landing-build-title", "landing-build-desc", false),
        ("mintaka", "landing-prove-title", "landing-prove-desc", true),
    ];
    html! {
        section class="lp-section" aria-labelledby="method-title" {
            div class="lp-wrap" {
                p class="lp-eyebrow" { (s.t("landing-method-eyebrow")) }
                h2 class="lp-h2" id="method-title" { (s.t("landing-method-title")) }
                p class="lp-lead" { (s.t("landing-method-lead")) }
                ol class="method__belt" role="list" {
                    @for (star_id, title_id, desc_id, accent) in steps {
                        @let star = orion::star(star_id);
                        li class=(if accent { "step step--accent" } else { "step" }) {
                            span class="step__node" aria-hidden="true" {}
                            p class="step__index" { (star.id) }
                            h3 class="step__title" { (s.t(title_id)) }
                            p class="step__desc" { (s.t(desc_id)) }
                            span class="step__coord" { (star.ra_label()) " · " (star.dec_label()) }
                        }
                    }
                }
            }
        }
    }
}

fn lab_callout(l: &Landing) -> Markup {
    let s = l.s;
    html! {
        section class="lp-section" aria-labelledby="lab-title" {
            div class="lp-wrap" {
                div class="lab-cta" {
                    p class="lp-eyebrow" { (s.t("landing-lab-eyebrow")) }
                    h2 class="lab-cta__title" id="lab-title" { (s.t("landing-lab-title")) }
                    p class="lab-cta__body" { (s.t("landing-lab-body")) }
                    div {
                        a class="btn btn--cta btn--lg" href=(l.lab()) {
                            (s.t("landing-lab-cta")) (arrow())
                        }
                    }
                }
            }
        }
    }
}

fn contact(l: &Landing) -> Markup {
    let s = l.s;
    let within = l.cfg.contact.reply_within.as_str();
    let body = s.ta("landing-contact-body", &[("within", within.into())]);
    let direct = l.cfg.contact.direct();
    html! {
        section id="start" class="lp-section" aria-labelledby="start-title" {
            div class="lp-wrap" {
                p class="lp-eyebrow" { (s.t("landing-contact-eyebrow")) }
                h2 class="lp-h2" id="start-title" { (s.t("landing-contact-title")) }
                div class="contact__grid" {
                    p class="contact__body" { (body) }
                    div class="contact__actions" {
                        a class="btn btn--cta btn--lg" href=(l.lab_at("#contact")) {
                            (s.t("landing-contact-cta")) (arrow())
                        }
                        @if let Some((href, text)) = direct.first() {
                            p class="contact__direct" {
                                span { (s.t("landing-contact-direct")) }
                                a href=(href) { (text) }
                            }
                        }
                    }
                }
            }
        }
    }
}

fn footer(l: &Landing) -> Markup {
    let s = l.s;
    html! {
        footer class="lp-footer" {
            div class="lp-wrap" {
                div class="lp-footer__top" {
                    div class="lp-footer__brand" {
                        a href=(l.studio_in(l.lang)) aria-label="Mintaka" { (l.logo) }
                        p class="lp-footer__tag" { (s.t("ft-tagline")) }
                        p { (s.t("ft-made")) }
                    }
                    div class="lp-footer__links" {
                        a href=(l.lab()) { (s.t("landing-to-lab")) }
                        a href=(l.url(&format!("/{}/privacy/", l.lang))) { (s.t("ft-privacy")) }
                        a href=(l.url("/.well-known/security.txt")) { (s.t("ft-security")) }
                    }
                }
                @if l.cfg.langs.len() > 1 {
                    nav class="lp-footer__lang" aria-label=(s.t("lang-switch")) {
                        @for al in &l.cfg.langs {
                            a href=(l.studio_in(al)) hreflang=(al) lang=(al)
                              aria-current=[(al == l.lang).then_some("page")] {
                                (s.t(&format!("lang-name-{al}")))
                            }
                        }
                    }
                }
                div class="lp-footer__meta" {
                    p class="lp-footer__note" { (s.t("landing-footer-note")) }
                    span class="lp-footer__build" { (s.t("ft-build")) " " (l.build) }
                }
                p { (s.ta("ft-copyright", &[("year", l.year.into())])) }
            }
        }
    }
}
