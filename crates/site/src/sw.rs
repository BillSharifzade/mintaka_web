//! `/sw.js`, the service worker of offline mode (spec §7.9, docs/CONTRACTS.md §15).
//!
//! xtask bundles `web/sw/sw.ts` into `sw.js` next to esbuild's other output (not in its
//! metafile). Here the one `self.__MINTAKA_SHELL__` in it becomes this build's file list: the
//! pages with their languages and content hashes (one per A/B test variant, any of which the
//! server may send), the shell (main.js, islands.css and the opening experiment — web/city/ with
//! its streets — each with its static imports), the fonts each language preloads, and every asset
//! of the build. (`renderers`, the particle hero's per-tier chunks, is empty since 2026-11-09.) Any change to a shipped file therefore changes the
//! worker's bytes, which is what makes browsers install the new one.
//!
//! Served at a fixed URL with the `document` policy (`no-cache` + ETag): the policy is about
//! revalidating on every use, which is what a worker script needs.

use crate::assets::{Dist, content_type_for};
use crate::util::sha256;
use anyhow::{Context, Result, bail, ensure};
use proof::{CachePolicy, RouteKind};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

/// The token in the bundle that the file list replaces (exactly once).
pub const PLACEHOLDER: &str = "self.__MINTAKA_SHELL__";

/// What the worker knows about this build (`Shell` in web/sw/lib.ts).
#[derive(Debug, Serialize, PartialEq)]
pub struct Shell {
    pub build: String,
    pub pages: BTreeMap<String, Page>,
    pub core: Vec<String>,
    pub fonts: BTreeMap<String, Vec<String>>,
    pub renderers: BTreeMap<String, Vec<String>>,
    pub assets: Vec<String>,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct Page {
    pub lang: String,
    /// First 128 bits of SHA-256 of the page's bytes, hex: the server's ETag, unquoted. One per
    /// variant on a page of the A/B test, control first; one everywhere else.
    pub etags: Vec<String>,
}

/// What the generator already decided, by URL.
pub struct Inputs<'a> {
    pub build: &'a str,
    pub js_dir: &'a Path,
    pub islands_css: &'a str,
    /// Language → the font URLs its pages preload.
    pub fonts: BTreeMap<String, Vec<String>>,
}

#[derive(Deserialize)]
struct Meta {
    outputs: BTreeMap<String, Output>,
}

#[derive(Deserialize)]
struct Output {
    #[serde(rename = "entryPoint")]
    entry_point: Option<String>,
    #[serde(default)]
    imports: Vec<Import>,
}

#[derive(Deserialize)]
struct Import {
    path: String,
    kind: String,
}

fn file_name(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
}

/// esbuild's chunk graph: entry source → file, and each file's static imports.
struct Chunks {
    by_entry: BTreeMap<String, String>,
    imports: BTreeMap<String, Vec<String>>,
}

impl Chunks {
    fn read(js_dir: &Path) -> Result<Chunks> {
        Chunks::parse(&std::fs::read(js_dir.join("meta.json")).context("reading esbuild's meta.json")?)
    }

    fn parse(metafile: &[u8]) -> Result<Chunks> {
        let meta: Meta = serde_json::from_slice(metafile)?;
        let mut by_entry = BTreeMap::new();
        let mut imports = BTreeMap::new();
        for (out, o) in meta.outputs {
            let file = file_name(&out).to_owned();
            if let Some(entry) = o.entry_point {
                by_entry.insert(entry, file.clone());
            }
            let statics = o.imports.iter().filter(|i| i.kind == "import-statement").map(|i| file_name(&i.path).to_owned()).collect();
            imports.insert(file, statics);
        }
        Ok(Chunks { by_entry, imports })
    }

    /// `/assets/` URLs of an entry's chunk and everything it imports statically, entry first.
    fn closure(&self, entry: &str) -> Result<Vec<String>> {
        let first = self.by_entry.get(entry).with_context(|| format!("esbuild output has no entry {entry}"))?;
        let mut seen = BTreeSet::new();
        let mut order = vec![];
        let mut todo = vec![first.clone()];
        while let Some(file) = todo.pop() {
            if seen.insert(file.clone()) {
                todo.extend(self.imports.get(&file).into_iter().flatten().cloned());
                order.push(format!("/assets/{file}"));
            }
        }
        Ok(order)
    }
}

