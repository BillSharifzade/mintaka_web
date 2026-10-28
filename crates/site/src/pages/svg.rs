//! Inline SVG: brand files from the kit, and figures computed from data (Orion, the Pamir).

use crate::i18n::Strings;
use crate::orion::{self, Tint};
use anyhow::{Context, Result};
use maud::{Markup, PreEscaped, html};
use std::fmt::Write as _;
use std::path::Path;

/// A brand-kit SVG, stripped of its `<title>`, made decorative or labelled, with a class.
pub fn brand(root: &Path, file: &str, class: &str, label: Option<&str>) -> Result<Markup> {
    let path = root.join("assets/brand/logo").join(file);
    let src = std::fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    let mut svg = src.trim().to_owned();
    if let (Some(a), Some(b)) = (svg.find("<title>"), svg.find("</title>")) {
        svg.replace_range(a..b + "</title>".len(), "");
    }
    // The kit marks every file role="img" aria-label="…"; restate it to fit the context.
    svg = svg.replacen(r#" role="img""#, "", 1);
    if let Some(a) = svg.find(r#" aria-label=""#) {
        let end = svg[a + 13..].find('"').map(|e| a + 13 + e + 1).unwrap_or(a);
        svg.replace_range(a..end, "");
    }
    // The kit uses ids like "mark" and "betelgeuse"; drop them so inlined copies never collide.
    for id in [r#" id="mark""#, r#" id="betelgeuse""#, r#" id="mintaka""#] {
        svg = svg.replace(id, "");
    }
    let attrs = match label {
        Some(l) => format!(r#"<svg class="{class}" role="img" aria-label="{}" focusable="false""#, crate::util::escape_xml(l)),
        None => format!(r#"<svg class="{class}" aria-hidden="true" focusable="false""#),
    };
    Ok(PreEscaped(svg.replacen("<svg", &attrs, 1)))
}

/// A brand-kit SVG as a `<symbol>` (defined once per page) and the `<svg><use>` that shows it.
pub fn brand_symbol(root: &Path, file: &str, id: &str) -> Result<(Markup, String)> {
    let path = root.join("assets/brand/logo").join(file);
    let src = std::fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    let view_box = src
        .split_once(r#"viewBox=""#)
        .and_then(|(_, rest)| rest.split_once('"'))
        .map(|(vb, _)| vb.to_owned())
        .context("brand SVG without a viewBox")?;
    let open_end = src.find('>').context("malformed SVG")? + 1;
    let close = src.rfind("</svg>").context("malformed SVG")?;
    let mut inner = src[open_end..close].to_owned();
    if let (Some(a), Some(b)) = (inner.find("<title>"), inner.find("</title>")) {
        inner.replace_range(a..b + "</title>".len(), "");
    }
    for attr in [r#" id="mark""#, r#" id="betelgeuse""#, r#" id="mintaka""#] {
        inner = inner.replace(attr, "");
    }
    let symbol = PreEscaped(format!(r#"<symbol id="{id}" viewBox="{view_box}">{inner}</symbol>"#));
    Ok((symbol, view_box))
}

/// Show a symbol defined by [`brand_symbol`].
pub fn use_symbol(id: &str, view_box: &str, class: &str) -> Markup {
    html! {
        svg class=(class) viewBox=(view_box) aria-hidden="true" focusable="false" { use href=(format!("#{id}")) {} }
    }
}

fn tint_class(t: Tint) -> &'static str {
    match t {
        Tint::Rust => "star star--rust",
        Tint::Blue => "star star--blue",
        Tint::Starlight => "star",
    }
}

/// The full Orion figure for S3 (and the anchor the hero engine lands its particles on).
pub fn orion_figure(width: f64) -> Markup {
    let (stars, height) = orion::layout(width, 28.0);
    let pos = |id: &str| stars.iter().find(|(s, ..)| s.id == id).map(|(_, x, y, _)| (*x, *y)).unwrap_or((0.0, 0.0));
    let mut lines = String::new();
    for (a, b, dashed) in orion::LINES {
        let ((x1, y1), (x2, y2)) = (pos(a), pos(b));
        let _ = write!(
            lines,
            r#"<line class="orion-line{}" x1="{x1:.1}" y1="{y1:.1}" x2="{x2:.1}" y2="{y2:.1}"/>"#,
            if dashed { " orion-line--belt" } else { "" }
        );
    }
    html! {
        svg class="orion" viewBox=(format!("0 0 {width:.0} {height:.0}")) aria-hidden="true" focusable="false" {
            g class="orion-lines" { (PreEscaped(lines)) }
            @for (s, x, y, r) in &stars {
                circle class=(tint_class(s.tint)) data-star=(s.id) cx=(format!("{x:.1}")) cy=(format!("{y:.1}")) r=(format!("{r:.1}")) {}
            }
        }
    }
}

/// The fixed Orion navigation: each star is a real link (works without JavaScript).
pub fn orion_nav(s: &Strings) -> Markup {
    let (stars, height) = orion::layout(76.0, 9.0);
    let pos = |id: &str| stars.iter().find(|(st, ..)| st.id == id).map(|(_, x, y, r)| (*x, *y, *r)).unwrap_or_default();
    let mut lines = String::new();
    for (a, b, _) in orion::LINES {
        let ((x1, y1, _), (x2, y2, _)) = (pos(a), pos(b));
        let _ = write!(lines, r#"<line x1="{x1:.1}" y1="{y1:.1}" x2="{x2:.1}" y2="{y2:.1}"/>"#);
    }
    let link = |id: &str, label_key: &str, href: &str| -> Markup {
        let st = orion::star(id);
        let (x, y, r) = pos(id);
        let r = r.max(2.2);
        html! {
            a href=(href) class="orion-nav__star" data-star=(id) data-section=(st.section) aria-label=(s.t(label_key)) {
                // 24 px targets (WCAG 2.5.8); neighbouring stars are ≥ 24.8 px apart, so none overlap.
                circle class="orion-nav__hit" cx=(format!("{x:.1}")) cy=(format!("{y:.1}")) r="12" {}
                circle class=(tint_class(st.tint)) cx=(format!("{x:.1}")) cy=(format!("{y:.1}")) r=(format!("{r:.1}")) {}
            }
        }
    };
    let belt: Vec<(f64, f64, f64)> = ["alnitak", "alnilam", "mintaka"].iter().map(|id| pos(id)).collect();
    html! {
        nav class="orion-nav" aria-label=(s.t("orion-label")) {
            svg viewBox=(format!("0 0 76 {height:.0}")) focusable="false" {
                g class="orion-nav__lines" aria-hidden="true" { (PreEscaped(lines)) }
                (link("betelgeuse", "orion-betelgeuse", "#highload"))
                (link("meissa", "orion-meissa", "#design"))
                (link("bellatrix", "orion-bellatrix", "#team"))
                a href="#process" class="orion-nav__star" data-star="belt" data-section="process" aria-label=(s.t("orion-belt")) {
                    @for (i, (x, y, r)) in belt.iter().enumerate() {
                        circle class="orion-nav__hit" cx=(format!("{x:.1}")) cy=(format!("{y:.1}")) r="10" {}
                        circle class=(if i == 2 { "star star--blue" } else { "star" }) cx=(format!("{x:.1}")) cy=(format!("{y:.1}")) r=(format!("{:.1}", r.max(2.2))) {}
                    }
                }
                (link("saiph", "orion-saiph", "#security"))
                (link("rigel", "orion-rigel", "#efficiency"))
            }
        }
    }
}

/// A star drawn the way the construction board draws Betelgeuse and Mintaka: disc + dashed ring.
pub fn star_mark(id: &str) -> Markup {
    let st = orion::star(id);
    let r = (st.r * 0.9).clamp(5.0, 18.0);
    html! {
        svg class="star-mark" viewBox="0 0 48 48" aria-hidden="true" focusable="false" {
            circle class="star-mark__ring" cx="24" cy="24" r=(format!("{:.1}", r + 5.0)) {}
            circle class=(tint_class(st.tint)) cx="24" cy="24" r=(format!("{r:.1}")) {}
        }
    }
}

/// An illustrated Pamir ridge: deterministic midpoint displacement (not a real skyline).
/// Returns the markup and the summit's x position (0–1).
pub fn pamir(width: f64, height: f64) -> (Markup, f64) {
    let n = 128;
    let mut ys = vec![0.0f64; n + 1];
    ys[0] = 0.55;
    ys[n] = 0.6;
    let mut seed: u32 = 0x9E37_79B9;
    let mut rand = move || {
        seed ^= seed << 13;
        seed ^= seed >> 17;
        seed ^= seed << 5;
        (seed as f64 / u32::MAX as f64) * 2.0 - 1.0
    };
    let mut step = n;
    let mut amp = 0.42;
    while step > 1 {
        let half = step / 2;
        let mut i = half;
        while i < n {
            ys[i] = (ys[i - half] + ys[i + half]) / 2.0 + rand() * amp;
            i += step;
        }
        amp *= 0.56;
        step = half;
    }
    // Lift a summit two-thirds of the way along and keep everything in range.
    let summit = (n as f64 * 0.64) as usize;
    for (i, y) in ys.iter_mut().enumerate() {
        let d = (i as f64 - summit as f64).abs() / n as f64;
        *y = (*y - (0.38 * (-d * d * 90.0).exp())).clamp(0.08, 0.92);
    }
    // The summit sits exactly at 64 % of the width (the caption is placed there by CSS).
    let lowest = ys.iter().cloned().fold(f64::MAX, f64::min);
    ys[summit] = lowest - 0.03;
    let mut d = format!("M0 {height:.0}");
    for (i, y) in ys.iter().enumerate() {
        let _ = write!(d, " L{:.1} {:.1}", i as f64 / n as f64 * width, y * height);
    }
    let _ = write!(d, " L{width:.0} {height:.0} Z");
    let markup = html! {
        svg class="pamir" viewBox=(format!("0 0 {width:.0} {height:.0}")) preserveAspectRatio="none" aria-hidden="true" focusable="false" {
            path class="pamir__ridge" d=(d) {}
        }
    };
    (markup, summit as f64 / n as f64)
}
