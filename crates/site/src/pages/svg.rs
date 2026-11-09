//! Inline SVG: brand files from the kit, and figures computed from data (Orion, the Pamir).

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

/// A brand-kit SVG inlined with its three colours turned into classes — `lg-s` (Starlight: the
/// shape and the letters), `lg-b` (Betelgeuse) and `lg-m` (Mintaka) — so a stylesheet can switch it
/// between the kit's colourways (on-dark, on-light, on-blue, on-rust) without a second file.
/// Decorative (`aria-hidden`): the element around it carries the name.
pub fn brand_classed(root: &Path, file: &str, class: &str) -> Result<Markup> {
    let svg = brand(root, file, class, None)?.into_string();
    let svg = svg
        .replace(r#" id="wordmark""#, "")
        .replace(r##"fill="#F4F5F7""##, r#"class="lg-s""#)
        .replace(r##"fill="#E2552D""##, r#"class="lg-b""#)
        .replace(r##"fill="#3D7BFF""##, r#"class="lg-m""#);
    if svg.contains("fill=\"#") {
        anyhow::bail!("{file}: a colour outside the brand palette — brand_classed knows Starlight, Betelgeuse and Mintaka");
    }
    Ok(PreEscaped(svg))
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
    // The kit's three colours become classes (as in `brand_classed`), so each copy takes the
    // colours of the room it is shown in: `.lg-s` follows `--logo-ink` (layout.css).
    let inner = inner
        .replace(r##"fill="#F4F5F7""##, r#"class="lg-s""#)
        .replace(r##"fill="#E2552D""##, r#"class="lg-b""#)
        .replace(r##"fill="#3D7BFF""##, r#"class="lg-m""#);
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