/// The file list for this build, from the routes already in `dist` and esbuild's chunk graph.
pub fn shell(dist: &Dist, inputs: &Inputs) -> Result<Shell> {
    let chunks = Chunks::read(inputs.js_dir)?;
    let mut pages = BTreeMap::new();
    // The worker sees URLs, so every path in the list carries the site's URL prefix.
    let link = |paths: Vec<String>| paths.iter().map(|p| dist.link(p)).collect::<Vec<_>>();
    for route in dist.routes.iter().filter(|r| r.kind == RouteKind::Page) {
        let mut etags = vec![];
        for file in std::iter::once(&route.file).chain(route.variants.iter().map(|v| &v.file)) {
            let bytes = std::fs::read(dist.dir.join(file))?;
            etags.push(sha256(&bytes)[..16].iter().map(|b| format!("{b:02x}")).collect());
        }
        pages.insert(dist.link(&route.path), Page { lang: route.lang.clone().unwrap_or_default(), etags });
    }
    let mut core = link(chunks.closure("web/main.ts")?);
    core.push(inputs.islands_css.to_owned());
    core.extend(link(chunks.closure("web/city/index.ts")?));
    if let Some(r) = dist.routes.iter().find(|r| r.path.starts_with("/assets/data/city.") && r.path.ends_with(".bin")) {
        core.push(dist.link(&r.path));
    }
    let mut seen = BTreeSet::new();
    core.retain(|u| seen.insert(u.clone()));
    let renderers = BTreeMap::new();
    let assets = dist.routes.iter().filter(|r| r.path.starts_with("/assets/") && !r.path.ends_with(".map")).map(|r| dist.link(&r.path)).collect();
    Ok(Shell { build: inputs.build.to_owned(), pages, core, fonts: inputs.fonts.clone(), renderers, assets })
}

/// Writes `/sw.js` into `dist`. Returns its size, or `None` when esbuild's output has no worker
/// (then the page's registration finds a 404 and there is simply no offline mode).
pub fn add(dist: &mut Dist, inputs: &Inputs) -> Result<Option<u64>> {
    let Ok(bundle) = std::fs::read_to_string(inputs.js_dir.join("sw.js")) else { return Ok(None) };
    let shell = shell(dist, inputs)?;
    for url in shell.core.iter().chain(shell.fonts.values().flatten()).chain(shell.renderers.values().flatten()) {
        ensure!(shell.assets.contains(url), "service worker shell names {url}, which is not a built asset");
    }
    let body = fill(&bundle, &serde_json::to_string(&shell)?)?;
    dist.add("/sw.js", "sw.js", body.as_bytes(), content_type_for("js"), CachePolicy::Document, RouteKind::File, None)?;
    Ok(Some(body.len() as u64))
}

fn fill(bundle: &str, json: &str) -> Result<String> {
    match bundle.matches(PLACEHOLDER).count() {
        1 => Ok(bundle.replacen(PLACEHOLDER, json, 1)),
        n => bail!("the service worker bundle should contain {PLACEHOLDER} once, found it {n} times"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_placeholder_is_filled_once() {
        let out = fill(r#"(()=>{var e=self.__MINTAKA_SHELL__;console.log(e.build)})();"#, r#"{"build":"b1"}"#).unwrap();
        assert_eq!(out, r#"(()=>{var e={"build":"b1"};console.log(e.build)})();"#);
        assert!(fill("var e=1;", "{}").is_err());
        assert!(fill("self.__MINTAKA_SHELL__;self.__MINTAKA_SHELL__", "{}").is_err());
    }

    #[test]
    fn closures_follow_static_imports_only() {
        let chunks = Chunks::parse(
            br#"{"outputs":{
              "target/web/main.A.js":{"entryPoint":"web/main.ts","imports":[{"path":"target/web/chunk.D.js","kind":"import-statement"},{"path":"target/web/city.H.js","kind":"dynamic-import"}]},
              "target/web/city.H.js":{"entryPoint":"web/city/index.ts","imports":[{"path":"target/web/chunk.D.js","kind":"import-statement"},{"path":"target/web/chunk.G.js","kind":"import-statement"}]},
              "target/web/chunk.D.js":{"imports":[]},
              "target/web/chunk.G.js":{"imports":[{"path":"target/web/chunk.D.js","kind":"import-statement"}]}
            }}"#,
        )
        .unwrap();
        assert_eq!(chunks.closure("web/main.ts").unwrap(), ["/assets/main.A.js", "/assets/chunk.D.js"]);
        let city = chunks.closure("web/city/index.ts").unwrap();
        assert_eq!(city.len(), 3);
        assert_eq!(city[0], "/assets/city.H.js");
        assert!(chunks.closure("web/nope.ts").is_err());
    }
}
