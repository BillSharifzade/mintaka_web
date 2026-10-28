//! Static site generator for mintaka.tj.
//!
//! Input: `content/` (strings, config), `web/styles/` (CSS), esbuild's output (JS), `assets/`
//! (fonts, brand files). Output: `dist/` plus `dist/manifest.json` for the server
//! (docs/CONTRACTS.md §3). Run through `cargo xtask build`.

pub mod assets;
pub mod budget;
pub mod config;
pub mod css;
pub mod fonts;
pub mod geo;
pub mod highlight;
pub mod i18n;
pub mod js;
pub mod orion;
pub mod pages;
pub mod text;
pub mod util;
pub mod well_known;

use anyhow::{Context, Result, bail};
use assets::{Dist, content_type_for};
use budget::Budget;
use config::SiteConfig;
use fonts::Fonts;
use i18n::Strings;
use pages::{Ctx, Page, document, home, misc, svg};
use proof::{CachePolicy, Csp, MANIFEST_VERSION, Manifest, RouteKind};
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

pub struct BuildOptions {
    /// Repository root (where `content/`, `web/`, `assets/` live).
    pub root: PathBuf,
    /// Output directory, normally `dist/`.
    pub out: PathBuf,
    /// esbuild's output directory (with `meta.json`).
    pub js_dir: PathBuf,
    /// Release builds drop unverified claims and fail on budget, coverage or launch issues.
    pub release: bool,
}

pub struct Report {
    pub build: String,
    pub budget: Budget,
    pub warnings: Vec<String>,
    pub routes: usize,
}

const HTML: &str = "text/html; charset=utf-8";

