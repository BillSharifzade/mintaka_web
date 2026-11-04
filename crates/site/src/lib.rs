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
pub mod lab;
pub mod orion;
pub mod pages;
pub mod sw;
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

/// The URL prefix this build is for (`content/site.toml` `base`, or `MINTAKA_BASE`): the scripts
/// are bundled with it (xtask passes it to esbuild as `MINTAKA_BASE`).
pub fn base(root: &std::path::Path) -> Result<String> {
    Ok(config::SiteConfig::load(root)?.base)
}

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
    let mut dist = Dist::create(&opts.out, &cfg.base)?;
    let mut warnings = vec![];
    // Every enabled translation has the source's files, messages and attributes (CONTRACTS §10.1).
    for lang in cfg.langs.iter().filter(|l| **l != cfg.default_lang) {
        warnings.extend(i18n::check_translation(root, &cfg.default_lang, lang)?);
    }
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
    // Lab pages inline the long-form stylesheet instead (no hero, instruments or contact form).
    let lab_css = format!("{}{}", css::bundle(&root.join("web/styles/lab.css"))?, css::minify(&fonts.css)?);
    let lab_style_hash = util::csp_hash(lab_css.as_bytes());
    // The studio landing page inlines its own stylesheet (none of the lab's hero, instruments or
    // islands), allowed by its own CSP hash.
    let landing_css = format!("{}{}", css::bundle(&root.join("web/styles/landing.css"))?, css::minify(&fonts.css)?);
    let landing_style_hash = util::csp_hash(landing_css.as_bytes());

    // Lab posts (spec §S11), rendered first: they are part of the build fingerprint.
    let mut lab_posts = BTreeMap::new();
    for lang in &cfg.langs {
        let s = Strings::load(root, lang)?;
        let posts = lab::load(root, lang, &cfg, &s)?;
        s.finish()?;
        lab_posts.insert(lang.clone(), posts);
    }

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

    // Team portraits (S12): AVIF with a WebP fallback, prepared by hand (see content/site.toml).
    let mut portraits = BTreeMap::new();
    for m in &cfg.team {
        let mut url = |ext: &str| -> Result<String> {
            let path = root.join("assets/team").join(format!("{}.{ext}", m.id));
            let bytes = std::fs::read(&path).with_context(|| format!("reading portrait {}", path.display()))?;
            dist.add_hashed(&format!("team/{}", m.id), ext, &bytes, content_type_for(ext))
        };
        let portrait = pages::Portrait { avif: url("avif")?, webp: url("webp")? };
        portraits.insert(m.id.clone(), portrait);
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
    fingerprint.extend(lab_css.as_bytes());
    fingerprint.extend(landing_css.as_bytes());
    for post in lab_posts.values().flatten() {
        fingerprint.extend(post.html.as_bytes());
    }
    // The studio landing page's one script (web/studio/, first-party): its URL is in the page.
    let studio_js = js.named.get("studio").cloned().context("esbuild output has no web/studio/studio.ts entry")?;
    fingerprint.extend(studio_js.as_bytes());
    let build = format!("{date}.{}", util::short_hash(&fingerprint).to_lowercase());
    // Its compressed size, as the footer states it ("this page runs N kB of JavaScript").
    let studio_brotli = {
        let file = studio_js.rsplit('/').next().unwrap_or_default();
        let bytes = std::fs::read(opts.js_dir.join(file)).with_context(|| format!("reading {file}"))?;
        assets::brotli(&bytes)?.len() as u64
    };

    let (sprite, logo_vb) = svg::brand_symbol(root, "mintaka-logo-horizontal-on-dark.svg", "logo")?;
    let logo = svg::use_symbol("logo", &logo_vb, "logo");
    let mark = svg::brand(root, "mintaka-mark-full-on-dark.svg", "mark", None)?;
    // The landing page inlines the brand files with their colours as classes: its logo changes
    // colourway with the chapter beneath it, and its script draws the mark's own shape.
    let landing_logo = svg::brand_classed(root, "mintaka-logo-horizontal-on-dark.svg", "logo")?;
    let landing_mark = svg::brand_classed(root, "mintaka-mark-full-on-dark.svg", "mark")?;
    let landing_wordmark = svg::brand_classed(root, "mintaka-wordmark-on-dark.svg", "wordmark")?;

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
    // Lazy string bundles (content/{lang}/islands/lazy/): one JSON file per bundle and language,
    // named `strings/<name>.<lang>` for `loadStrings()` in web/shared/i18n.ts.
    let mut gaps: BTreeMap<char, BTreeSet<String>> = BTreeMap::new();
    for lang in &cfg.langs {
        let inline = i18n::island_strings(root, lang)?;
        for (name, strings) in i18n::lazy_island_strings(root, lang)? {
            for (key, value) in &strings {
                if inline.contains_key(key) {
                    bail!("content/{lang}/islands/lazy/{name}.ftl: {key} is also an inline island message");
                }
                for c in fonts.uncovered_text(value) {
                    gaps.entry(c).or_default().insert(format!("{lang}:{key}"));
                }
            }
            let stem = format!("strings/{name}.{lang}");
            let url = dist.add_hashed(&stem, "json", serde_json::to_string(&strings)?.as_bytes(), content_type_for("json"))?;
            named.insert(stem, url);
        }
    }
    // Each page lists only its own language's string bundles: loadStrings() asks for no other.
    let assets_for = |lang: &str| -> Result<String> {
        let own = |k: &String| k.strip_prefix("strings/").is_none_or(|b| b.ends_with(&format!(".{lang}")));
        let list: BTreeMap<&String, &String> = named.iter().filter(|(k, _)| own(k)).collect();
        Ok(util::json_for_html(&serde_json::to_value(&list)?))
    };

    let osm_date = std::fs::read_to_string(root.join("assets/data/dushanbe.meta.json"))
        .ok()
        .and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok())
        .and_then(|m| m["osm_timestamp"].as_str().map(|s| s.chars().take(10).collect::<String>()))
        .unwrap_or_else(|| "—".into());
    let mut text_map = BTreeMap::new();
    let mut bot = BTreeMap::new();
    let mut not_found = BTreeMap::new();
    let host = format!("{}{}", cfg.origin.trim_start_matches("https://").trim_start_matches("http://"), cfg.base);
    let mut lab_pages: Vec<(String, String)> = vec![];
    let mut landing_pages: Vec<(String, String)> = vec![];

    let mut people = vec![];
    for lang in &cfg.langs {
        let s = Strings::load(root, lang)?;
        if *lang == cfg.default_lang {
            for m in &cfg.team {
                let t = |field: &str| s.t(&format!("team-{}-{field}", m.id));
                people.push((t("role"), t("name"), m.github.clone(), m.site.clone()));
            }
        }
        let islands = i18n::island_strings(root, lang)?;
        for (key, value) in &islands {
            for c in fonts.uncovered_text(value) {
                gaps.entry(c).or_default().insert(format!("{lang}:{key}"));
            }
        }
        let island_json = util::json_for_html(&serde_json::to_value(&islands)?);
        let assets_json = assets_for(lang)?;
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
            portraits: &portraits,
            lab: lab_posts.get(lang).map_or(&[][..], Vec::as_slice),
            lab_fallback: lab_posts
                .get(lang)
                .is_none_or(Vec::is_empty)
                .then(|| lab_posts.get(&cfg.default_lang).filter(|p| !p.is_empty()).map(|p| (cfg.default_lang.as_str(), p.as_slice())))
                .flatten(),
        };

        let mut page = |rel_path: &str, title: String, body: maud::Markup, is_home: bool, index: bool| -> Result<String> {
            let path = format!("/{lang}{rel_path}");
            let html = document(&ctx, Page { path: &path, title, description: s.t("meta-description"), is_home, body, index, long_form: None }).into_string();
            for c in fonts.uncovered(&html) {
                gaps.entry(c).or_default().insert(path.clone());
            }
            Ok(html)
        };

        // The home page, once per variant while an A/B test runs (spec §12): the control as the
        // route's own file, the others beside it. The server picks one for every page load.
        let (home_path, home_file) = (format!("/{lang}/"), format!("{lang}/index.html"));
        match &cfg.experiment {
            None => {
                let html = page("/", s.t("meta-title"), home::body(&ctx), true, true)?;
                dist.add(&home_path, &home_file, html.as_bytes(), HTML, CachePolicy::Document, RouteKind::Page, Some(lang))?;
            }
            Some(exp) => {
                for (i, variant) in exp.ids().enumerate() {
                    let html = s.as_variant(&exp.control, variant, || page("/", s.t("meta-title"), home::body(&ctx), true, true))?;
                    if i == 0 {
                        dist.add(&home_path, &home_file, html.as_bytes(), HTML, CachePolicy::Document, RouteKind::Page, Some(lang))?;
                    } else {
                        dist.add_variant(&home_path, variant, &format!("{lang}/index.{variant}.html"), html.as_bytes())?;
                    }
                }
            }
        }

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

        // The studio landing page (/{lang}/studio/): the company's front door, linking to the lab.
        // Its own document, stylesheet and one first-party script, so none of the lab's chrome
        // comes with it. Rendered after the `page` closure's last use above, which holds `gaps`.
        let kb = format!("{:.1}", studio_brotli as f64 / 1000.0);
        let landing = pages::landing::Landing {
            cfg: &cfg,
            s: &s,
            lang,
            build: &build,
            year: today.0,
            logo: &landing_logo,
            mark: &landing_mark,
            wordmark: &landing_wordmark,
            portraits: &portraits,
            font_preloads: fonts.preloads_for(lang).collect(),
            css: &landing_css,
            script: &studio_js,
            script_kb: if lang == "en" { kb } else { kb.replace('.', ",") },
        };
        let landing_html = pages::landing::page(&landing).into_string();
        for c in fonts.uncovered(&landing_html) {
            gaps.entry(c).or_default().insert(format!("/{lang}/studio/"));
        }
        dist.add(&format!("/{lang}/studio/"), &format!("{lang}/studio/index.html"), landing_html.as_bytes(), HTML, CachePolicy::Document, RouteKind::Page, Some(lang))?;
        landing_pages.push((format!("/{lang}/studio/"), landing_html));

        // The Lab (spec §S11): long-form pages with no JavaScript, and the Atom feed. Their
        // glyphs and the no-script rule are checked after the loop.
        for (path, html) in pages::lab::pages(&ctx, &lab_posts, &lab_css) {
            let file = format!("{}index.html", path.trim_start_matches('/'));
            dist.add(&path, &file, html.as_bytes(), HTML, CachePolicy::Document, RouteKind::Page, Some(lang))?;
            lab_pages.push((path, html));
        }
        if !ctx.lab.is_empty() {
            let feed = lab::atom(&cfg, lang, &s, ctx.lab);
            dist.add(&lab::feed_path(lang), &format!("{lang}/lab/feed.xml"), feed.as_bytes(), content_type_for("atom"), CachePolicy::Short, RouteKind::File, Some(lang))?;
        }

        let lang_path = if *lang == cfg.default_lang { String::new() } else { format!("/{lang}") };
        let tp = text::pages(root, &s, &host, &lang_path, &cfg.contact)?;
        let mut files = BTreeMap::new();
        for (key, body) in [("/", &tp.index), ("/pillars", &tp.pillars), ("/contact", &tp.contact)] {
            let name = if key == "/" { "index" } else { key.trim_start_matches('/') };
            let rel = format!("text/{lang}/{name}.ansi");
            dist.write(&rel, body.as_bytes())?;
            files.insert(key.to_owned(), rel);
        }
        text_map.insert(lang.clone(), files);

        // What the Telegram bot writes to clients whose brief is in this language
        // (docs/CONTRACTS.md §19): rendered now, with `{brief}` left for the server.
        let args = [
            ("within", cfg.contact.reply_within.as_str().into()),
            ("site", cfg.abs(&format!("/{lang}/")).into()),
            ("brief", "{brief}".into()),
        ];
        bot.insert(lang.clone(), proof::BOT_MESSAGES.iter().map(|id| (id.to_string(), s.ta(id, &args))).collect::<BTreeMap<_, _>>());

        s.finish()?;
    }

    // Lab pages: every glyph covered, and no script of any kind (spec §9.5).
    for (path, html) in &lab_pages {
        for c in fonts.uncovered(html) {
            gaps.entry(c).or_default().insert(path.clone());
        }
        if html.contains("<script") {
            bail!("{path} contains a <script> element: Lab pages ship no JavaScript (spec §9.5)");
        }
    }
    let lab_urls = lab::sitemap_urls(&lab_posts);

    // Well-known files.
    let short = |d: &mut Dist, url: &str, body: String, ct: &str| d.add(url, url.trim_start_matches('/'), body.as_bytes(), ct, CachePolicy::Short, RouteKind::File, None);
    short(&mut dist, "/robots.txt", well_known::robots(&cfg), content_type_for("txt"))?;
    short(&mut dist, "/humans.txt", well_known::humans(&cfg, &date, &people), content_type_for("txt"))?;
    short(&mut dist, "/sitemap.xml", well_known::sitemap(&cfg, &["/", "/studio/", "/privacy/"], &lab_urls, &date), content_type_for("xml"))?;
    short(&mut dist, "/.well-known/security.txt", well_known::security_txt(&cfg, days), content_type_for("txt"))?;
    short(&mut dist, "/manifest.webmanifest", well_known::webmanifest(&cfg), content_type_for("webmanifest"))?;

    // Offline mode: the service worker, with the file list of everything added above.
    let preloads = cfg.langs.iter().map(|l| (l.clone(), fonts.preloads_for(l).map(str::to_owned).collect())).collect();
    let sw_inputs = sw::Inputs { build: &build, js_dir: &opts.js_dir, islands_css: &islands_css_url, fonts: preloads };
    let sw_bytes = sw::add(&mut dist, &sw_inputs)?;
    if sw_bytes.is_none() {
        let msg = "no service worker in esbuild's output (sw.js): this build has no offline mode";
        if opts.release {
            bail!(msg);
        }
        warnings.push(msg.into());
    }

    dist.compress_all()?;

    // Budgets (spec §10.1).
    let mut budget = Budget::default();
    // By route path or by link (hashed asset URLs carry the URL prefix).
    let route_size = |dist: &Dist, path: &str| {
        let path = path.strip_prefix(dist.base.as_str()).filter(|p| p.starts_with('/')).unwrap_or(path);
        dist.routes.iter().find(|r| r.path == path).map(|r| dist.wire_size(r)).unwrap_or(0)
    };
    // A visitor's first view is one language's home page and fonts plus the shared files below.
    let mut first_view_page = BTreeMap::new();
    for lang in &cfg.langs {
        // The 14 kB rule is about the first screen arriving in the first round trip (spec §10.1):
        // measure the compressed bytes up to the end of the hero, and cap the whole page separately.
        // Every variant of an A/B test is a page a visitor may get, so each has to fit; the first
        // view counts the largest.
        let route = dist.routes.iter().find(|r| r.path == format!("/{lang}/")).context("no home page route")?;
        let mut files = vec![(String::new(), route.file.clone(), route.encodings.clone())];
        files.extend(route.variants.iter().map(|v| (format!(", variant {}", v.id), v.file.clone(), v.encodings.clone())));
        let mut home = 0;
        for (variant, file, encodings) in files {
            let html = std::fs::read_to_string(dist.dir.join(&file))?;
            let cut = html.find(r#"<section id="physics""#).unwrap_or(html.len());
            let first_screen = assets::brotli(&html.as_bytes()[..cut])?.len() as u64;
            budget.add(format!("/{lang}/ first screen{variant}: head, CSS, header, hero (brotli)"), first_screen, Some(14_000));
            let whole = dist.file_wire_size(&file, &encodings);
            budget.add(format!("/{lang}/ whole page{variant} (brotli)"), whole, Some(32_000));
            home = home.max(whole);
        }
        let font_bytes: u64 = fonts.preloads_for(lang).map(|url| route_size(&dist, url)).sum();
        first_view_page.insert(lang.clone(), home + font_bytes);
    }
    let mut first_view = 0;
    budget.add("inline page CSS (minified)", page_css.len() as u64, None);
    // Lab pages: the same 14 kB first-round-trip rule for what a reader sees first (head, CSS,
    // header, the post's title and summary), and the home page's 32 kB cap for a whole page. The
    // feed is read by feed readers, never on a first visit, so it only gets a ceiling.
    for (path, html) in &lab_pages {
        let cut = html.find(r#"<div class="post__body"#).or_else(|| html.find(r#"<ol class="lab-list"#)).unwrap_or(html.len());
        budget.add(format!("{path} first screen (brotli)"), assets::brotli(&html.as_bytes()[..cut])?.len() as u64, Some(14_000));
        budget.add(format!("{path} whole page (brotli)"), route_size(&dist, path), Some(32_000));
    }
    if !lab_pages.is_empty() {
        budget.add("inline Lab CSS (minified)", lab_css.len() as u64, None);
    }
    // The studio landing page: the same first-round-trip rule (header + hero) and the whole-page
    // cap. The first screen is complete without its script (which only adds motion and
    // measurements), so the script is capped on its own, and together with the page.
    for (path, html) in &landing_pages {
        let cut = html.find(r#"<section id="measured""#).context("studio landing page lost its first-screen marker")?;
        budget.add(format!("{path} first screen (brotli)"), assets::brotli(&html.as_bytes()[..cut])?.len() as u64, Some(14_000));
        let whole = route_size(&dist, path);
        budget.add(format!("{path} whole page (brotli)"), whole, Some(32_000));
        budget.add(format!("{path} page + its script (brotli)"), whole + route_size(&dist, &studio_js), Some(40_000));
    }
    if !landing_pages.is_empty() {
        budget.add("inline landing CSS (minified)", landing_css.len() as u64, None);
    }
    for lang in lab_posts.iter().filter(|(_, posts)| !posts.is_empty()).map(|(lang, _)| lang) {
        budget.add(format!("{} (brotli)", lab::feed_path(lang)), route_size(&dist, &lab::feed_path(lang)), Some(64_000));
    }
    let js_limit = |name: &str| -> Option<u64> {
        match name {
            "main" => Some(4_000),
            // The hero is a small core plus exactly one renderer chunk: together ≤ 25 kB.
            "hero" => Some(6_000),
            "webgpu" | "webgl" => Some(19_000),
            // Raised from 6 kB on 2026-10-30: the panel's rows and their text arrive on first opening
            // (a lazy string bundle), ~0.3 kB of code here for 1.4–2.0 kB brotli less HTML on every
            // page view.
            "hud" => Some(6_200),
            "terminal" => Some(8_000),
            "pow" => Some(4_000),
            // The console challenge: client and application form, fetched only on a challenge
            // command (its text is a lazy string bundle, not counted here).
            "ctf" => Some(7_000),
            // Real-user measurements (loaded on idle), offline mode (on idle), the console's visitors
            // row and, while an A/B test runs, its lines in that row. rum was 3,000 until the A/B
            // test's exposure and conversion fields and the URL prefix took it to ~3,020 (2026-10-30).
            "rum" => Some(3_200),
            "offline" | "visitors" | "experiment" => Some(2_000),
            "physics" | "armour" | "trace" | "live" | "laws" | "contact" | "routing" | "loadsim" => Some(15_000),
            // Three chapters, the coverage sky, the crash minimiser and a hand-input panel. Raised
            // from 21 kB on 2026-10-30 for the code that fetches its run-time text (a lazy string
            // bundle) on the first start and builds the hand-input panel on first opening: about
            // 0.45 kB here for 1.6 kB brotli less HTML on every Russian page view.
            "fuzzer" => Some(21_500),
            // The studio landing page's motion, star chart and self-measurement (web/studio/).
            "studio" => Some(16_000),
            _ => None,
        }
    };
    for (file, bytes) in &js.files {
        let name = file.split('.').next().unwrap_or(file);
        budget.add(format!("{file} (minified)"), *bytes, js_limit(name));
    }
    // First view: boot, hero core and its renderer (WebGL2, the larger one), HUD, nav, console, the
    // two idle-time modules every view loads (real-user measurements, offline mode), and the shared
    // chunks they import.
    let first_js = js.files.iter().map(|(f, _)| f.as_str()).filter(|f| {
        ["main", "hero", "webgl", "hud", "nav", "console", "rum", "offline"].contains(&f.split('.').next().unwrap_or(f))
    });
    for file in js.with_imports(first_js) {
        first_view += route_size(&dist, &format!("/assets/{file}"));
    }
    for (name, bytes) in &wasm_sizes {
        let (label, limit) = if name.ends_with(".bin") { (format!("{name} (lazy data, raw)"), 3_000_000) } else { (format!("{name}.wasm (lazy)"), 250_000) };
        budget.add(label, *bytes, Some(limit));
    }
    first_view += route_size(&dist, &islands_css_url);
    if let Some(bytes) = sw_bytes {
        // The file list grows with every page and language: raised from 8 kB on 2026-10-30, when
        // Russian and Tajik tripled the pages (24, three ETags on each home page for the A/B test)
        // and string bundles (77 assets), every URL carrying the /mintaka prefix (~1 kB of it).
        budget.add("sw.js (minified, with this build's file list)", bytes, Some(11_000));
        first_view += route_size(&dist, "/sw.js");
    }
    budget.add("islands CSS (minified)", islands_css.len() as u64, None);
    for (lang, page) in &first_view_page {
        budget.add(format!("/{lang}/ first full view, compressed (HTML, fonts, boot, hero, HUD, RUM, offline)"), first_view + page, Some(100_000));
    }

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

    // Every inline stylesheet of every page — each A/B test variant and each 404 page too — is
    // one the CSP allows.
    let mut style_src = vec![style_hash];
    if !lab_pages.is_empty() {
        style_src.push(lab_style_hash);
    }
    if !landing_pages.is_empty() {
        style_src.push(landing_style_hash);
    }
    let html_files = dist.routes.iter().filter(|r| r.content_type == HTML).flat_map(|r| std::iter::once(&r.file).chain(r.variants.iter().map(|v| &v.file)));
    for file in html_files.chain(not_found.values()) {
        let html = std::fs::read_to_string(dist.dir.join(file))?;
        for (css, _) in html.split("<style>").skip(1).filter_map(|rest| rest.split_once("</style>")) {
            if !style_src.contains(&util::csp_hash(css.as_bytes())) {
                bail!("{file}: an inline <style> whose hash is not in the CSP");
            }
        }
    }

    let routes = dist.routes.len();
    let manifest = Manifest {
        version: MANIFEST_VERSION,
        build: build.clone(),
        origin: cfg.origin.clone(),
        base: cfg.base.clone(),
        default_lang: cfg.default_lang.clone(),
        langs: cfg.langs.clone(),
        csp: Csp { style_src, script_src: vec![] },
        routes: std::mem::take(&mut dist.routes),
        not_found,
        text: text_map,
        experiment: cfg.experiment.as_ref().map(config::Experiment::manifest),
        bot,
    };
    dist.write("manifest.json", serde_json::to_string_pretty(&manifest)?.as_bytes())?;

    Ok(Report { build, budget, warnings, routes })
}
