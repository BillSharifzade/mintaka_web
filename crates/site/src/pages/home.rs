//! The lab (`/{lang}/`): nothing but experiments. It opens with one already running — shortest
//! paths across Dushanbe's streets, in the visitor's browser (web/city/) — then the eight
//! instruments, each a station with what it tests and where it runs, then the lab notes.

use super::{Ctx, instrument};
use crate::geo;
use maud::{Markup, PreEscaped, html};

pub fn body(ctx: &Ctx) -> Markup {
    html! {
        (hero(ctx))
        (instruments(ctx))
        (notes(ctx))
    }
}

/// "Press **P**" → "Press <kbd>P</kbd>" (the bold syntax marks keys here).
fn rich_kbd(text: &str) -> Markup {
    let html = crate::i18n::rich_markup(text).into_string().replace("<strong>", "<kbd>").replace("</strong>", "</kbd>");
    PreEscaped(html)
}

/// The opening experiment. The map and its numbers are filled in by web/city/; without a script
/// (or with Data Saver) the words stand on their own over the dark grid.
fn hero(ctx: &Ctx) -> Markup {
    let s = ctx.s;
    let stat = |key: &str, k: &str, unit: Option<&str>| {
        html! {
            div class="stat" {
                dt { (s.t(key)) }
                dd { span data-c=(k) { "—" } @if let Some(u) = unit { " " span class="stat__unit" { (u) } } }
            }
        }
    };
    let ms = s.unit("ms");
    html! {
        section id="hero" class="lab-hero" aria-labelledby="hero-title" {
            div class="lab-hero__map" {}
            div class="wrap lab-hero__in" {
                div class="lab-hero__text" {
                    p class="lab-hero__label" {
                        span class="lamp lamp--live" aria-hidden="true" {}
                        (s.ta("lab-hero-label", &[("date", ctx.osm_date.into())]))
                    }
                    h1 id="hero-title" class="lab-hero__title" { (s.t("hero-title")) }
                    p class="lab-hero__sub" { (s.t("lab-hero-sub")) }
                    p class="lab-hero__hint lab-hero__hint--mouse" { (s.t("lab-hero-hint")) }
                    p class="lab-hero__hint lab-hero__hint--touch" { (s.t("lab-hero-hint-touch")) }
                    div class="lab-hero__actions" {
                        a class="btn btn--primary" href="#instruments" { (s.t("lab-hero-open")) }
                        // The searches run on their own for as long as the map is on screen, so they
                        // can be paused (WCAG 2.2.2). Shown by web/main.ts once the map runs.
                        button class="btn btn--ghost lab-hero__motion" type="button" data-action="motion" hidden {
                            span { (s.t("lab-hero-pause")) }
                            span hidden { (s.t("lab-hero-play")) }
                        }
                    }
                    p class="lab-hero__note" { (rich_kbd(&s.t("hero-note"))) }
                }
                div class="lab-hero__panel" {
                    dl class="lab-hero__stats" {
                        (stat("lab-stat-searches", "searches", None))
                        (stat("lab-stat-astar", "astar", Some(ms)))
                        (stat("lab-stat-dijkstra", "dijkstra", Some(ms)))
                        (stat("lab-stat-settled", "settled", None))
                        (stat("lab-stat-frame", "frame", Some(ms)))
                    }
                    p class="lab-hero__probe" data-c="probe" aria-live="off" {}
                    ol class="lab-hero__log" aria-label=(s.t("lab-hero-log")) {}
                }
            }
        }
    }
}

/// One instrument's station: where it sits, what it is called, what it tests.
struct Station {
    /// Anchor. The first station of each discipline keeps that discipline's old anchor, so links
    /// from the studio page (`#efficiency`, …) still land on its experiment.
    anchor: &'static str,
    island: &'static str,
    discipline: &'static str,
    name: &'static str,
    plate: &'static str,
    title: &'static str,
    tests: &'static str,
    runs: &'static str,
}

const STATIONS: [Station; 8] = [
    Station { anchor: "physics", island: "physics", discipline: "lab-d-physics", name: "lab-n-physics", plate: "physics-instrument-id", title: "physics-instrument-title", tests: "lab-t-physics", runs: "lab-runs-both" },
    Station { anchor: "efficiency", island: "routing", discipline: "nav-efficiency", name: "lab-n-routing", plate: "eff-race-id", title: "eff-race-title", tests: "lab-t-routing", runs: "lab-runs-browser" },
    Station { anchor: "security", island: "fuzzer", discipline: "nav-security", name: "lab-n-fuzzer", plate: "sec-fuzz-id", title: "sec-fuzz-title", tests: "lab-t-fuzzer", runs: "lab-runs-browser" },
    Station { anchor: "armour", island: "armour", discipline: "nav-security", name: "lab-n-armour", plate: "sec-instrument-id", title: "sec-instrument-title", tests: "lab-t-armour", runs: "lab-runs-both" },
    Station { anchor: "highload", island: "trace", discipline: "nav-highload", name: "lab-n-trace", plate: "hl-trace-id", title: "hl-trace-title", tests: "lab-t-trace", runs: "lab-runs-both" },
    Station { anchor: "live", island: "live", discipline: "nav-highload", name: "lab-n-live", plate: "hl-live-id", title: "hl-live-title", tests: "lab-t-live", runs: "lab-runs-server" },
    Station { anchor: "overload", island: "loadsim", discipline: "nav-highload", name: "lab-n-loadsim", plate: "hl-lab-id", title: "hl-lab-title", tests: "lab-t-loadsim", runs: "lab-runs-sim" },
    Station { anchor: "design", island: "laws", discipline: "nav-design", name: "lab-n-laws", plate: "ds-laws-id", title: "ds-laws-title", tests: "lab-t-laws", runs: "lab-runs-browser" },
];

