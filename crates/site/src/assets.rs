//! Everything written to `dist/`, and the route table the server will load.

use crate::util::short_hash;
use anyhow::{Context, Result};
use proof::{CachePolicy, Encoding, Route, RouteKind};
use std::io::Write as _;
use std::path::{Path, PathBuf};

pub struct Dist {
    pub dir: PathBuf,
    pub routes: Vec<Route>,
}

impl Dist {
    pub fn create(dir: &Path) -> Result<Self> {
        if dir.exists() {
            std::fs::remove_dir_all(dir).with_context(|| format!("cleaning {}", dir.display()))?;
        }
        std::fs::create_dir_all(dir)?;
        Ok(Self { dir: dir.to_owned(), routes: vec![] })
    }

    /// Write a file without registering a route (e.g. curl-mode text, 404 pages).
    pub fn write(&self, rel: &str, bytes: &[u8]) -> Result<()> {
        let path = self.dir.join(rel);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&path, bytes).with_context(|| format!("writing {}", path.display()))
    }

    /// Write a file and serve it at `url`.
    #[allow(clippy::too_many_arguments)]
    pub fn add(&mut self, url: &str, rel: &str, bytes: &[u8], content_type: &str, cache: CachePolicy, kind: RouteKind, lang: Option<&str>) -> Result<()> {
        self.write(rel, bytes)?;
        self.routes.push(Route {
            path: url.to_owned(),
            file: rel.to_owned(),
            content_type: content_type.to_owned(),
            cache,
            encodings: vec![],
            lang: lang.map(str::to_owned),
            kind,
        });
        Ok(())
    }

    /// Write a content-hashed asset: `/assets/{stem}.{HASH}.{ext}`. Returns its URL.
    pub fn add_hashed(&mut self, stem: &str, ext: &str, bytes: &[u8], content_type: &str) -> Result<String> {
        let name = format!("{stem}.{}.{ext}", short_hash(bytes));
        let rel = format!("assets/{name}");
        let url = format!("/{rel}");
        self.add(&url, &rel, bytes, content_type, CachePolicy::Immutable, RouteKind::Asset, None)?;
        Ok(url)
    }

    /// Precompress every compressible route (brotli 11, gzip 9) when it saves at least 5 %.
    pub fn compress_all(&mut self) -> Result<()> {
        for route in &mut self.routes {
            if !compressible(&route.content_type) {
                continue;
            }
            let path = self.dir.join(&route.file);
            let raw = std::fs::read(&path)?;
            let br = brotli(&raw)?;
            if (br.len() as f64) < raw.len() as f64 * 0.95 {
                std::fs::write(path.with_file_name(format!("{}.br", file_name(&path))), &br)?;
                route.encodings.push(Encoding::Br);
            }
            let gz = gzip(&raw)?;
            if (gz.len() as f64) < raw.len() as f64 * 0.95 {
                std::fs::write(path.with_file_name(format!("{}.gz", file_name(&path))), &gz)?;
                route.encodings.push(Encoding::Gzip);
            }
        }
        Ok(())
    }

    /// Size in bytes of a route's best encoding (brotli if present).
    pub fn wire_size(&self, route: &Route) -> u64 {
        let path = self.dir.join(&route.file);
        let br = path.with_file_name(format!("{}.br", file_name(&path)));
        std::fs::metadata(if route.encodings.contains(&Encoding::Br) { br } else { path }).map(|m| m.len()).unwrap_or(0)
    }
}

fn file_name(p: &Path) -> String {
    p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()
}

fn compressible(content_type: &str) -> bool {
    content_type.starts_with("text/")
        || content_type.starts_with("application/json")
        || content_type.starts_with("application/manifest+json")
        || content_type.starts_with("application/xml")
        || content_type.starts_with("application/atom+xml")
        || content_type.starts_with("image/svg+xml")
        || content_type.starts_with("image/x-icon")
        || content_type.starts_with("application/wasm")
        || content_type.starts_with("application/octet-stream")
}

pub fn brotli(data: &[u8]) -> Result<Vec<u8>> {
    let mut out = Vec::with_capacity(data.len() / 3);
    let params = brotli::enc::BrotliEncoderParams { quality: 11, lgwin: 22, ..Default::default() };
    brotli::BrotliCompress(&mut &data[..], &mut out, &params)?;
    Ok(out)
}

pub fn gzip(data: &[u8]) -> Result<Vec<u8>> {
    let mut enc = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::best());
    enc.write_all(data)?;
    Ok(enc.finish()?)
}

pub fn content_type_for(ext: &str) -> &'static str {
    match ext {
        "html" => "text/html; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "js" | "mjs" => "text/javascript; charset=utf-8",
        "json" => "application/json",
        "webmanifest" => "application/manifest+json",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "avif" => "image/avif",
        "webp" => "image/webp",
        "ico" => "image/x-icon",
        "woff2" => "font/woff2",
        "wasm" => "application/wasm",
        "txt" => "text/plain; charset=utf-8",
        "xml" => "application/xml; charset=utf-8",
        "atom" => "application/atom+xml; charset=utf-8",
        "map" => "application/json",
        _ => "application/octet-stream",
    }
}
