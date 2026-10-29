//! `cargo xtask build [--release]` — Rust → WebAssembly (instruments), TypeScript (esbuild),
//! then the site generator → dist/.
//! `cargo xtask dev` — build, run the server, rebuild and restart on every change.

use anyhow::{Context, Result, bail};
use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::time::{Duration, SystemTime};

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().context("workspace root")?.to_owned();
    let release = args.iter().any(|a| a == "--release");
    match args.first().map(String::as_str) {
        Some("build") => build(&root, release).map(|_| ()),
        Some("dev") => dev(&root),
        Some("graph") => graph(&root, args.iter().any(|a| a == "--fetch")),
        _ => {
            eprintln!("usage: cargo xtask build [--release]\n       cargo xtask dev\n       cargo xtask graph [--fetch]");
            std::process::exit(2);
        }
    }
}

/// Car-routable roads in a box around Dushanbe (S4 routing race). Overpass QL.
const OVERPASS_ROADS: &str = r#"[out:json][timeout:180];way["highway"~"^(motorway|trunk|primary|secondary|tertiary|unclassified|residential|living_street|motorway_link|trunk_link|primary_link|secondary_link|tertiary_link)$"](38.49,68.69,38.63,68.87);(._;>;);out body qt;"#;

/// `cargo xtask graph [--fetch]`: OpenStreetMap extract → road graph + Contraction Hierarchy →
/// `assets/data/dushanbe.bin` (committed, so normal builds never touch the network).
fn graph(root: &Path, fetch: bool) -> Result<()> {
    let src = root.join("target/osm/dushanbe-roads.json");
    if fetch || !src.exists() {
        std::fs::create_dir_all(src.parent().context("path")?)?;
        println!("  fetching roads from the Overpass API…");
        let ok = Command::new("curl")
            .args(["-sf", "-m", "240", "-A", "mintaka-web build", "--data-urlencode"])
            .arg(format!("data={OVERPASS_ROADS}"))
            .arg("https://overpass-api.de/api/interpreter")
            .arg("-o")
            .arg(&src)
            .status()?
            .success();
        if !ok {
            bail!("Overpass download failed (the public server is often busy — retry, or use a mirror)");
        }
    }
    let json = std::fs::read_to_string(&src)?;
    let stamp = serde_json::from_str::<serde_json::Value>(&json)?["osm3s"]["timestamp_osm_base"].as_str().unwrap_or("").to_owned();
    let t0 = std::time::Instant::now();
    let (g, st) = routing::prep::from_overpass(&json).map_err(|e| anyhow::anyhow!(e))?;
    let bytes = g.to_bytes();
    let out = root.join("assets/data");
    std::fs::create_dir_all(&out)?;
    std::fs::write(out.join("dushanbe.bin"), &bytes)?;
    let meta = serde_json::json!({
        "source": "OpenStreetMap contributors, ODbL 1.0 (https://www.openstreetmap.org/copyright)",
        "osm_timestamp": stamp,
        "query": OVERPASS_ROADS,
        "osm_ways": st.osm_ways, "osm_nodes": st.osm_nodes,
        "nodes": st.nodes, "arcs": st.arcs, "segments": st.segments, "points": st.points, "shortcuts": st.shortcuts,
        "bytes": bytes.len(),
    });
    std::fs::write(out.join("dushanbe.meta.json"), serde_json::to_string_pretty(&meta)? + "\n")?;
    println!(
        "  dushanbe.bin: {} intersections, {} arcs, {} segments ({} points), {} shortcuts · {} bytes · OSM {} · {} ms",
        st.nodes, st.arcs, st.segments, st.points, st.shortcuts, bytes.len(), stamp, t0.elapsed().as_millis()
    );
    Ok(())
}