pub fn build(opts: &BuildOptions) -> Result<Report> {
    let root = &opts.root;
    let cfg = SiteConfig::load(root)?;
    let mut dist = Dist::create(&opts.out)?;
    let mut warnings = vec![];
    let today = util::today_utc();
    let date = util::iso_date(today);
    let days = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs() / 86_400).unwrap_or(0) as i64;

    // Fonts, scripts, styles.
    let fonts = Fonts::process(root, &mut dist)?;
    let js = js::import(&opts.js_dir, &mut dist)?;
    let islands_css = css::bundle(&root.join("web/styles/islands.css"))?;
    let islands_css_url = dist.add_hashed("islands", "css", islands_css.as_bytes(), content_type_for("css"))?;
    let page_css = format!("{}{}", css::bundle(&root.join("web/styles/page.css"))?, css::minify(&fonts.css)?);
    let style_hash = util::csp_hash(page_css.as_bytes());

    // Brand files at fixed URLs (browsers and crawlers ask for these by name).
    for (src, url) in [
        ("icons/favicon.ico", "/favicon.ico"),
        ("icons/favicon.svg", "/favicon.svg"),
        ("icons/apple-touch-icon.png", "/apple-touch-icon.png"),
        ("icons/icon-192.png", "/icon-192.png"),
        ("icons/icon-512.png", "/icon-512.png"),
        ("icons/icon-512.svg", "/icon-512.svg"),
        ("social/og-image-1200x630.png", "/og-image.png"),
    ] {
        let bytes = std::fs::read(root.join("assets/brand").join(src)).with_context(|| format!("reading brand file {src}"))?;
        let ext = url.rsplit('.').next().unwrap_or("");
        dist.add(url, url.trim_start_matches('/'), &bytes, content_type_for(ext), CachePolicy::Short, RouteKind::File, None)?;
    }

    // The build id: date + hash of everything that shapes the pages.
    let mut fingerprint = page_css.clone().into_bytes();
    fingerprint.extend(js.main.as_bytes());
    fingerprint.extend(islands_css_url.as_bytes());
    for lang in &cfg.langs {
        if let Ok(t) = std::fs::read(root.join("content").join(lang).join("page.ftl")) {
            fingerprint.extend(t);
        }
    }
    fingerprint.extend(std::fs::read(root.join("content/site.toml")).unwrap_or_default());
    let build = format!("{date}.{}", util::short_hash(&fingerprint).to_lowercase());

    let (sprite, logo_vb) = svg::brand_symbol(root, "mintaka-logo-horizontal-on-dark.svg", "logo")?;
    let logo = svg::use_symbol("logo", &logo_vb, "logo");
    let mark = svg::brand(root, "mintaka-mark-full-on-dark.svg", "mark", None)?;

    let mut named = js.named.clone();
    named.insert("islands.css".into(), islands_css_url.clone());

    // WebAssembly instruments built by xtask into web/wasm/<name>/<name>_bg.wasm.
    let mut wasm_sizes = vec![];
    if let Ok(dirs) = std::fs::read_dir(root.join("web/wasm")) {
        let mut dirs: Vec<_> = dirs.filter_map(|d| d.ok().map(|d| d.path())).filter(|p| p.is_dir()).collect();
        dirs.sort();
        for dir in dirs {
            let name = dir.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
            let path = dir.join(format!("{name}_bg.wasm"));
            let Ok(bytes) = std::fs::read(&path) else { continue };
            let url = dist.add_hashed(&name, "wasm", &bytes, content_type_for("wasm"))?;
            named.insert(format!("{name}.wasm"), url);
            wasm_sizes.push((name, bytes.len() as u64));
        }
    }
    // Data files the instruments fetch lazily (e.g. the Dushanbe road graph).
    if let Ok(files) = std::fs::read_dir(root.join("assets/data")) {
        let mut files: Vec<_> = files.filter_map(|d| d.ok().map(|d| d.path())).filter(|p| p.extension().is_some_and(|e| e == "bin")).collect();
        files.sort();
        for path in files {
            let stem = path.file_stem().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
            let bytes = std::fs::read(&path)?;
            let url = dist.add_hashed(&format!("data/{stem}"), "bin", &bytes, "application/octet-stream")?;
            named.insert(format!("{stem}.bin"), url);
            wasm_sizes.push((format!("{stem}.bin"), bytes.len() as u64));
        }
    }
    let assets_json = util::json_for_html(&serde_json::to_value(&named)?);

    let osm_date = std::fs::read_to_string(root.join("assets/data/dushanbe.meta.json"))
        .ok()
        .and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok())
        .and_then(|m| m["osm_timestamp"].as_str().map(|s| s.chars().take(10).collect::<String>()))
        .unwrap_or_else(|| "—".into());
    let mut text_map = BTreeMap::new();
    let mut not_found = BTreeMap::new();
    let mut gaps: BTreeMap<char, BTreeSet<String>> = BTreeMap::new();
    let host = cfg.origin.trim_start_matches("https://").trim_start_matches("http://").to_owned();

    for lang in &cfg.langs {
        let s = Strings::load(root, lang)?;
        let islands = i18n::island_strings(root, lang)?;
        for (key, value) in &islands {
            for c in fonts.uncovered_text(value) {
                gaps.entry(c).or_default().insert(format!("{lang}:{key}"));
            }
        }
        let island_json = util::json_for_html(&serde_json::to_value(&islands)?);
        let ctx = Ctx {
            s: &s,
            cfg: &cfg,
            lang,
            release: opts.release,
            build: &build,
            year: today.0,
            css: &page_css,
            main_js: &js.main,
            font_preloads: fonts.preloads_for(lang).collect(),
            island_json: &island_json,
            assets_json: &assets_json,
            sprite: &sprite,
            logo: &logo,
            mark: &mark,
            osm_date: &osm_date,
        };

        let mut page = |rel_path: &str, title: String, body: maud::Markup, is_home: bool, index: bool| -> Result<String> {
            let path = format!("/{lang}{rel_path}");
            let html = document(&ctx, Page { path: &path, title, description: s.t("meta-description"), is_home, body, index }).into_string();
            for c in fonts.uncovered(&html) {
                gaps.entry(c).or_default().insert(path.clone());
            }
            Ok(html)
        };

        let html = page("/", s.t("meta-title"), home::body(&ctx), true, true)?;
        dist.add(&format!("/{lang}/"), &format!("{lang}/index.html"), html.as_bytes(), HTML, CachePolicy::Document, RouteKind::Page, Some(lang))?;

        let html = page("/privacy/", format!("{} — Mintaka", s.t("pv-title")), misc::privacy(&ctx), false, true)?;
        dist.add(&format!("/{lang}/privacy/"), &format!("{lang}/privacy/index.html"), html.as_bytes(), HTML, CachePolicy::Document, RouteKind::Page, Some(lang))?;

        let html = page("/sent/", format!("{} — Mintaka", s.t("sent-title")), misc::sent(&ctx), false, false)?;
        dist.add(&format!("/{lang}/sent/"), &format!("{lang}/sent/index.html"), html.as_bytes(), HTML, CachePolicy::Document, RouteKind::Page, Some(lang))?;

        // The 404 page is served with status 404 for unknown paths; it is not a route itself.
        let html = page("/404", format!("{} — Mintaka", s.t("nf-title")), misc::not_found(&ctx), false, false)?;
        if !html.contains(misc::SUGGEST_MARKER) {
            bail!("404 page lost its suggestion marker");
        }
        // Written uncompressed: the server fills the suggestion marker per request.
        dist.write(&format!("{lang}/404.html"), html.as_bytes())?;
        not_found.insert(lang.clone(), format!("{lang}/404.html"));

        let tp = text::pages(root, &s, &host, &cfg.contact.email, &cfg.contact.telegram)?;
        let mut files = BTreeMap::new();
        for (key, body) in [("/", &tp.index), ("/pillars", &tp.pillars), ("/contact", &tp.contact)] {
            let name = if key == "/" { "index" } else { key.trim_start_matches('/') };
            let rel = format!("text/{lang}/{name}.ansi");
            dist.write(&rel, body.as_bytes())?;
            files.insert(key.to_owned(), rel);
        }
        text_map.insert(lang.clone(), files);

        s.finish()?;
    }

    // Well-known files.
    let short = |d: &mut Dist, url: &str, body: String, ct: &str| d.add(url, url.trim_start_matches('/'), body.as_bytes(), ct, CachePolicy::Short, RouteKind::File, None);
    short(&mut dist, "/robots.txt", well_known::robots(&cfg), content_type_for("txt"))?;
    short(&mut dist, "/humans.txt", well_known::humans(&cfg, &date), content_type_for("txt"))?;
    short(&mut dist, "/sitemap.xml", well_known::sitemap(&cfg, &["/", "/privacy/"], &date), content_type_for("xml"))?;
    short(&mut dist, "/.well-known/security.txt", well_known::security_txt(&cfg, days), content_type_for("txt"))?;
    short(&mut dist, "/manifest.webmanifest", well_known::webmanifest(&cfg), content_type_for("webmanifest"))?;

    dist.compress_all()?;

    // Budgets (spec §10.1).
    let mut budget = Budget::default();
    let route_size = |dist: &Dist, path: &str| dist.routes.iter().find(|r| r.path == path).map(|r| dist.wire_size(r)).unwrap_or(0);
    let mut first_view = 0;
    for lang in &cfg.langs {
        // The 14 kB rule is about the first screen arriving in the first round trip (spec §10.1):
        // measure the compressed bytes up to the end of the hero, and cap the whole page separately.
        let html = std::fs::read_to_string(dist.dir.join(format!("{lang}/index.html")))?;
        let cut = html.find(r#"<section id="physics""#).unwrap_or(html.len());
        budget.add(format!("/{lang}/ first screen: head, CSS, header, hero (brotli)"), assets::brotli(&html.as_bytes()[..cut])?.len() as u64, Some(14_000));
        let home = route_size(&dist, &format!("/{lang}/"));
        budget.add(format!("/{lang}/ whole page (brotli)"), home, Some(32_000));
        first_view += home;
        for url in fonts.preloads_for(lang) {
            first_view += route_size(&dist, url);
        }
    }
    budget.add("inline page CSS (minified)", page_css.len() as u64, None);
    let js_limit = |name: &str| -> Option<u64> {
        match name {
            "main" => Some(4_000),
            // The hero is a small core plus exactly one renderer chunk: together ≤ 25 kB.
            "hero" => Some(6_000),
            "webgpu" | "webgl" => Some(19_000),
            "hud" => Some(6_000),
            "terminal" => Some(8_000),
            "pow" => Some(4_000),
            "physics" | "armour" | "trace" | "live" | "laws" | "contact" | "routing" | "shield" | "loadsim" => Some(15_000),
            _ => None,
        }
    };
    for (file, bytes) in &js.files {
        let name = file.split('.').next().unwrap_or(file);
        budget.add(format!("{file} (minified)"), *bytes, js_limit(name));
        // First view: boot, hero core and its renderer (WebGL2, the larger one), HUD, nav, console.
        if ["main", "hero", "webgl", "hud", "nav", "console"].contains(&name) {
            first_view += route_size(&dist, &format!("/assets/{file}"));
        }
    }
    for (name, bytes) in &wasm_sizes {
        let (label, limit) = if name.ends_with(".bin") { (format!("{name} (lazy data, raw)"), 3_000_000) } else { (format!("{name}.wasm (lazy)"), 250_000) };
        budget.add(label, *bytes, Some(limit));
    }
    first_view += route_size(&dist, &islands_css_url);
    budget.add("islands CSS (minified)", islands_css.len() as u64, None);
    budget.add("first full view, compressed (HTML, fonts, boot, hero, HUD)", first_view, Some(100_000));

    // Report and enforce.
    if !gaps.is_empty() {
        let list: Vec<String> = gaps.iter().map(|(c, w)| format!("{c:?} U+{:04X} in {}", *c as u32, w.iter().cloned().collect::<Vec<_>>().join(", "))).collect();
        let msg = format!("characters no shipped font covers (they fall back to system fonts):\n    {}", list.join("\n    "));
        if opts.release {
            bail!(msg);
        }
        warnings.push(msg);
    }
    let unverified: Vec<&str> = cfg.claims.iter().filter(|c| !c.verified).map(|c| c.key.as_str()).collect();
    if !unverified.is_empty() {
        warnings.push(format!(
            "{} unverified claims {} (mark them verified in content/site.toml only when true): {}",
            unverified.len(),
            if opts.release { "were left out of this release build" } else { "are shown with an UNVERIFIED badge" },
            unverified.join(", ")
        ));
    }
    if !cfg.launch_ready {
        warnings.push("content/site.toml: launch_ready = false — the domain and contact details are placeholders".into());
    }
    let over = budget.over();
    if opts.release && !over.is_empty() {
        bail!("over budget:\n{}", budget.table());
    }

    let routes = dist.routes.len();
    let manifest = Manifest {
        version: MANIFEST_VERSION,
        build: build.clone(),
        origin: cfg.origin.clone(),
        default_lang: cfg.default_lang.clone(),
        langs: cfg.langs.clone(),
        csp: Csp { style_src: vec![style_hash], script_src: vec![] },
        routes: std::mem::take(&mut dist.routes),
        not_found,
        text: text_map,
    };
    dist.write("manifest.json", serde_json::to_string_pretty(&manifest)?.as_bytes())?;

    Ok(Report { build, budget, warnings, routes })
}
