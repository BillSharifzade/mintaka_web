//! The landing page, S1–S13 (spec §6). S14 is the shared footer.

use super::{Ctx, instrument, pillar_head, svg};
use crate::config::Member;
use crate::geo;
use crate::highlight;
use maud::{Markup, PreEscaped, html};

const EMAIL_SNIPPET: &str = r#"/// An `Email` is only ever the result of parsing one.
pub struct Email { local: Local, domain: Domain }

impl FromStr for Email {
    type Err = InvalidEmail;
    fn from_str(raw: &str) -> Result<Self, Self::Err> {
        let (local, domain) = raw.rsplit_once('@').ok_or(InvalidEmail)?;
        Ok(Email { local: local.parse()?, domain: domain.parse()? })
    }   // parsed once, at the edge: everything after takes an Email
}"#;

pub fn body(ctx: &Ctx) -> Markup {
    html! {
        // The observatory: the particle engine's three anchors (the hourglass, the ray, Orion).
        (hero(ctx))
        (physics(ctx))
        (pillars(ctx))
        // The bench: the lit room where the instruments stand.
        div class="bench lit" {
            (efficiency(ctx))
            (security(ctx))
            (highload(ctx))
            (design(ctx))
            (process(ctx))
            (lab(ctx))
            (team(ctx))
            (contact(ctx))
        }
        (horizon(ctx))
    }
}

fn hero(ctx: &Ctx) -> Markup {
    let s = ctx.s;
    html! {
        section id="hero" class="hero" aria-labelledby="hero-title" {
            div class="wrap hero__grid" {
                div class="hero__text" {
                    p class="eyebrow hero__eyebrow" {
                        @for (i, part) in s.t("hero-eyebrow").split(" · ").enumerate() {
                            @if i > 0 { span class="hero__sep" aria-hidden="true" { " · " } }
                            span class="hero__part" { (part) }
                        }
                    }
                    h1 id="hero-title" class="hero__title" { (s.t("hero-title")) }
                    p class="hero__sub" { (s.t("hero-sub")) }
                    div class="hero__actions" {
                        a class="btn btn--primary" href="#contact" { (s.t("hero-cta")) }
                        a class="btn btn--ghost" href="#physics" data-action="proof" { (s.t("hero-proof")) }
                    }
                    p class="hero__note" { (rich_kbd(&s.t("hero-note"))) }
                    // The particles move for as long as the hero is on screen, so they can be paused
                    // (WCAG 2.2.2). Shown by web/main.ts once the engine draws: with reduced motion or
                    // without JavaScript the hero is a still poster and needs no control.
                    button class="btn btn--quiet btn--sm hero__motion" type="button" data-action="motion" hidden {
                        span { (s.t("hero-motion-pause")) }
                        span hidden { (s.t("hero-motion-play")) }
                    }
                }
                // The mark under observation: the engine draws it in the figure, inside a viewfinder,
                // and reports what its own frames cost underneath (web/main.ts fills the numbers).
                div class="hero__scope" {
                    div class="hero__figure" data-anchor="hourglass" {
                        div class="hero__poster" { (ctx.mark) }
                    }
                    p class="hero__readout" hidden {
                        span class="hero__live" { (s.t("hero-live")) }
                        span data-r="tier" {}
                        span { span data-r="count" {} " " (s.t("hero-particles")) }
                        span { span data-r="ms" {} " " (s.t("hero-frame")) }
                    }
                }
            }
        }
    }
}

/// "Press **P**" → "Press <kbd>P</kbd>" (the bold syntax marks keys here).
fn rich_kbd(text: &str) -> Markup {
    let html = crate::i18n::rich_markup(text).into_string().replace("<strong>", "<kbd>").replace("</strong>", "</kbd>");
    PreEscaped(html)
}

