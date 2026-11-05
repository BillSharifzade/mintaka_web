//! The studio landing page (`/{lang}/studio/`): the company's front door.
//!
//! Signature: the constellation of Orion, from real J2000 coordinates, constructs itself into the
//! Mintaka mark — and then the page passes *through* the mark. Every chapter is one of its colours
//! (the Void sky, the Starlight body, Mintaka blue, Betelgeuse rust), and every change of chapter
//! is a part of the mark growing to fill the screen: the body, then Mintaka, then Betelgeuse.
//!
//! Progressive enhancement, strictly: every word, link and portrait is in this HTML and the page
//! reads completely with no script and with motion off. One small first-party module
//! (`web/studio/`, no third-party code) adds the construction, the scroll scenes, the live star
//! chart under the pointer and the page's measurements of itself.

use maud::{DOCTYPE, Markup, PreEscaped, html};
use serde_json::json;
use std::collections::BTreeMap;

use crate::config::SiteConfig;
use crate::i18n::Strings;
use crate::orion;
use crate::pages::Portrait;

/// Everything the landing template needs. No island data, no lab chrome.
pub struct Landing<'a> {
    pub cfg: &'a SiteConfig,
    pub s: &'a Strings,
    pub lang: &'a str,
    pub build: &'a str,
    pub year: i64,
    /// The horizontal logo, inlined with its colours as classes: it changes colourway with the
    /// chapter beneath it.
    pub logo: &'a Markup,
    /// The full mark, inlined the same way: the hero's resting figure, and the shape the script
    /// draws its construction towards.
    pub mark: &'a Markup,
    /// The wordmark, inlined: the footer's sign-off.
    pub wordmark: &'a Markup,
    /// Team portraits by member id.
    pub portraits: &'a BTreeMap<String, Portrait>,
    /// Preloaded font URLs for this language.
    pub font_preloads: Vec<&'a str>,
    /// Minified landing CSS, inlined in the one `<style>` (its hash is in the CSP).
    pub css: &'a str,
    /// The page's one script (first-party, `web/studio/`).
    pub script: &'a str,
    /// Its compressed size, already formatted for this language ("6.2" / "6,2"), for the footer.
    pub script_kb: String,
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

/// A north-east arrow for links that leave for the lab. Inline SVG, so it never depends on font
/// coverage.
fn arrow() -> Markup {
    html! {
        svg class="arrow" viewBox="0 0 16 16" aria-hidden="true" focusable="false" {
            path d="M4.5 11.5 L11.5 4.5 M6 4.5 H11.5 V10" {}
        }
    }
}

/// A heading's words, each in a clipping box so it can rise into view. Plain spans and spaces, so
/// the heading reads exactly as written to assistive technology and with no script.
fn words(text: &str) -> Markup {
    html! {
        @for (i, w) in text.split(' ').filter(|w| !w.is_empty()).enumerate() {
            @if i > 0 { " " }
            span class="w" { span class="wi" { (w) } }
        }
    }
}

/// A link that looks like a button.
fn button(href: &str, label: &str, class: &str, tail: Option<Markup>) -> Markup {
    html! {
        a class=(class) href=(href) {
            (label)
            @if let Some(t) = tail { (t) }
        }
    }
}

/// A language as the switcher names it. Tajik's language code is `tg`, which the links keep (in
/// `hreflang` and `lang`, where browsers, search engines and screen readers need it), but readers
/// in Tajikistan know it by the country's TJ.
fn lang_code(lang: &str) -> String {
    match lang {
        "tg" => "TJ".to_owned(),
        other => other.to_uppercase(),
    }
}