/// Crates marked `[package.metadata.mintaka] wasm = true`, by package name.
/// Crates marked `[package.metadata.mintaka] wasm = true`, and whether they use wasm-bindgen
/// (`bindgen = false` ships the module as compiled, for plain C-ABI exports).
fn wasm_crates(root: &Path) -> Result<Vec<(String, bool)>> {
    let mut out = vec![];
    for entry in std::fs::read_dir(root.join("crates"))? {
        let manifest = entry?.path().join("Cargo.toml");
        let Ok(text) = std::fs::read_to_string(&manifest) else { continue };
        if !text.contains("[package.metadata.mintaka]") || !text.lines().any(|l| l.trim() == "wasm = true") {
            continue;
        }
        let name = text
            .lines()
            .find_map(|l| l.trim().strip_prefix("name = \"").and_then(|r| r.strip_suffix('"')))
            .with_context(|| format!("no package name in {}", manifest.display()))?;
        let bindgen = !text.lines().any(|l| l.trim() == "bindgen = false");
        out.push((name.to_owned(), bindgen));
    }
    out.sort();
    Ok(out)
}

/// Rust → WebAssembly: one cargo build for every instrument crate, then wasm-bindgen glue into
/// `web/wasm/<name>/` (bundled by esbuild with the island that imports it) and wasm-opt. Either
/// way the module ends up as `web/wasm/<name>/<name>_bg.wasm`, which the generator ships.
fn wasm(root: &Path) -> Result<()> {
    let crates = wasm_crates(root)?;
    if crates.is_empty() {
        return Ok(());
    }
    let mut cmd = Command::new("cargo");
    cmd.current_dir(root).args(["build", "--quiet", "--target", "wasm32-unknown-unknown", "--profile", "wasm"]);
    for (c, _) in &crates {
        cmd.args(["-p", c]);
    }
    if !cmd.status().context("running cargo for wasm32")?.success() {
        bail!("WebAssembly build failed (is the target installed? `pacman -S rust-wasm`)");
    }
    for (c, bindgen) in &crates {
        let lib = c.replace('-', "_");
        let input = root.join(format!("target/wasm32-unknown-unknown/wasm/{lib}.wasm"));
        let out_dir = root.join("web/wasm").join(c);
        let bg = out_dir.join(format!("{c}_bg.wasm"));
        if *bindgen {
            let ok = Command::new("wasm-bindgen")
                .args(["--target", "web", "--out-name", c])
                .arg("--out-dir")
                .arg(&out_dir)
                .arg(&input)
                .status()
                .context("running wasm-bindgen (`pacman -S wasm-bindgen`)")?
                .success();
            if !ok {
                bail!("wasm-bindgen failed for {c}");
            }
        } else {
            std::fs::create_dir_all(&out_dir)?;
            std::fs::copy(&input, &bg).with_context(|| format!("copying {}", input.display()))?;
        }
        let ok = Command::new("wasm-opt")
            .args([
                "-O3",
                "--enable-bulk-memory",
                "--enable-nontrapping-float-to-int",
                "--enable-sign-ext",
                "--enable-mutable-globals",
                "--enable-reference-types",
                "--enable-multivalue",
            ])
            .arg(&bg)
            .arg("-o")
            .arg(&bg)
            .status()
            .context("running wasm-opt (`pacman -S binaryen`)")?
            .success();
        if !ok {
            bail!("wasm-opt failed for {c}");
        }
    }
    Ok(())
}

fn esbuild(root: &Path, release: bool) -> Result<PathBuf> {
    let out = root.join("target/web");
    if out.exists() {
        std::fs::remove_dir_all(&out)?;
    }
    let bin = root.join("node_modules/.bin/esbuild");
    if !bin.exists() {
        bail!("esbuild not found — run `npm install` in {}", root.display());
    }
    let mut entries = vec!["web/main.ts".to_owned()];
    let mut workers = vec![];
    find(&root.join("web"), &mut |p| {
        if p.to_string_lossy().ends_with(".worker.ts") {
            workers.push(p.strip_prefix(root).unwrap_or(p).to_string_lossy().into_owned());
        }
    })?;
    workers.sort();
    entries.extend(workers);
    let mut cmd = Command::new(&bin);
    cmd.current_dir(root).args(&entries).args([
        "--bundle",
        "--splitting",
        "--format=esm",
        "--target=es2022,chrome111,firefox115,safari16.4",
        "--minify",
        "--legal-comments=none",
        "--charset=utf8",
        "--entry-names=[name].[hash]",
        "--chunk-names=[name].[hash]",
        "--log-level=warning",
    ]);
    cmd.arg(format!("--outdir={}", out.display())).arg(format!("--metafile={}", out.join("meta.json").display()));
    cmd.arg(format!("--define:MINTAKA_DEV={}", !release));
    if !release {
        cmd.arg("--sourcemap=linked");
    }
    let status = cmd.status().context("running esbuild")?;
    if !status.success() {
        bail!("esbuild failed");
    }
    Ok(out)
}

