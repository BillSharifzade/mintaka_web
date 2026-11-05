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

/// Drop the custom properties a minified stylesheet declares but never reads through `var()` (the
/// shared tokens carry many a single page has no use for), then any rule left empty. Repeats until
/// nothing changes, since a dropped value may have been another property's only reader.
///
/// Only for a page whose script never reads custom properties itself (the studio page's doesn't).
pub fn prune_custom_properties(css: &str) -> String {
    let mut css = css.to_owned();
    loop {
        let decls = custom_declarations(&css);
        let used: std::collections::HashSet<&str> = css
            .match_indices("var(--")
            .map(|(i, _)| {
                let name = &css[i + 4..];
                &name[..name.find(|c: char| !(c.is_ascii_alphanumeric() || c == '-' || c == '_')).unwrap_or(name.len())]
            })
            .collect();
        let dead: Vec<(usize, usize)> = decls.into_iter().filter(|(_, _, name)| !used.contains(name.as_str())).map(|(a, b, _)| (a, b)).collect();
        if dead.is_empty() {
            break;
        }
        for (a, b) in dead.into_iter().rev() {
            css.replace_range(a..b, "");
        }
        css = css.replace(";}", "}").replace("{;", "{");
    }
    drop_empty_rules(&css)
}

/// Custom property declarations in minified CSS: (start, end, name), the span including the
/// separating `;` when there is one. Quotes and brackets are skipped over, so a value holding `;`
/// or `}` in a string never ends the declaration early.
fn custom_declarations(css: &str) -> Vec<(usize, usize, String)> {
    let b = css.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i + 2 < b.len() {
        let at_start = i == 0 || matches!(b[i - 1], b'{' | b';');
        if at_start && b[i] == b'-' && b[i + 1] == b'-' {
            let colon = match css[i..].find(':') {
                Some(c) => i + c,
                None => break,
            };
            let name = &css[i..colon];
            if name.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'-' || c == b'_') {
                let (mut j, mut depth, mut quote) = (colon + 1, 0i32, 0u8);
                while j < b.len() {
                    let c = b[j];
                    if quote != 0 {
                        if c == b'\\' {
                            j += 1;
                        } else if c == quote {
                            quote = 0;
                        }
                    } else {
                        match c {
                            b'"' | b'\'' => quote = c,
                            b'(' | b'[' | b'{' => depth += 1,
                            b')' | b']' => depth -= 1,
                            b'}' if depth > 0 => depth -= 1,
                            b';' | b'}' if depth == 0 => break,
                            _ => {}
                        }
                    }
                    j += 1;
                }
                let end = if j < b.len() && b[j] == b';' { j + 1 } else { j };
                out.push((i, end, name.to_owned()));
                i = end;
                continue;
            }
        }
        i += 1;
    }
    out
}

/// Remove style rules (and `@media`/`@supports` blocks) that have become empty. Layer blocks stay,
/// so the cascade's layer order can never change.
fn drop_empty_rules(css: &str) -> String {
    let mut css = css.to_owned();
    loop {
        let before = css.len();
        let mut i = 0;
        while let Some(off) = css[i..].find("{}") {
            let close = i + off;
            // The prelude runs back to the previous `}`, `{` or `;`.
            let start = css[..close].rfind(['}', '{', ';']).map_or(0, |p| p + 1);
            let prelude = &css[start..close];
            if prelude.starts_with("@layer") || prelude.starts_with("@font-face") || prelude.is_empty() {
                i = close + 2;
                continue;
            }
            css.replace_range(start..close + 2, "");
            i = start;
        }
        if css.len() == before {
            return css;
        }
    }
}

fn finish(sheet: &mut StyleSheet) -> Result<String> {
    let targets = targets();
    sheet.minify(MinifyOptions { targets, ..Default::default() }).map_err(|e| anyhow!("CSS minify: {e}"))?;
    let out = sheet
        .to_css(PrinterOptions { minify: true, targets, ..Default::default() })
        .map_err(|e| anyhow!("CSS print: {e}"))?;
    Ok(out.code)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unread_custom_properties_go() {
        assert_eq!(prune_custom_properties(":root{--a:1;--b:2}.x{color:var(--a)}"), ":root{--a:1}.x{color:var(--a)}");
        assert_eq!(prune_custom_properties(":root{--b:2;--a:1}.x{color:var(--a)}"), ":root{--a:1}.x{color:var(--a)}");
    }

    #[test]
    fn a_property_read_only_by_a_dropped_one_goes_too() {
        assert_eq!(prune_custom_properties(":root{--a:var(--b);--b:1;--c:2}.x{top:0}"), ".x{top:0}");
    }

    #[test]
    fn strings_and_brackets_never_end_a_value() {
        let css = r#":root{--s:"a;b}";--f:calc((1px + 2px));--t:1}.x{content:var(--s);width:var(--f)}"#;
        assert_eq!(prune_custom_properties(css), r#":root{--s:"a;b}";--f:calc((1px + 2px))}.x{content:var(--s);width:var(--f)}"#);
    }

    #[test]
    fn emptied_rules_and_media_go_but_layers_stay() {
        assert_eq!(prune_custom_properties("@media (x){:root{--z:1}}.y{a:b}"), ".y{a:b}");
        assert_eq!(prune_custom_properties("@layer t{:root{--z:1}}.y{a:b}"), "@layer t{}.y{a:b}");
    }
}