/// The stars the logo's corners stand on (Bellatrix, Rigel, Saiph), left faintly in its body: the
/// mark's own construction, quietly showing. The hero's canvas reads these circles (`lg-f`) and
/// draws the same dots, so the still figure and the drawn one stay identical.
fn mark_with_stars(mark: &Markup) -> Markup {
    let dots: String = ["bellatrix", "rigel", "saiph"]
        .iter()
        .filter_map(|id| {
            let (_, x, y) = orion::LOGO.iter().find(|(i, ..)| i == id)?;
            Some(format!(r#"<circle class="lg-f" cx="{x}" cy="{y}" r="{:.2}"/>"#, faint_r(id)))
        })
        .collect();
    PreEscaped(mark.0.replacen("</g></svg>", &format!("{dots}</g></svg>"), 1))
}

/// A corner star's faint dot, in mark units: by its brightness on the brand board (Rigel, the
/// brightest of the three, is the largest).
fn faint_r(id: &str) -> f64 {
    2.2 + orion::star(id).r * 0.16
}

/// The wordmark with each letter its own path (the brand file draws all seven as one), so the
/// footer can raise them one by one; the dot of the i — Mintaka — lands last, with a ring.
///
/// The brand path is a run of absolute subpaths (`M…`); a subpath whose box lies inside the one
/// before it is that letter's counter (the bowls of the two a's) and stays with it.
fn wordmark_letters(wordmark: &Markup) -> Markup {
    let svg = &wordmark.0;
    let Some(start) = svg.find(r#"<path d=""#).map(|i| i + 9) else { return wordmark.clone() };
    let Some(len) = svg[start..].find('"') else { return wordmark.clone() };
    let d = &svg[start..start + len];
    let bbox = |sub: &str| {
        let nums: Vec<f64> = sub
            .split(|c: char| !(c.is_ascii_digit() || c == '.' || c == '-'))
            .filter(|t| !t.is_empty())
            .filter_map(|t| t.parse().ok())
            .collect();
        nums.chunks(2).filter(|p| p.len() == 2).fold([f64::MAX, f64::MAX, f64::MIN, f64::MIN], |b, p| {
            [b[0].min(p[0]), b[1].min(p[1]), b[2].max(p[0]), b[3].max(p[1])]
        })
    };
    let mut letters: Vec<(String, [f64; 4])> = Vec::new();
    for sub in d.split('M').filter(|s| !s.trim().is_empty()) {
        let sub = format!("M{sub}");
        let b = bbox(&sub);
        match letters.last_mut() {
            Some((path, outer)) if b[0] >= outer[0] && b[1] >= outer[1] && b[2] <= outer[2] && b[3] <= outer[3] => path.push_str(&sub),
            _ => letters.push((sub, b)),
        }
    }
    let paths: String = letters.iter().map(|(p, _)| format!(r#"<path class="lg-s wm-l" d="{p}"/>"#)).collect();
    // The original path ends at its closing `/>`; swap it for the letters.
    let Some(end) = svg[start..].find("/>").map(|i| start + i + 2) else { return wordmark.clone() };
    let mut out = String::with_capacity(svg.len() + 400);
    out.push_str(&svg[..start - 9]);
    out.push_str(&paths);
    out.push_str(&svg[end..]);
    // Mintaka, the i's dot, lands last; a copy of it underneath rings out as it does.
    if let Some(a) = out.find("<circle ")
        && let Some(b) = out[a..].find("/>").map(|i| a + i + 2)
    {
        let dot = out[a..b].to_owned();
        let ping = dot.replace(r#"class="lg-m""#, r#"class="wm-ping""#);
        let dot = dot.replace(r#"class="lg-m""#, r#"class="lg-m wm-dot""#);
        out.replace_range(a..b, &format!("{ping}{dot}"));
    }
    PreEscaped(out)
}

/// Right ascension and declination to the minute: "5h 14m", "−8° 12′".
fn ra_short(st: &orion::Star) -> String {
    format!("{}h {:02}m", st.ra.0, st.ra.1)
}
fn dec_short(st: &orion::Star) -> String {
    let sign = if st.dec.0 < 0.0 { "−" } else { "+" };
    format!("{sign}{}° {:02}′", st.dec.1, st.dec.2)
}

/// A small chart of Orion beside the chapter's heading: the legend to the four promises below it,
/// each of which stands under one of its stars, as the lab's sections do. Real positions, from the
/// same catalogue as the hero; the promise's star lights up while the reader is on its row.
fn legend(s: &Strings, stars: &[&str]) -> Markup {
    let (pts, h) = orion::layout(200.0, 18.0);
    let at = |id: &str| pts.iter().find(|(st, ..)| st.id == id).map(|(_, x, y, _)| (*x, *y)).unwrap_or_default();
    html! {
        svg class="lgd" viewBox=(format!("0 0 200 {h:.0}")) aria-hidden="true" focusable="false" {
            @for (a, b, _) in orion::LINES {
                @let ((ax, ay), (bx, by)) = (at(a), at(b));
                path class="lgd__line" d=(format!("M{ax:.1} {ay:.1}L{bx:.1} {by:.1}")) pathLength="1" {}
            }
            @for (st, x, y, _) in &pts {
                g class=(format!("lgd__star lgd__star--{}", st.id)) {
                    circle cx=(format!("{x:.1}")) cy=(format!("{y:.1}")) r=(format!("{:.2}", 1.2 + st.r * 0.12)) {}
                    @if stars.contains(&st.id) {
                        // Labels sit where the figure's lines are not: for the stars at the edges
                        // (Betelgeuse, Rigel), to their left — Betelgeuse's out into the margin.
                        @let left = *x > 150.0 || *x < 30.0;
                        text x=(format!("{:.1}", if left { x - 8.0 } else { x + 8.0 })) y=(format!("{:.1}", y + 2.6))
                            text-anchor=[left.then_some("end")] {
                            (s.t(&format!("star-{}", st.id)).to_uppercase())
                        }
                    }
                }
            }
        }
    }
}

/// Mark the Tajik word in the name story with `lang="tg"`.
fn word_tg(text: &str) -> Markup {
    let word = "минтақа";
    match text.find(word) {
        Some(i) => html! { (&text[..i]) span lang="tg" { (word) } (&text[i + word.len()..]) },
        None => html! { (text) },
    }
}

/// A whole number grouped the way the page's language writes it: 4,000 / 4 000.
fn group(n: u32, lang: &str) -> String {
    let digits = n.to_string();
    if digits.len() <= 3 {
        return digits;
    }
    let sep = if lang == "en" { "," } else { "\u{a0}" };
    let (head, tail) = digits.split_at(digits.len() - 3);
    format!("{head}{sep}{tail}")
}

fn round(v: f64, places: i32) -> f64 {
    let f = 10f64.powi(places);
    (v * f).round() / f
}

/// What the star chart in the hero needs, as JSON for the script: each star's name and fact (in
/// this language), its catalogue coordinates, where it really is and where the logo's construction
/// puts it — both in the mark's own units — plus the fit between the two and the projection centre,
/// so the script can turn any point under the pointer back into right ascension and declination.
///
/// The fit is one rotation and one scale about Alnilam (both systems have it at 0,0), by least
/// squares over the seven stars the logo uses: z = Σ conj(p)·q / Σ |p|².
fn sky_data(s: &Strings) -> String {
    let logo = |id: &str| orion::LOGO.iter().find(|(i, ..)| *i == id).map(|(_, x, y)| (*x, *y));
    let (mut re, mut im, mut nn) = (0.0, 0.0, 0.0);
    for st in &orion::STARS {
        if let Some((qx, qy)) = logo(st.id) {
            let (px, py) = orion::project(st);
            re += px * qx + py * qy;
            im += px * qy - py * qx;
            nn += px * px + py * py;
        }
    }
    let (a, b) = (re / nn, im / nn);
    let stars: Vec<serde_json::Value> = orion::STARS
        .iter()
        .map(|st| {
            let (px, py) = orion::project(st);
            json!({
                "id": st.id,
                "n": s.t(&format!("star-{}", st.id)),
                "f": s.t(&format!("star-{}-fact", st.id)),
                "c": format!("{} · {}", st.ra_label(), st.dec_label()),
                "s": [round(a * px - b * py, 1), round(a * py + b * px, 1)],
                "l": logo(st.id).map(|(x, y)| [x, y]),
                "r": st.r,
            })
        })
        .collect();
    let c = orion::star("alnilam");
    json!({ "fit": [round(a, 5), round(b, 5)], "c0": [round(c.ra_deg(), 5), round(c.dec_deg(), 5)], "stars": stars }).to_string()
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
    // The view-source greeting an engineer reading our HTML meets first.
    let greeting = format!(
        "<!--\n      ●            Mintaka · software at the edge of physics\n       ╲   ·\n        ╲ ·  ●     A studio that measures its own claims.\n    · ·  ●         Every word here reads with no script; the one module this\n        ╱    ╲     page loads is ours, line for line ({} kB).\n       ●      ●    The lab proves the rest: {}\n-->\n",
        l.script_kb,
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
                script type="module" src=(l.script) {}
            }
            body {
                a class="skip" href="#main" { (s.t("skip-link")) }
                (header(l))
                main id="main" tabindex="-1" {
                    (hero(l))
                    (measured(l))
                    (method(l))
                    (lab(l))
                    (people(l))
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
        header class="hd" data-theme="void" {
            div class="hd__in" {
                a class="hd__logo" href=(l.studio_in(l.lang)) aria-label="Mintaka" { (l.logo) }
                nav class="hd__nav" aria-label=(s.t("nav-label")) {
                    a class="hd__link" href=(l.lab()) { (s.t("landing-to-lab")) }
                    @if l.cfg.langs.len() > 1 {
                        span class="hd__lang" {
                            @for al in &l.cfg.langs {
                                a href=(l.studio_in(al)) hreflang=(al) lang=(al) title=(s.t(&format!("lang-name-{al}")))
                                  aria-current=[(al == l.lang).then_some("page")] {
                                    (lang_code(al))
                                }
                            }
                        }
                    }
                }
                (button("#start", &s.t("nav-cta"), "btn btn--cta btn--sm", None))
            }
        }
    }
}

fn hero(l: &Landing) -> Markup {
    let s = l.s;
    let m = orion::star("mintaka");
    html! {
        section class="ch hero" data-theme="void" data-scene="hero" aria-labelledby="hero-title" {
            div class="hero__pin" {
                div class="hero__stars" aria-hidden="true" {}
                canvas class="hero__sky" aria-hidden="true" data-sky=(sky_data(s)) {}
                div class="hero__fig" role="img" aria-label=(s.t("landing-sky-label")) { (mark_with_stars(l.mark)) }
                div class="wrap hero__copy" {
                    p class="eyebrow hero__eyebrow" { (s.t("hero-eyebrow")) }
                    h1 class="hero__title" id="hero-title" { (words(&s.t("hero-title"))) }
                    p class="hero__lead" { (s.t("landing-hero-lead")) }
                    div class="hero__cta" {
                        (button("#start", &s.t("nav-cta"), "btn btn--cta btn--lg", None))
                        a class="go" href=(l.lab()) { span { (s.t("landing-hero-lab")) } (arrow()) }
                    }
                }
                div class="wrap hero__foot" {
                    p { span class="hero__star" { (s.t("star-mintaka")) } " · " (m.ra_label()) " · " (m.dec_label()) }
                    p class="hero__here" { (s.t("landing-here")) }
                    // The sky twinkles for as long as the page is open: a way to stop it (WCAG 2.2.2).
                    // Shown by the script, which is what animates it.
                    button class="pause" type="button" hidden aria-label=(s.t("hero-motion-pause"))
                        data-pause=(s.t("hero-motion-pause")) data-play=(s.t("hero-motion-play")) {
                        svg class="pause__icon" viewBox="0 0 16 16" aria-hidden="true" focusable="false" {
                            path class="pause__bars" d="M5.5 3.5v9M10.5 3.5v9" {}
                            path class="pause__play" d="M5.5 3.2v9.6l7.5-4.8z" {}
                        }
                    }
                }
            }
        }
    }
}

/// One of the four promises: its pillar (as the lab names it), the star it stands under there, what
/// we build, its figure (prefix, number, unit), what the figure is, and how we do it. Text fields are
/// string keys.
struct Promise {
    pillar: &'static str,
    star: &'static str,
    title: &'static str,
    pre: &'static str,
    num: u32,
    unit: &'static str,
    caption: &'static str,
    desc: &'static str,
}

fn measured(l: &Landing) -> Markup {
    let s = l.s;
    // The proof for each promise lives in the lab, which the lab chapter below links to.
    let rows = [
        Promise { pillar: "efficiency", star: "rigel", title: "landing-eff-title", pre: "<", num: 50, unit: "landing-eff-unit", caption: "landing-eff-caption", desc: "landing-eff-desc" },
        Promise { pillar: "security", star: "saiph", title: "landing-sec-title", pre: "", num: 0, unit: "", caption: "landing-sec-caption", desc: "landing-sec-desc" },
        Promise { pillar: "highload", star: "betelgeuse", title: "landing-high-title", pre: "", num: 4000, unit: "landing-high-unit", caption: "landing-high-caption", desc: "landing-high-desc" },
        Promise { pillar: "design", star: "meissa", title: "landing-des-title", pre: "<", num: 14, unit: "landing-unit-kb", caption: "landing-des-caption", desc: "landing-des-desc" },
    ];
    let stars: Vec<&str> = rows.iter().map(|r| r.star).collect();
    // The page's own numbers, read in the visitor's browser by the script (hidden without it).
    let metrics = [
        ("html", "landing-readout-html", "landing-unit-kb"),
        ("total", "landing-readout-total", "landing-unit-kb"),
        ("js", "landing-readout-js", "landing-unit-kb"),
        ("third", "landing-readout-third", ""),
        ("ttfb", "landing-readout-ttfb", "landing-unit-ms"),
        ("fcp", "landing-readout-fcp", "landing-unit-ms"),
    ];
    html! {
        section id="measured" class="ch measured" data-theme="light" aria-labelledby="measured-title" {
            div class="wrap" {
                header class="head head--chart" {
                    p class="eyebrow" data-reveal { (s.t("landing-pillars-eyebrow")) }
                    h2 class="h2" id="measured-title" data-split { (words(&s.t("landing-pillars-title"))) }
                    p class="lead" data-reveal { (s.t("landing-pillars-lead")) }
                    div class="head__chart" data-reveal { (legend(s, &stars)) }
                }
                ol class="rows" role="list" {
                    @for Promise { pillar, star, title, pre, num, unit, caption, desc } in rows {
                        @let st = orion::star(star);
                        li class=(format!("row row--{star}")) data-reveal {
                            div class="row__fig" {
                                p class="fig" {
                                    @if !pre.is_empty() { span class="fig__pre" { (pre) } }
                                    @if num > 0 {
                                        span class="fig__num" data-count=(num) { (group(num, l.lang)) }
                                    } @else {
                                        span class="fig__num" { "0" }
                                    }
                                    @if !unit.is_empty() { span class="fig__unit" { (s.t(unit)) } }
                                }
                                p class="row__cap" { (s.t(caption)) }
                            }
                            div class="row__text" {
                                p class="row__name" {
                                    span { (s.t(&format!("nav-{pillar}"))) }
                                    // The promise's star, as on a chart's legend: a quiet aside.
                                    span class="row__star" aria-hidden="true" {
                                        i class="row__dot" {}
                                        (s.t(&format!("star-{star}")))
                                        span class="row__radec" { (ra_short(st)) " · " (dec_short(st)) }
                                    }
                                }
                                h3 class="row__title" { (s.t(title)) }
                                p class="row__desc" { (s.t(desc)) }
                            }
                        }
                    }
                }
                div class="readout" role="group" hidden data-reveal aria-labelledby="readout-title" {
                    div class="readout__head" {
                        h3 class="readout__title" id="readout-title" {
                            span class="live" aria-hidden="true" {}
                            (s.t("landing-readout-title"))
                        }
                        p class="readout__note" { (s.t("landing-readout-note")) }
                    }
                    dl class="readout__grid" {
                        @for (key, label, unit) in metrics {
                            div class="metric" {
                                dt { (s.t(label)) }
                                dd {
                                    span class="metric__v" data-m=(key) { "—" }
                                    @if !unit.is_empty() { " " span class="metric__u" { (s.t(unit)) } }
                                }
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
    // The three belt stars, west to east as the eye reads them, each a step. The staircase they
    // form on a wide screen rises at the belt's own 1:2 slope.
    let steps = [
        ("alnitak", "landing-measure-title", "landing-measure-desc"),
        ("alnilam", "landing-build-title", "landing-build-desc"),
        ("mintaka", "landing-prove-title", "landing-prove-desc"),
    ];
    html! {
        section id="method" class="ch method" data-theme="light" data-scene="belt" aria-labelledby="method-title" {
            div class="method__pin" {
                div class="wrap method__in" {
                    header class="head method__head" {
                        p class="eyebrow" data-reveal { (s.t("landing-method-eyebrow")) }
                        h2 class="sr" id="method-title" { (s.t("landing-method-title")) }
                        p class="lead" data-reveal { (s.t("landing-method-lead")) }
                    }
                    ol class="steps" role="list" {
                        @for (i, (star, title, desc)) in steps.iter().enumerate() {
                            li class=(format!("step step--{}", i + 1)) {
                                span class=(if *star == "mintaka" { "step__star step__star--m" } else { "step__star" }) aria-hidden="true" {}
                                p class="step__name" { (s.t(&format!("star-{star}"))) }
                                h3 class="step__title" { (s.t(title)) }
                                p class="step__desc" { (s.t(desc)) }
                            }
                        }
                    }
                }
                svg class="belt" aria-hidden="true" focusable="false" {
                    path class="belt__ext" d="M0 0" {}
                    path class="belt__line" d="M0 0" {}
                }
                div class="burst burst--blue" aria-hidden="true" {}
            }
        }
    }
}

fn lab(l: &Landing) -> Markup {
    let s = l.s;
    // The lab's live instruments — the proof behind the promises above — each named by its own
    // title there and linked to its section.
    let rows = [
        ("physics-instrument-title", "#physics"),
        ("eff-race-title", "#efficiency"),
        ("sec-fuzz-title", "#security"),
        ("hl-lab-title", "#highload"),
        ("ds-laws-title", "#design"),
    ];
    html! {
        section id="lab" class="ch lab" data-theme="blue" aria-labelledby="lab-title" {
            div class="wrap" {
                header class="head" {
                    p class="eyebrow" data-reveal { (s.t("landing-to-lab")) }
                    h2 class="h2" id="lab-title" data-split { (words(&s.t("landing-lab-title"))) }
                    p class="lead lead--wide" data-reveal { (s.t("landing-lab-body")) }
                }
                ul class="exps" role="list" {
                    @for (title, anchor) in rows {
                        li class="exp" data-reveal {
                            a class="exp__a" href=(l.lab_at(anchor)) {
                                span class="exp__title" { (s.t(title)) }
                                span class="exp__go" aria-hidden="true" { (arrow()) }
                            }
                        }
                    }
                }
                div class="lab__cta" data-reveal {
                    (button(&l.lab(), &s.t("landing-lab-cta"), "btn btn--cta btn--lg", Some(arrow())))
                }
            }
        }
    }
}

fn people(l: &Landing) -> Markup {
    let s = l.s;
    html! {
        section id="people" class="ch people" data-theme="blue" aria-labelledby="people-title" {
            div class="wrap" {
                header class="head" {
                    p class="eyebrow" data-reveal { (s.t("team-eyebrow")) }
                    h2 class="h2" id="people-title" data-split { (words(&s.t("landing-team-title"))) }
                }
                ul class="founders" role="list" {
                    @for m in &l.cfg.team {
                        @let name = s.t(&format!("team-{}-name", m.id));
                        @let photo_key = format!("team-{}-photo", m.id);
                        @let alt = if s.has(&photo_key) { s.t(&photo_key) } else { s.ta("team-photo-alt", &[("name", name.clone().into())]) };
                        li class="founder" data-reveal {
                            @if let Some(p) = l.portraits.get(&m.id) {
                                picture class="founder__photo" {
                                    source type="image/avif" srcset=(p.avif);
                                    img src=(p.webp) width="224" height="224" loading="lazy" decoding="async" alt=(alt);
                                }
                            }
                            div class="founder__text" {
                                p class="founder__role" { (s.t(&format!("team-{}-role", m.id))) }
                                h3 class="founder__name" { (name) }
                                p class="founder__line" { (s.t(&format!("team-{}-line", m.id))) }
                                @if !m.site.is_empty() {
                                    // Whose site, for a screen reader listing links out of context.
                                    a class="btn btn--line btn--sm founder__site" href=(m.site) {
                                        (s.t("landing-team-site")) span class="sr" { ", " (name) } (arrow())
                                    }
                                }
                            }
                        }
                    }
                }
                div class="story" {
                    h3 class="eyebrow story__title" data-reveal { (s.t("team-name-title")) }
                    ul class="story__facts" role="list" {
                        li data-reveal { span class="story__dot" aria-hidden="true" {} (s.t("team-name-star")) }
                        li data-reveal { span class="story__dot" aria-hidden="true" {} (word_tg(&s.t("team-name-word"))) }
                        li data-reveal { span class="story__dot story__dot--m" aria-hidden="true" {} (s.t("team-name-pass")) }
                    }
                    p class="story__end" data-split { (words(&s.t("team-name-end"))) }
                }
            }
        }
    }
}

/// The last chapter, and every "Start a project" on the page lands here: the brief form itself.
/// A plain HTML form — it works with no script — posting to the same endpoint as the lab's form
/// (`/api/contact`, which answers a form post with a redirect to `/{lang}/sent/`). Same fields,
/// minus the timeline: the shorter the form, the likelier the brief.
fn contact(l: &Landing) -> Markup {
    let s = l.s;
    let c = &l.cfg.contact;
    let kinds = ["new-product", "performance", "security-audit", "design", "ai", "not-sure"];
    let optional = html! { " " span class="field__opt" { "(" (s.t("ct-optional")) ")" } };
    let field = |name: &str, label: Markup, input: Markup, hint: Option<String>| -> Markup {
        html! {
            div class="field" {
                label class="field__label" for=(format!("b-{name}")) { (label) }
                @if let Some(h) = hint { p class="field__hint" id=(format!("bh-{name}")) { (h) } }
                (input)
            }
        }
    };
    html! {
        section id="start" class="ch contact" data-theme="rust" aria-labelledby="start-title" {
            div class="wrap contact__grid" {
                div class="contact__intro" {
                    // A long first word ("Расскажите,") would outgrow its column at the largest
                    // size and run into the form: such a title is sized by the column instead.
                    @let title = s.t("landing-contact-title");
                    @let long = title.split(' ').any(|w| w.chars().count() >= 10);
                    h2 class=(if long { "contact__title contact__title--long" } else { "contact__title" }) id="start-title" data-split { (words(&title)) }
                    p class="contact__body" data-reveal { (s.ta("ct-lead", &[("within", c.reply_within.as_str().into())])) }
                    @if let Some((href, text)) = c.direct().first() {
                        p class="contact__direct" data-reveal {
                            span { (s.t("landing-contact-direct")) }
                            a href=(href) { (text) }
                        }
                    }
                }
                form class="brief" method="post" action=(l.url("/api/contact")) data-reveal {
                    input type="hidden" name="lang" value=(l.lang);
                    fieldset class="brief__group" {
                        legend class="field__label" { (s.t("ct-kind")) }
                        div class="chips" {
                            @for k in kinds {
                                label class="chip" {
                                    input class="sr" type="checkbox" name="kind" value=(k);
                                    span { (s.t(&format!("ct-kind-{k}"))) }
                                }
                            }
                        }
                    }
                    (field("message", html! { (s.t("ct-message")) (optional) },
                        html! { textarea class="input" id="b-message" name="message" maxlength="5000" rows="4" aria-describedby="bh-message" {} },
                        Some(s.t("ct-message-hint"))))
                    div class="brief__row" {
                        (field("name", html! { (s.t("ct-name")) },
                            html! { input class="input" id="b-name" name="name" required maxlength="120" autocomplete="name"; },
                            None))
                        (field("company", html! { (s.t("ct-company")) (optional) },
                            html! { input class="input" id="b-company" name="company" maxlength="120" autocomplete="organization"; },
                            None))
                    }
                    p class="brief__rule" id="bh-contact" { (s.t("ct-contact-rule")) }
                    div class="brief__row" {
                        (field("email", html! { (s.t("ct-email")) },
                            html! { input class="input" id="b-email" name="email" type="email" maxlength="254" autocomplete="email" aria-describedby="bh-contact"; },
                            None))
                        (field("telegram", html! { (s.t("ct-telegram")) },
                            html! { input class="input" id="b-telegram" name="telegram" maxlength="33" autocomplete="off" spellcheck="false" autocapitalize="none" placeholder="@" aria-describedby="bh-contact"; },
                            None))
                    }
                    div class="brief__trap" aria-hidden="true" {
                        label { (s.t("ct-trap")) input name="website" tabindex="-1" autocomplete="off"; }
                    }
                    div class="brief__send" {
                        button class="btn btn--cta btn--xl" type="submit" { (s.t("ct-send")) (arrow()) }
                        p class="brief__privacy" { (s.t("ct-privacy")) }
                    }
                }
            }
        }
    }
}

fn footer(l: &Landing) -> Markup {
    let s = l.s;
    html! {
        footer class="ft" data-theme="void" {
            div class="wrap" {
                div class="ft__mark" aria-hidden="true" { (wordmark_letters(l.wordmark)) }
                div class="ft__grid" {
                    div class="ft__brand" {
                        p class="ft__tag" { (s.t("ft-tagline")) }
                        p { (s.t("ft-made")) }
                    }
                    div class="ft__links" {
                        a href=(l.lab()) { (s.t("landing-to-lab")) }
                        a href=(l.url(&format!("/{}/privacy/", l.lang))) { (s.t("ft-privacy")) }
                        a href=(l.url("/.well-known/security.txt")) { (s.t("ft-security")) }
                    }
                    @if l.cfg.langs.len() > 1 {
                        nav class="ft__lang" aria-label=(s.t("lang-switch")) {
                            @for al in &l.cfg.langs {
                                a href=(l.studio_in(al)) hreflang=(al) lang=(al)
                                  aria-current=[(al == l.lang).then_some("page")] {
                                    (s.t(&format!("lang-name-{al}")))
                                }
                            }
                        }
                    }
                }
                div class="ft__meta" {
                    p class="ft__note" { (s.ta("landing-footer-note", &[("kb", l.script_kb.clone().into())])) }
                    p { (s.ta("ft-copyright", &[("year", l.year.into())])) " · " (s.t("ft-build")) " " span class="ft__build" { (l.build) } }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_group_by_language() {
        assert_eq!(group(14, "en"), "14");
        assert_eq!(group(4000, "en"), "4,000");
        assert_eq!(group(4000, "ru"), "4\u{a0}000");
    }

    #[test]
    fn tajik_is_tj_to_the_eye_and_tg_to_the_machine() {
        assert_eq!(lang_code("tg"), "TJ");
        assert_eq!(lang_code("ru"), "RU");
    }

    #[test]
    fn the_wordmark_splits_into_its_seven_letters() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let wm = crate::pages::svg::brand_classed(&root, "mintaka-wordmark-on-dark.svg", "wordmark").unwrap();
        let out = wordmark_letters(&wm).into_string();
        // m, i, n, t, a, k, a — each a's bowl stays with its letter.
        assert_eq!(out.matches(r#"class="lg-s wm-l""#).count(), 7, "{out}");
        assert_eq!(out.matches(r#"class="wm-ping""#).count(), 1);
        assert_eq!(out.matches(r#"class="lg-m wm-dot""#).count(), 1);
        // Nothing of the brand path is lost: the same numbers, in the same order.
        let digits = |s: &str| s.chars().filter(|c| c.is_ascii_digit()).count();
        assert_eq!(digits(&out) - digits(&wm.0), digits(r#"cx="98.5" cy="-63.2" r="8.51""#));
    }

    #[test]
    fn the_corner_stars_sit_inside_the_mark() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let mark = crate::pages::svg::brand_classed(&root, "mintaka-mark-full-on-dark.svg", "mark").unwrap();
        let out = mark_with_stars(&mark).into_string();
        assert_eq!(out.matches(r#"class="lg-f""#).count(), 3);
        // Each dot clears the body's edges, which pass at least 8.8 units from its centre.
        for id in ["bellatrix", "rigel", "saiph"] {
            assert!(faint_r(id) < 6.0, "{id}");
        }
    }

    #[test]
    fn headings_split_into_words_and_read_the_same() {
        let html = words("Software at the edge").into_string();
        assert_eq!(html.matches(r#"class="w""#).count(), 4);
        let text: String = html.split('<').map(|part| part.split_once('>').map_or(part, |(_, t)| t)).collect();
        assert_eq!(text, "Software at the edge");
    }

    #[test]
    fn the_sky_fits_the_logo() {
        // The fitted real sky lands near the logo's construction points (the brand board says the
        // shoulders and feet tilt as they do in the real sky): every star within 30 units.
        let fit: serde_json::Value = {
            let (mut re, mut im, mut nn) = (0.0, 0.0, 0.0);
            for st in &orion::STARS {
                if let Some((_, qx, qy)) = orion::LOGO.iter().find(|(i, ..)| *i == st.id) {
                    let (px, py) = orion::project(st);
                    re += px * qx + py * qy;
                    im += px * qy - py * qx;
                    nn += px * px + py * py;
                }
            }
            json!([re / nn, im / nn])
        };
        let (a, b) = (fit[0].as_f64().unwrap(), fit[1].as_f64().unwrap());
        for (id, qx, qy) in orion::LOGO {
            let (px, py) = orion::project(orion::star(id));
            let (x, y) = (a * px - b * py, a * py + b * px);
            let d = ((x - qx).powi(2) + (y - qy).powi(2)).sqrt();
            assert!(d < 30.0, "{id} is {d:.1} units from its construction point");
        }
    }
}