fn find(dir: &Path, f: &mut dyn FnMut(&Path)) -> Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            find(&path, f)?;
        } else {
            f(&path);
        }
    }
    Ok(())
}

fn build(root: &Path, release: bool) -> Result<String> {
    let t0 = std::time::Instant::now();
    wasm(root)?;
    let js_dir = esbuild(root, release)?;
    let report = site::build(&site::BuildOptions { root: root.to_owned(), out: root.join("dist"), js_dir, release })?;
    println!("\n  mintaka · build {} · {} routes · {} ms\n", report.build, report.routes, t0.elapsed().as_millis());
    print!("{}", report.budget.table());
    for w in &report.warnings {
        println!("\n  warning: {w}");
    }
    println!();
    Ok(report.build)
}

/// Newest modification time under the watched inputs.
fn newest(root: &Path) -> SystemTime {
    let mut t = SystemTime::UNIX_EPOCH;
    for dir in ["web", "content", "assets", "crates"] {
        let _ = find(&root.join(dir), &mut |p| {
            // web/wasm is our own output; watching it would rebuild forever.
            if p.components().any(|c| c.as_os_str() == "wasm") && p.starts_with(root.join("web")) {
                return;
            }
            if let Ok(m) = std::fs::metadata(p).and_then(|m| m.modified()) {
                t = t.max(m);
            }
        });
    }
    t
}

fn server(root: &Path) -> Result<Child> {
    let status = Command::new("cargo").current_dir(root).args(["build", "--quiet", "-p", "server"]).status()?;
    if !status.success() {
        bail!("the server does not build yet");
    }
    let location = std::env::var("MINTAKA_LOCATION").unwrap_or_else(|_| "Dushanbe,TJ,38.5598,68.7870".into());
    // 8080 is often taken by other dev servers; override with MINTAKA_LISTEN.
    let listen = std::env::var("MINTAKA_LISTEN").unwrap_or_else(|_| "127.0.0.1:8761".into());
    println!("  serving http://{listen}/  (location {location})\n");
    Ok(Command::new(root.join("target/debug/mintaka-server"))
        .current_dir(root)
        .args(["--dist", "dist", "--listen", &listen, "--location", &location])
        .spawn()?)
}

fn dev(root: &Path) -> Result<()> {
    let generator_mtime = |root: &Path| {
        let mut t = SystemTime::UNIX_EPOCH;
        let _ = find(&root.join("crates/site/src"), &mut |p| {
            if let Ok(m) = std::fs::metadata(p).and_then(|m| m.modified()) {
                t = t.max(m);
            }
        });
        t
    };
    let generator_at_start = generator_mtime(root);
    let mut last = newest(root);
    if let Err(e) = build(root, false) {
        eprintln!("  build failed: {e:#}");
    }
    let mut child = server(root).map_err(|e| eprintln!("  {e:#}")).ok();
    loop {
        std::thread::sleep(Duration::from_millis(350));
        let now = newest(root);
        if now <= last {
            continue;
        }
        last = now;
        if generator_mtime(root) > generator_at_start {
            eprintln!("  the generator's source changed — stop and rerun `cargo xtask dev` to recompile it");
        }
        match build(root, false) {
            Ok(_) => {
                if let Some(mut c) = child.take() {
                    let _ = c.kill();
                    let _ = c.wait();
                }
                child = server(root).map_err(|e| eprintln!("  {e:#}")).ok();
            }
            Err(e) => eprintln!("  build failed: {e:#}"),
        }
    }
}