/// The instruments: a rail that names them all (and lights up as each one runs), and the stations.
fn instruments(ctx: &Ctx) -> Markup {
    let s = ctx.s;
    html! {
        section id="instruments" class="bench" aria-labelledby="instruments-title" {
            div class="wrap bench__grid" {
                nav class="rail" aria-labelledby="instruments-title" {
                    h2 class="rail__title" id="instruments-title" { (s.t("lab-inst-title")) }
                    ol class="rail__list" role="list" {
                        @for (i, st) in STATIONS.iter().enumerate() {
                            li {
                                a class="rail__link" href=(format!("#{}", st.anchor)) data-for=(st.island) {
                                    span class="rail__code" { (format!("{:02}", i + 1)) }
                                    span class="rail__name" { (s.t(st.name)) }
                                    span class="lamp" aria-hidden="true" {}
                                }
                            }
                        }
                    }
                    p class="rail__lead" { (s.t("lab-inst-lead")) }
                }
                div class="bench__stations" {
                    @for (i, st) in STATIONS.iter().enumerate() {
                        (station(ctx, i + 1, st))
                    }
                }
            }
        }
    }
}

fn station(ctx: &Ctx, n: usize, st: &Station) -> Markup {
    let s = ctx.s;
    let title_id = format!("{}-station", st.anchor);
    let (body, foot) = match st.island {
        "physics" => (physics_fallback(ctx), Some(html! { (s.t("physics-foot")) })),
        "routing" => (fallback(&s.t("eff-race-fallback")), Some(html! { (s.ta("eff-race-foot", &[("date", ctx.osm_date.into())])) })),
        "fuzzer" => (fallback(&s.t("sec-fuzz-fallback")), Some(html! { (s.t("sec-fuzz-foot")) })),
        "armour" => (fallback(&s.t("sec-instrument-fallback")), None),
        "trace" => (fallback(&s.t("hl-trace-fallback")), None),
        "live" => (fallback(&s.t("hl-live-fallback")), None),
        "loadsim" => (fallback(&s.t("hl-lab-fallback")), None),
        _ => (fallback(&s.t("ds-laws-fallback")), None),
    };
    html! {
        section id=(st.anchor) class="station" aria-labelledby=(title_id) {
            header class="station__head" {
                p class="station__meta" {
                    span class="station__code" { (format!("{n:02}")) }
                    span { (s.t(st.discipline)) }
                    span class="station__runs" { (s.t(st.runs)) }
                }
                h2 class="station__title" id=(title_id) { (s.t(st.name)) }
                p class="station__tests" { (s.t(st.tests)) }
            }
            (instrument(st.island, &s.t(st.plate), &s.t(st.title), &s.t("instrument-static"), body, foot))
        }
    }
}

fn fallback(text: &str) -> Markup {
    html! { p class="instrument__fallback" { (text) } }
}

/// Without JavaScript the physics station still answers: the floor on a round trip from Dushanbe
/// to each city, from the great-circle distance and the speed of light in fibre.
fn physics_fallback(ctx: &Ctx) -> Markup {
    let s = ctx.s;
    let origin = (ctx.cfg.company.lat, ctx.cfg.company.lon);
    let rows: Vec<(String, f64, f64)> = geo::CITIES
        .iter()
        .map(|(key, lat, lon)| {
            let km = geo::haversine_km(origin, (*lat, *lon));
            (s.t(key), km, geo::round_trip_floor_ms(km))
        })
        .collect();
    html! {
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
    }
}

/// The lab notes (`content/{lang}/lab/`), each linking to its page. A language without posts
/// lists the default language's, says so, and marks them with their language.
fn notes(ctx: &Ctx) -> Markup {
    let s = ctx.s;
    let listed = super::lab::listed(ctx);
    let other = listed.and_then(|(_, l)| (l != ctx.lang).then_some(l));
    html! {
        section id="notes" class="notes" aria-labelledby="notes-title" {
            div class="wrap" {
                header class="notes__head" {
                    p class="station__meta" { span { (s.t("lab-eyebrow")) } }
                    h2 class="notes__title" id="notes-title" { (s.t("nav-notes")) }
                    p class="notes__lead" { (s.t("lab-lead")) }
                }
                @if let Some((posts, posts_lang)) = listed {
                    @if other.is_some() { p class="notes__untranslated" { (s.t("lab-untranslated")) } }
                    ol class="notes__list" role="list" {
                        @for post in posts {
                            li class="note" {
                                time class="note__date" datetime=(post.date) { (post.date) }
                                span class="note__pillar" { (s.t(&post.pillar.label_key())) }
                                a class="note__title" href=(ctx.url(&post.path(posts_lang))) hreflang=[other] lang=[other] { (post.title) }
                            }
                        }
                    }
                    p class="notes__more" {
                        a href=(ctx.url(&crate::lab::index_path(ctx.lang))) { (s.t("lab-all")) }
                        span aria-hidden="true" { " · " }
                        a href=(ctx.url(&crate::lab::feed_path(posts_lang))) type="application/atom+xml" hreflang=[other] { (s.t("lab-feed")) }
                    }
                }
            }
        }
    }
}
