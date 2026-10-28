//! Import esbuild's output (built by `cargo xtask`): copy the hashed files into `dist/assets/`
//! and find the entry points in the metafile.

use crate::assets::{Dist, content_type_for};
use anyhow::{Context, Result};
use proof::{CachePolicy, RouteKind};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::Path;

#[derive(Deserialize)]
struct Meta {
    outputs: BTreeMap<String, Output>,
}

#[derive(Deserialize)]
struct Output {
    #[serde(rename = "entryPoint")]
    entry_point: Option<String>,
    bytes: u64,
}

pub struct Js {
    /// URL of the boot script (`web/main.ts`).
    pub main: String,
    /// Named entries other than main (workers): "pow.worker" → URL.
    pub named: BTreeMap<String, String>,
    /// Every JS file: (file name, minified bytes) — for the budget report.
    pub files: Vec<(String, u64)>,
}

pub fn import(js_dir: &Path, dist: &mut Dist) -> Result<Js> {
    let meta_path = js_dir.join("meta.json");
    let meta: Meta = serde_json::from_str(&std::fs::read_to_string(&meta_path).with_context(|| {
        format!("reading {} — run `cargo xtask build`, which runs esbuild first", meta_path.display())
    })?)?;
    let mut main = None;
    let mut named = BTreeMap::new();
    let mut files = vec![];
    for (out_path, out) in &meta.outputs {
        let file = Path::new(out_path).file_name().context("output without a file name")?.to_string_lossy().into_owned();
        let bytes = std::fs::read(js_dir.join(&file)).with_context(|| format!("reading esbuild output {file}"))?;
        let ext = file.rsplit('.').next().unwrap_or("");
        let url = format!("/assets/{file}");
        dist.add(&url, &format!("assets/{file}"), &bytes, content_type_for(ext), CachePolicy::Immutable, RouteKind::Asset, None)?;
        if ext == "js" {
            files.push((file.clone(), out.bytes));
        }
        if let Some(entry) = &out.entry_point {
            if entry.ends_with("web/main.ts") {
                main = Some(url.clone());
            } else if ext == "js" {
                // "web/instruments/pow.worker.ts" → "pow.worker"
                let stem = Path::new(entry).file_name().map(|n| n.to_string_lossy().trim_end_matches(".ts").to_owned()).unwrap_or_default();
                named.insert(stem, url.clone());
            }
        }
    }
    files.sort_by_key(|f| std::cmp::Reverse(f.1));
    Ok(Js { main: main.context("esbuild metafile has no web/main.ts entry")?, named, files })
}