/// Highlighted code as numbered lines: each line a span, the numbers drawn by CSS counters (so a
/// copy takes only the code). The snippets have no token spanning two lines.
fn numbered(html: &str) -> Markup {
    let mut out = String::with_capacity(html.len() + 64);
    for line in html.split('\n') {
        out.push_str(r#"<span class="code__line">"#);
        out.push_str(line);
        out.push_str("</span>");
    }
    PreEscaped(out)
}

fn physics(ctx: &Ctx) -> Markup {
    let s = ctx.s;
    let origin = (ctx.cfg.company.lat, ctx.cfg.company.lon);
    let rows: Vec<(String, f64, f64)> = geo::CITIES
        .iter()
        .map(|(key, lat, lon)| {
            let km = geo::haversine_km(origin, (*lat, *lon));
            (s.t(key), km, geo::round_trip_floor_ms(km))
        })
        .collect();
    let fallback = html! {
        p class="physics__static-lead" { (s.t("physics-static-lead")) }
        table class="table physics__table" {
            thead { tr {
                th scope="col" { (s.t("physics-static-city")) }
                th scope="col" class="table__num" { (s.t("physics-static-distance")) }
                th scope="col" class="table__num" { (s.t("physics-static-floor")) }
            } }
            tbody {
                @for (city, km, ms) in &rows {
                    tr {
                        th scope="row" { (city) }
                        td class="table__num" { span class="num" { (s.int(km.round() as u64)) span class="num__unit" { (s.unit("km")) } } }
                        td class="table__num" { span class="num" { (s.dec(*ms, 1)) span class="num__unit" { (s.unit("ms")) } } }
                    }
                }
            }
        }
    };
    html! {
        section id="physics" class="sec sec--physics" aria-labelledby="physics-title" {
            div class="wrap" {
                header class="sec__head sec__head--wide" {
                    p class="eyebrow" { (s.t("physics-eyebrow")) }
                    h2 class="sec__title" id="physics-title" { (s.t("physics-title")) }
                    p class="lead" { (s.t("physics-lead")) }
                }
                div class="physics__ray" data-anchor="ray" aria-hidden="true" {}
                (instrument("physics", &s.t("physics-instrument-id"), &s.t("physics-instrument-title"), &s.t("instrument-static"), fallback, Some(html! { (s.t("physics-foot")) })))
            }
        }
    }
}

fn pillars(ctx: &Ctx) -> Markup {
    let s = ctx.s;
    // A star chart: each pillar's label stands beside the figure at its star's height and points at
    // it with a leader line — Betelgeuse and Saiph to the left, Meissa and Rigel to the right.
    // Reading order runs down the sky.
    let cards = [
        ("design", "meissa", "pillar-design", "r"),
        ("highload", "betelgeuse", "pillar-highload", "l"),
        ("efficiency", "rigel", "pillar-efficiency", "r"),
        ("security", "saiph", "pillar-security", "l"),
    ];
    html! {
        section id="pillars" class="sec sec--pillars" aria-labelledby="pillars-title" {
            div class="wrap" {
                header class="sec__head sec__head--center" {
                    p class="eyebrow" { (s.t("pillars-eyebrow")) }
                    h2 class="sec__title" id="pillars-title" { (s.t("pillars-title")) }
                    p class="lead" { (s.t("pillars-lead")) }
                }
                div class="constellation" {
                    @for (section, star, key, area) in cards {
                        a class=(format!("pcard pcard--{area}")) href=(format!("#{section}")) data-star=(star) {
                            span class="pcard__star" {
                                span class="callout" data-star=(star) {
                                    span class="callout__name" { (s.t(&format!("star-{star}"))) }
                                }
                            }
                            span class="pcard__name" { (s.t(key)) }
                            span class="pcard__line" { (s.t(&format!("{key}-line"))) }
                        }
                    }
                    figure class="constellation__figure" data-anchor="orion" aria-hidden="true" {
                        (svg::orion_chart(320.0, &[("meissa", true), ("betelgeuse", false), ("rigel", true), ("saiph", false)]))
                    }
                }
            }
        }
    }
}

fn efficiency(ctx: &Ctx) -> Markup {
    let s = ctx.s;
    let cards = ["algo", "data", "arch", "types", "research", "stack"];
    html! {
        section id="efficiency" class="sec sec--pillar" data-star="rigel" aria-labelledby="efficiency-title" {
            div class="wrap" {
                (pillar_head(ctx, "rigel", "efficiency", "eff-eyebrow", "eff-title", "eff-lead"))
                (instrument("routing", &s.t("eff-race-id"), &s.t("eff-race-title"), &s.t("instrument-static"),
                    html! { p class="instrument__fallback" { (s.t("eff-race-fallback")) } },
                    Some(html! { (s.ta("eff-race-foot", &[("date", ctx.osm_date.into())])) })))
                (spec(&s.t("eff-methods"), cards.iter().map(|c| (s.t(&format!("eff-{c}")), s.t(&format!("eff-{c}-text"))))))
                figure class="code" {
                    div class="code__bar" {
                        span class="code__file mono" { (s.t("eff-code-file")) }
                        span class="code__title" { (s.t("eff-code-title")) }
                    }
                    pre class="code__pre" { code { (numbered(&highlight::rust(EMAIL_SNIPPET))) } }
                    figcaption class="code__caption" { (s.rich("eff-code-caption")) }
                }
            }
        }
    }
}

fn claims_list(ctx: &Ctx, group: &str, class: &str) -> Markup {
    let s = ctx.s;
    html! {
        ul class=(class) role="list" {
            @for claim in ctx.cfg.claims(group, ctx.release) {
                li class="claim" data-verified=(claim.verified) {
                    span class="claim__text" { (s.rich(&claim.key)) }
                    @if !claim.verified {
                        " "
                        span class="badge badge--warn claim__flag" { (s.t("claim-unverified")) }
                    }
                }
            }
        }
    }
}

/// A specification sheet: a small label, then term–description pairs on ruled rows, like the
/// methods section of a lab record.
fn spec(label: &str, rows: impl Iterator<Item = (String, String)>) -> Markup {
    html! {
        div class="spec" {
            p class="spec__label" { (label) }
            (spec_rows(rows))
        }
    }
}

fn spec_rows(rows: impl Iterator<Item = (String, String)>) -> Markup {
    html! {
        dl class="spec__rows" {
            @for (term, text) in rows {
                div class="spec__row" { dt { (term) } dd { (text) } }
            }
        }
    }
}

fn security(ctx: &Ctx) -> Markup {
    let s = ctx.s;
    let fallback = html! { p class="instrument__fallback" { (s.t("sec-instrument-fallback")) } };
    html! {
        section id="security" class="sec sec--pillar" data-star="saiph" aria-labelledby="security-title" {
            div class="wrap" {
                (pillar_head(ctx, "saiph", "security", "sec-eyebrow", "sec-title", "sec-lead"))
                (instrument("fuzzer", &s.t("sec-fuzz-id"), &s.t("sec-fuzz-title"), &s.t("instrument-static"),
                    html! { p class="instrument__fallback" { (s.t("sec-fuzz-fallback")) } },
                    Some(html! { (s.t("sec-fuzz-foot")) })))
                div class="duel" {
                    section class="duel__col duel__col--off" aria-labelledby="sec-off-title" {
                        h3 class="duel__title" id="sec-off-title" { (s.t("sec-offence")) span class="duel__note" { (s.t("sec-offence-note")) } }
                        ul class="duel__list" role="list" { @for i in 1..=6 { li { (s.rich(&format!("sec-off-{i}"))) } } }
                    }
                    section class="duel__col duel__col--def" aria-labelledby="sec-def-title" {
                        h3 class="duel__title" id="sec-def-title" { (s.t("sec-defence")) span class="duel__note" { (s.t("sec-defence-note")) } }
                        ul class="duel__list" role="list" { @for i in 1..=6 { li { (s.rich(&format!("sec-def-{i}"))) } } }
                    }
                }
                div class="armour" {
                    div class="armour__text" {
                        h3 class="armour__title" { (s.t("sec-armour-title")) }
                        p class="armour__lead" { (s.t("sec-armour-lead")) }
                        (claims_list(ctx, "armour", "armour__list"))
                    }
                    (instrument("armour", &s.t("sec-instrument-id"), &s.t("sec-instrument-title"), &s.t("instrument-static"), fallback, None))
                }
                p class="sec__aside" { (s.t("sec-ctf")) " " kbd { "~" } }
            }
        }
    }
}

fn highload(ctx: &Ctx) -> Markup {
    let s = ctx.s;
    let layers = ["software", "hardware", "network", "platform"];
    let trace_fallback = html! { p class="instrument__fallback" { (s.t("hl-trace-fallback")) } };
    let live_fallback = html! { p class="instrument__fallback" { (s.t("hl-live-fallback")) } };
    html! {
        section id="highload" class="sec sec--pillar" data-star="betelgeuse" aria-labelledby="highload-title" {
            div class="wrap" {
                (pillar_head(ctx, "betelgeuse", "highload", "hl-eyebrow", "hl-title", "hl-lead"))
                div class="instruments-2" {
                    (instrument("trace", &s.t("hl-trace-id"), &s.t("hl-trace-title"), &s.t("instrument-static"), trace_fallback, None))
                    (instrument("live", &s.t("hl-live-id"), &s.t("hl-live-title"), &s.t("instrument-static"), live_fallback, None))
                    (instrument("loadsim", &s.t("hl-lab-id"), &s.t("hl-lab-title"), &s.t("instrument-static"),
                        html! { p class="instrument__fallback" { (s.t("hl-lab-fallback")) } }, None))
                }
                div class="layers" role="group" aria-label=(s.t("hl-layers")) {
                    @for l in layers {
                        div class="layers__row" {
                            h3 class="layers__name" { (s.t(&format!("hl-{l}"))) }
                            ul class="layers__items" role="list" {
                                @for item in s.t(&format!("hl-{l}-items")).split(" · ") { li class="tag" { (item) } }
                            }
                        }
                    }
                }
                div class="story" role="note" aria-labelledby="story-title" {
                    (svg::star_mark("betelgeuse"))
                    div {
                        h3 class="story__title" id="story-title" { (s.t("hl-story-title")) }
                        p class="story__text" { (s.t("hl-story")) }
                        p class="source story__source" { (s.t("hl-story-source")) }
                    }
                }
            }
        }
    }
}

fn design(ctx: &Ctx) -> Markup {
    let s = ctx.s;
    let fallback = html! { p class="instrument__fallback" { (s.t("ds-laws-fallback")) } };
    html! {
        section id="design" class="sec sec--pillar" data-star="meissa" aria-labelledby="design-title" {
            div class="wrap" {
                (pillar_head(ctx, "meissa", "design", "ds-eyebrow", "ds-title", "ds-lead"))
                (instrument("laws", &s.t("ds-laws-id"), &s.t("ds-laws-title"), &s.t("instrument-static"), fallback, None))
                // Until a practice is verified, release builds leave the whole block out.
                @if ctx.cfg.claims("practice", ctx.release).next().is_some() {
                    div class="practice" {
                        h3 class="practice__title" { (s.t("ds-practice")) }
                        (claims_list(ctx, "practice", "practice__list"))
                    }
                }
                p class="sec__aside" { (s.t("ds-proof")) }
            }
        }
    }
}

fn process(ctx: &Ctx) -> Markup {
    let s = ctx.s;
    let steps = [("alnitak", "measure"), ("alnilam", "build"), ("mintaka", "prove")];
    let pack = ["perf", "sec", "ux", "ops"];
    html! {
        section id="process" class="sec sec--process" aria-labelledby="process-title" {
            div class="wrap" {
                header class="sec__head sec__head--wide" {
                    p class="eyebrow" { (s.t("pr-eyebrow")) }
                    h2 class="sec__title" id="process-title" { (s.t("pr-title")) }
                }
                ol class="belt" {
                    @for (star, step) in steps {
                        li class="belt__step" data-star=(star) {
                            (svg::star_mark(star))
                            p class="belt__star" { span class="callout__name" { (s.t(&format!("star-{star}"))) } " " span class="belt__meaning" { (s.t(&format!("star-{star}-fact"))) } }
                            h3 class="belt__name" { (s.t(&format!("pr-{step}"))) }
                            p class="belt__text" { (s.t(&format!("pr-{step}-text"))) }
                        }
                    }
                }
                div class="pack" {
                    div class="pack__head" {
                        h3 class="pack__title" { (s.t("pr-pack-title")) }
                        p class="pack__lead" { (s.t("pr-pack-lead")) }
                    }
                    ul class="reports" role="list" {
                        @for p in pack {
                            li class="report" {
                                h4 class="report__title" { (s.t(&format!("pr-pack-{p}"))) }
                                p class="report__body" { (s.t(&format!("pr-pack-{p}-text"))) }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// S11: the published Lab posts (`content/{lang}/lab/`), each linking to its page. A language
/// without posts lists the default language's, says so, and marks them with their language.
fn lab(ctx: &Ctx) -> Markup {
    let s = ctx.s;
    let listed = super::lab::listed(ctx);
    let other = listed.and_then(|(_, l)| (l != ctx.lang).then_some(l));
    html! {
        section id="lab" class="sec sec--lab" aria-labelledby="lab-title" {
            div class="wrap" {
                header class="sec__head sec__head--wide" {
                    p class="eyebrow" { (s.t("lab-eyebrow")) }
                    h2 class="sec__title" id="lab-title" { (s.t("lab-title")) }
                    p class="lead" { (s.t("lab-lead")) }
                }
                @if let Some((posts, posts_lang)) = listed {
                    @if other.is_some() { p class="lab__note" { (s.t("lab-untranslated")) } }
                    ol class="lab" role="list" {
                        @for post in posts {
                            li class="lab__post" {
                                span class="lab__pillar mono" { (s.t(&post.pillar.label_key())) }
                                a class="lab__title" href=(ctx.url(&post.path(posts_lang))) hreflang=[other] lang=[other] { (post.title) }
                                time class="lab__state" datetime=(post.date) { (post.date) }
                            }
                        }
                    }
                    p class="lab__more" {
                        a href=(ctx.url(&crate::lab::index_path(ctx.lang))) { (s.t("lab-all")) }
                        span aria-hidden="true" { " · " }
                        a href=(ctx.url(&crate::lab::feed_path(posts_lang))) type="application/atom+xml" hreflang=[other] { (s.t("lab-feed")) }
                    }
                }
            }
        }
    }
}

fn team(ctx: &Ctx) -> Markup {
    let s = ctx.s;
    html! {
        section id="team" class="sec sec--team" data-star="bellatrix" aria-labelledby="team-title" {
            div class="wrap" {
                (pillar_head(ctx, "bellatrix", "team", "team-eyebrow", "team-title", "team-body"))
                @if !ctx.cfg.team.is_empty() {
                    ul class="team-grid" role="list" {
                        @for m in &ctx.cfg.team {
                            (member(ctx, m))
                        }
                    }
                }
                div class="name-story" {
                    h3 class="name-story__title" { (s.t("team-name-title")) }
                    p { (s.t("team-name-star")) }
                    p { (word_tg(&s.t("team-name-word"))) }
                    p { (s.t("team-name-pass")) }
                    p class="name-story__end" { (s.t("team-name-end")) }
                }
                div id="careers" class="careers" {
                    p class="careers__line" { (s.t("team-careers")) }
                    a class="btn btn--ghost" href=(ctx.cfg.contact.careers().0) { (s.t("team-join")) }
                }
            }
        }
    }
}

/// The end of the bench: a window onto the night over the Pamir, with Orion in it.
fn horizon(ctx: &Ctx) -> Markup {
    let (ridge, _summit) = svg::pamir(1200.0, 220.0);
    html! {
        div class="horizon" {
            div class="horizon__sky" aria-hidden="true" { (svg::orion_figure(220.0)) }
            (ridge)
            p class="horizon__caption" { (ctx.s.t("team-peak")) }
        }
    }
}

/// One card of the team grid (spec §S12): portrait, name, role, one line, links.
fn member(ctx: &Ctx, m: &Member) -> Markup {
    let s = ctx.s;
    let name = s.t(&format!("team-{}-name", m.id));
    let host = m.site.trim_start_matches("https://").split('/').next().unwrap_or("");
    let photo_key = format!("team-{}-photo", m.id);
    let alt = if s.has(&photo_key) { s.t(&photo_key) } else { s.ta("team-photo-alt", &[("name", name.clone().into())]) };
    html! {
        li class="member" {
            @if let Some(p) = ctx.portraits.get(&m.id) {
                picture class="member__photo" {
                    source type="image/avif" srcset=(p.avif);
                    img src=(p.webp) width="224" height="224" loading="lazy" decoding="async" alt=(alt);
                }
            }
            div class="member__text" {
                h3 class="member__name" { (name) }
                p class="member__role" { (s.t(&format!("team-{}-role", m.id))) }
                p class="member__line" { (s.t(&format!("team-{}-line", m.id))) }
                ul class="member__links" role="list" {
                    li { a href=(format!("https://github.com/{}", m.github)) { "github.com/" (m.github) } }
                    @if !host.is_empty() {
                        li { a href=(m.site) { (host) } }
                    }
                }
            }
        }
    }
}

/// Mark the Tajik word in the name story with `lang="tg"` (correct pronunciation and font).
fn word_tg(text: &str) -> Markup {
    let word = "минтақа";
    match text.find(word) {
        Some(i) => html! { (&text[..i]) span lang="tg" { (word) } (&text[i + word.len()..]) },
        None => html! { (text) },
    }
}

fn contact(ctx: &Ctx) -> Markup {
    let s = ctx.s;
    let c = &ctx.cfg.contact;
    let kinds = ["new-product", "performance", "security-audit", "design", "ai", "not-sure"];
    let timelines = ["asap", "1-3-months", "later", "exploring"];
    let field = |name: &str, label: Markup, input: Markup, hint: Option<String>| -> Markup {
        html! {
            div class="field" data-field=(name) {
                label class="field__label" for=(format!("f-{name}")) { (label) }
                @if let Some(h) = hint { p class="field__hint" id=(format!("h-{name}")) { (h) } }
                (input)
                p class="field__error" id=(format!("e-{name}")) hidden {}
            }
        }
    };
    let optional = html! { " " span class="field__optional" { "(" (s.t("ct-optional")) ")" } };
    html! {
        section id="contact" class="sec sec--contact" aria-labelledby="contact-title" {
            div class="wrap contact-grid" {
                header class="sec__head" {
                    p class="eyebrow" { (s.t("ct-eyebrow")) }
                    h2 class="sec__title" id="contact-title" { (s.t("ct-title")) }
                    p class="lead" { (s.ta("ct-lead", &[("within", c.reply_within.as_str().into())])) }
                    div class="direct" {
                        p class="direct__title" { (s.t("ct-direct")) }
                        ul class="direct__list" role="list" {
                            @for (href, text) in c.direct() { li { a href=(href) { (text) } } }
                        }
                    }
                    p class="contact__privacy" { (s.t("ct-privacy")) }
                }
                form id="contact-form" class="contact" data-island="contact" method="post" action=(ctx.url("/api/contact")) {
                    input type="hidden" name="lang" value=(ctx.lang);
                    fieldset class="contact__group" data-field="kind" {
                        legend class="field__label" { (s.t("ct-kind")) }
                        div class="chips" {
                            @for k in kinds {
                                label class="chip" { input class="visually-hidden" type="checkbox" name="kind" value=(k); (s.t(&format!("ct-kind-{k}"))) }
                            }
                        }
                    }
                    (field("message", html! { (s.t("ct-message")) (optional) },
                        html! { textarea class="input" id="f-message" name="message" maxlength="5000" rows="5" aria-describedby="h-message e-message" {} },
                        Some(s.t("ct-message-hint"))))
                    fieldset class="contact__group" data-field="timeline" {
                        legend class="field__label" { (s.t("ct-timeline")) (optional) }
                        div class="chips" {
                            @for t in timelines {
                                label class="chip" { input class="visually-hidden" type="radio" name="timeline" value=(t); (s.t(&format!("ct-timeline-{t}"))) }
                            }
                        }
                    }
                    div class="contact__row" {
                        (field("name", html! { (s.t("ct-name")) },
                            html! { input class="input" id="f-name" name="name" required maxlength="120" autocomplete="name" aria-describedby="e-name"; },
                            None))
                        (field("company", html! { (s.t("ct-company")) (optional) },
                            html! { input class="input" id="f-company" name="company" maxlength="120" autocomplete="organization" aria-describedby="e-company"; },
                            None))
                    }
                    p class="contact__rule" id="h-contact" { (s.t("ct-contact-rule")) }
                    div class="contact__row" {
                        (field("email", html! { (s.t("ct-email")) },
                            html! { input class="input" id="f-email" name="email" type="email" maxlength="254" autocomplete="email" aria-describedby="h-contact e-email"; },
                            None))
                        (field("telegram", html! { (s.t("ct-telegram")) },
                            html! { input class="input" id="f-telegram" name="telegram" maxlength="33" autocomplete="off" spellcheck="false" autocapitalize="none" placeholder="@" aria-describedby="h-telegram h-contact e-telegram"; },
                            Some(s.t("ct-telegram-hint"))))
                    }
                    p class="field__error" id="e-contact" hidden {}
                    div class="contact__trap" aria-hidden="true" {
                        label { (s.t("ct-trap")) input name="website" tabindex="-1" autocomplete="off"; }
                    }
                    p class="contact__status" id="contact-status" role="status" aria-live="polite" {}
                    button class="btn btn--primary contact__send" type="submit" { (s.t("ct-send")) }
                }
            }
        }
    }
}
