//! The studio landing page (`/{lang}/studio/`): the company's front door. A distinct, lean page
//! — its own small stylesheet, no islands and no JavaScript at all (like the Lab, spec §9.5) —
//! that links to the lab for the proof. Signature: the whole constellation of Orion, projected
//! from the real star coordinates (`crate::orion`), with the company's star, Mintaka, picked out
//! and alight. Everything moves with CSS alone; nothing here needs a script to be seen.

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

/// A short coordinate, trimmed: SVG geometry rounded to 0.1 so the markup stays small.
fn fmt(v: f64) -> String {
    let r = (v * 10.0).round() / 10.0;
    if (r - r.round()).abs() < f64::EPSILON {
        format!("{}", r as i64)
    } else {
        format!("{r}")
    }
}

/// The hero signature: the full figure of Orion, projected from real J2000 coordinates
/// (`crate::orion`), drawn as a star map. Each star carries its real colour — Betelgeuse warm,
/// Rigel and the Belt blue-white — and Mintaka, the studio's star, is picked out and alight.
/// Decorative detail lives in CSS; the SVG is only geometry, so it degrades to a still star map.
fn constellation(l: &Landing) -> Markup {
    let s = l.s;
    let width = 740.0;
    let margin = 104.0;
    let (stars, height) = orion::layout(width, margin);
    let find = |id: &str| -> (f64, f64) {
        stars.iter().find(|(st, ..)| st.id == id).map(|(_, x, y, _)| (*x, *y)).unwrap_or((0.0, 0.0))
    };
    let tint = |t: orion::Tint| match t {
        orion::Tint::Rust => "o-star o-star--rust",
        orion::Tint::Blue => "o-star o-star--mintaka",
        orion::Tint::Starlight => "o-star o-star--white",
    };
    html! {
        svg class="o-sky__fig" viewBox=(format!("0 0 {} {}", fmt(width), fmt(height)))
            role="img" aria-label=(s.t("landing-sky-label")) preserveAspectRatio="xMidYMid meet" {
            g class="o-lines" {
                @for (a, b, belt) in orion::LINES {
                    @let (ax, ay) = find(a);
                    @let (bx, by) = find(b);
                    @let cls = if belt { "o-line o-line--belt" } else { "o-line" };
                    line class=(cls) x1=(fmt(ax)) y1=(fmt(ay)) x2=(fmt(bx)) y2=(fmt(by)) pathLength="1" {}
                }
            }
            g class="o-stars" {
                @for (st, x, y, r) in &stars {
                    g class=(tint(st.tint)) {
                        circle class="o-halo" cx=(fmt(*x)) cy=(fmt(*y)) r=(fmt(r * 2.6)) {}
                        circle class="o-dot" cx=(fmt(*x)) cy=(fmt(*y)) r=(fmt(r.max(3.2))) {}
                        @if st.id == "mintaka" {
                            circle class="o-ring" cx=(fmt(*x)) cy=(fmt(*y)) r=(fmt(r * 1.9)) {}
                        }
                    }
                }
            }
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
                div class="lp-sky" aria-hidden="true" {}
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
    let m = orion::star("mintaka");
    // The four measured facts, shown as a quiet telemetry strip at the foot of the hero. Each is a
    // claim the lab proves; the colours and dots are decoration, the words are the evidence.
    let proofs = ["landing-eff-proof", "landing-sec-proof", "landing-high-proof", "landing-des-proof"];
    html! {
        section class="lp-hero" {
            div class="lp-hero__sky" {
                figure class="o-sky" { (constellation(l)) }
            }
            div class="o-sky__cap" aria-hidden="true" {
                span class="o-sky__name" { "Mintaka" }
                span class="o-sky__coord" { (m.ra_label()) " · " (m.dec_label()) }
            }
            div class="lp-wrap lp-hero__inner" {
                p class="lp-eyebrow lp-hero__eyebrow" { (s.t("hero-eyebrow")) }
                h1 class="lp-h1" { (s.t("hero-title")) }
                p class="lp-hero__lead" { (s.t("landing-hero-lead")) }
                div class="lp-cta-row" {
                    a class="btn btn--cta btn--lg" href="#start" { (s.t("nav-cta")) }
                    a class="btn btn--ghost btn--lg" href=(l.lab()) {
                        (s.t("landing-hero-lab")) (arrow())
                    }
                }
                ul class="lp-ticker" role="list" aria-label=(s.t("landing-pillars-eyebrow")) {
                    @for p in proofs {
                        li class="lp-ticker__item" {
                            span class="lp-ticker__dot" aria-hidden="true" {}
                            (s.t(p))
                        }
                    }
                }
            }
        }
    }
}

fn pillars(l: &Landing) -> Markup {
    let s = l.s;
    // (nav string id for the name, description id, proof id, lab anchor it is proven at, tint).
    let rows = [
        ("nav-efficiency", "landing-eff-desc", "landing-eff-proof", "#efficiency", "card--eff"),
        ("nav-security", "landing-sec-desc", "landing-sec-proof", "#security", "card--sec"),
        ("nav-highload", "landing-high-desc", "landing-high-proof", "#highload", "card--high"),
        ("nav-design", "landing-des-desc", "landing-des-proof", "#design", "card--des"),
    ];
    html! {
        section class="lp-section lp-pillars" aria-labelledby="pillars-title" {
            div class="lp-wrap" {
                div class="lp-head" {
                    p class="lp-eyebrow lp-in" { (s.t("landing-pillars-eyebrow")) }
                    h2 class="lp-h2 lp-in" id="pillars-title" { (s.t("landing-pillars-title")) }
                    p class="lp-lead lp-in" { (s.t("landing-pillars-lead")) }
                }
                div class="cards" {
                    @for (name_id, desc_id, proof_id, anchor, tint) in rows {
                        @let name = s.t(name_id);
                        a class=(format!("card lp-in {tint}")) href=(l.lab_at(anchor))
                          aria-label=(format!("{name}: {}", s.t("landing-pillar-link"))) {
                            span class="card__proof" {
                                span class="card__dot" aria-hidden="true" {}
                                (s.t(proof_id))
                            }
                            h3 class="card__name" { (name) }
                            p class="card__desc" { (s.t(desc_id)) }
                            span class="card__link" aria-hidden="true" {
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
        section class="lp-section lp-method" aria-labelledby="method-title" {
            div class="lp-wrap" {
                div class="lp-head" {
                    p class="lp-eyebrow lp-in" { (s.t("landing-method-eyebrow")) }
                    h2 class="lp-h2 lp-in" id="method-title" { (s.t("landing-method-title")) }
                    p class="lp-lead lp-in" { (s.t("landing-method-lead")) }
                }
                ol class="method__belt" role="list" {
                    @for (star_id, title_id, desc_id, accent) in steps {
                        @let star = orion::star(star_id);
                        li class=(if accent { "step step--accent lp-in" } else { "step lp-in" }) {
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
        section class="lp-section lp-lab" aria-labelledby="lab-title" {
            div class="lp-wrap" {
                div class="lab-cta lp-in" {
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
        section id="start" class="lp-section lp-contact" aria-labelledby="start-title" {
            div class="lp-wrap" {
                div class="lp-head" {
                    p class="lp-eyebrow lp-in" { (s.t("landing-contact-eyebrow")) }
                    h2 class="lp-h2 lp-in" id="start-title" { (s.t("landing-contact-title")) }
                }
                div class="contact__grid lp-in" {
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
