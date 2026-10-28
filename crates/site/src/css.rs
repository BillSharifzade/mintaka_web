//! CSS with Lightning CSS (written in Rust): bundle `@import`s, lower modern syntax for the
//! browsers we support, minify.

use anyhow::{Result, anyhow};
use lightningcss::bundler::{Bundler, FileProvider};
use lightningcss::stylesheet::{MinifyOptions, ParserOptions, PrinterOptions, StyleSheet};
use lightningcss::targets::{Browsers, Targets};
use std::path::Path;

/// The oldest browsers the page is tuned for. Older ones still get readable content.
fn targets() -> Targets {
    let v = |major: u32, minor: u32| Some((major << 16) | (minor << 8));
    Targets::from(Browsers {
        chrome: v(111, 0),
        edge: v(111, 0),
        firefox: v(115, 0),
        safari: v(16, 4),
        ios_saf: v(16, 4),
        android: v(111, 0),
        samsung: v(22, 0),
        opera: v(97, 0),
        ..Browsers::default()
    })
}

/// Bundle and minify a stylesheet entry point.
pub fn bundle(entry: &Path) -> Result<String> {
    let fs = FileProvider::new();
    let mut bundler = Bundler::new(&fs, None, ParserOptions::default());
    let mut sheet = bundler.bundle(entry).map_err(|e| anyhow!("CSS bundle {}: {e}", entry.display()))?;
    finish(&mut sheet)
}

/// Minify a stylesheet given as a string (generated CSS such as @font-face rules).
pub fn minify(source: &str) -> Result<String> {
    let mut sheet = StyleSheet::parse(source, ParserOptions::default()).map_err(|e| anyhow!("CSS parse: {e}"))?;
    finish(&mut sheet)
}

fn finish(sheet: &mut StyleSheet) -> Result<String> {
    let targets = targets();
    sheet.minify(MinifyOptions { targets, ..Default::default() }).map_err(|e| anyhow!("CSS minify: {e}"))?;
    let out = sheet
        .to_css(PrinterOptions { minify: true, targets, ..Default::default() })
        .map_err(|e| anyhow!("CSS print: {e}"))?;
    Ok(out.code)
}
