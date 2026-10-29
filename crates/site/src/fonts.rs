//! Web fonts: hashed copies, `@font-face` rules, metric-matched fallbacks (no layout shift when
//! the web font swaps in) and a glyph-coverage check that reads each shipped WOFF2's `cmap`.

use crate::assets::{Dist, content_type_for};
use anyhow::{Context, Result, bail};
use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::path::Path;

const LATIN: &str = "U+0000-00FF,U+0131,U+0152-0153,U+02BB-02BC,U+02C6,U+02DA,U+02DC,U+0304,U+0308,U+0329,U+2000-206F,U+20AC,U+2122,U+2191,U+2193,U+2212,U+2215,U+FEFF,U+FFFD";
const LATIN_EXT: &str = "U+0100-02BA,U+02BD-02C5,U+02C7-02CC,U+02CE-02D7,U+02DD-02FF,U+0304,U+0308,U+0329,U+1D00-1DBF,U+1E00-1E9F,U+1EF2-1EFF,U+2020,U+20A0-20AB,U+20AD-20C0,U+2113,U+2C60-2C7F,U+A720-A7FF";
const CYRILLIC: &str = "U+0301,U+0400-045F,U+0490-0491,U+04B0-04B1,U+2116";
const CYRILLIC_EXT: &str = "U+0460-052F,U+1C80-1C8A,U+20B4,U+2DE0-2DFF,U+A640-A69F,U+FE2E-FE2F";
const MATH: &str = "U+0302-0303,U+0305,U+0307-0308,U+0310,U+0312,U+0315,U+031A,U+0326-0327,U+032C,U+032F-0330,U+0332-0333,U+0338,U+033A,U+0346,U+034D,U+0391-03A1,U+03A3-03A9,U+03B1-03C9,U+03D1,U+03D5-03D6,U+03F0-03F1,U+03F4-03F5,U+2016-2017,U+2034-2038,U+203C,U+2040,U+2043,U+2047,U+2050,U+2057,U+205F,U+2070-2071,U+2074-208E,U+2090-209C,U+20D0-20DC,U+20E1,U+20E5-20EF,U+2100-2112,U+2114-2115,U+2117-2121,U+2123-214F,U+2190,U+2192,U+2194-21AE,U+21B0-21E5,U+21F1-21F2,U+21F4-2211,U+2213-2214,U+2216-22FF,U+2308-230B,U+2310,U+2319,U+231C-2321,U+2336-237A,U+237C,U+2395,U+239B-23B7,U+23D0,U+23DC-23E1,U+2474-2475,U+25AF,U+25B3,U+25B7,U+25BD,U+25C1,U+25CA,U+25CC,U+25FB,U+266D-266F,U+27C0-27FF,U+2900-2AFF,U+2B0E-2B11,U+2B30-2B4C,U+2BFE,U+3030,U+FF5B,U+FF5D,U+1D400-1D7FF,U+1EE00-1EEFF";

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Role {
    Sans,
    Mono,
}

struct FaceDef {
    family: &'static str,
    file: &'static str,
    weight: &'static str,
    range: &'static str,
    role: Role,
    /// Languages whose pages preload this file.
    preload: &'static [&'static str],
}

const FACES: &[FaceDef] = &[
    FaceDef { family: "Space Grotesk", file: "space-grotesk-latin-wght-normal.woff2", weight: "300 700", range: LATIN, role: Role::Sans, preload: &["en", "ru", "tg"] },
    FaceDef { family: "Space Grotesk", file: "space-grotesk-latin-ext-wght-normal.woff2", weight: "300 700", range: LATIN_EXT, role: Role::Sans, preload: &[] },
    FaceDef { family: "Onest", file: "onest-cyrillic-wght-normal.woff2", weight: "100 900", range: CYRILLIC, role: Role::Sans, preload: &["ru", "tg"] },
    FaceDef { family: "Onest", file: "onest-cyrillic-ext-wght-normal.woff2", weight: "100 900", range: CYRILLIC_EXT, role: Role::Sans, preload: &["tg"] },
    FaceDef { family: "Onest", file: "onest-math-wght-normal.woff2", weight: "100 900", range: MATH, role: Role::Sans, preload: &[] },
    FaceDef { family: "IBM Plex Mono", file: "ibm-plex-mono-latin-400-normal.woff2", weight: "400", range: LATIN, role: Role::Mono, preload: &[] },
    FaceDef { family: "IBM Plex Mono", file: "ibm-plex-mono-cyrillic-400-normal.woff2", weight: "400", range: CYRILLIC, role: Role::Mono, preload: &[] },
    FaceDef { family: "IBM Plex Mono", file: "ibm-plex-mono-cyrillic-ext-400-normal.woff2", weight: "400", range: CYRILLIC_EXT, role: Role::Mono, preload: &[] },
];

/// Metrics of the local fonts we adjust to look like the web fonts while those load
/// (OS/2 xAvgCharWidth and hhea metrics, font units).
struct LocalMetrics {
    locals: &'static [&'static str],
    upm: f64,
    avg_width: f64,
}

const ARIAL: LocalMetrics = LocalMetrics { locals: &["Arial", "ArialMT", "Liberation Sans", "Arimo"], upm: 2048.0, avg_width: 904.0 };
const COURIER_NEW: LocalMetrics = LocalMetrics { locals: &["Courier New", "CourierNewPSMT", "Liberation Mono", "Cousine"], upm: 2048.0, avg_width: 1229.0 };

pub struct Fonts {
    /// `@font-face` rules, ready to minify.
    pub css: String,
    /// (lang, url) preload hints.
    preloads: Vec<(&'static str, String)>,
    sans: BTreeSet<u32>,
    mono: BTreeSet<u32>,
}

impl Fonts {
    pub fn process(root: &Path, dist: &mut Dist) -> Result<Self> {
        let mut css = String::new();
        let mut preloads = vec![];
        let mut sans = BTreeSet::new();
        let mut mono = BTreeSet::new();
        let mut fallback_source: [Option<FaceInfo>; 2] = [None, None];

        for def in FACES {
            let path = root.join("assets/fonts").join(def.file);
            let bytes = std::fs::read(&path).with_context(|| format!("reading {}", path.display()))?;
            let info = inspect_woff2(&bytes).with_context(|| format!("parsing {}", path.display()))?;
            let ranges = parse_unicode_range(def.range)?;
            let covered: BTreeSet<u32> = info.codepoints.iter().copied().filter(|c| ranges.iter().any(|(a, b)| c >= a && c <= b)).collect();
            match def.role {
                Role::Sans => sans.extend(&covered),
                Role::Mono => mono.extend(&covered),
            }
            let slot = if def.role == Role::Sans { 0 } else { 1 };
            if fallback_source[slot].is_none() {
                fallback_source[slot] = Some(info.clone());
            }

            let stem = def.file.trim_end_matches(".woff2").trim_end_matches("-normal").trim_end_matches("-wght");
            let url = dist.add_hashed(&format!("fonts/{stem}"), "woff2", &bytes, content_type_for("woff2"))?;
            for lang in def.preload {
                preloads.push((*lang, url.clone()));
            }
            let _ = write!(
                css,
                "@font-face{{font-family:\"{}\";src:url({url}) format(\"woff2\");font-weight:{};font-style:normal;font-display:swap;unicode-range:{}}}",
                def.family, def.weight, def.range
            );
        }

        // Fallback faces sized to the web fonts' metrics, so swapping causes no layout shift.
        for (slot, (name, local)) in [("Space Grotesk Fallback", ARIAL), ("IBM Plex Mono Fallback", COURIER_NEW)].into_iter().enumerate() {
            let Some(info) = &fallback_source[slot] else { continue };
            let size_adjust = (info.avg_width / info.upm) / (local.avg_width / local.upm);
            let pct = |v: f64| format!("{:.2}%", v / info.upm / size_adjust * 100.0);
            let srcs: Vec<String> = local.locals.iter().map(|l| format!("local(\"{l}\")")).collect();
            let _ = write!(
                css,
                "@font-face{{font-family:\"{name}\";src:{};size-adjust:{:.2}%;ascent-override:{};descent-override:{};line-gap-override:{}}}",
                srcs.join(","),
                size_adjust * 100.0,
                pct(info.ascender),
                pct(info.descender.abs()),
                pct(info.line_gap)
            );
        }

        Ok(Self { css, preloads, sans, mono })
    }

    pub fn preloads_for<'a>(&'a self, lang: &'a str) -> impl Iterator<Item = &'a str> + 'a {
        self.preloads.iter().filter(move |(l, _)| *l == lang).map(|(_, u)| u.as_str())
    }

    /// Characters in `html` that no shipped font renders (in their sans or mono context).
    pub fn uncovered(&self, html: &str) -> BTreeSet<char> {
        let mut missing = BTreeSet::new();
        for (text, role) in text_runs(html) {
            for c in text.chars() {
                // Not drawn: white space, controls, the emoji selector, and MathML's invisible
                // operators (U+2061–2064: function application, invisible times, separator, plus).
                if c.is_whitespace() || c.is_control() || c == '\u{FE0F}' || ('\u{2061}'..='\u{2064}').contains(&c) {
                    continue;
                }
                let cp = c as u32;
                // Both stacks fall back through Onest (in `sans`) before the system font.
                let ok = match role {
                    Role::Sans => self.sans.contains(&cp),
                    Role::Mono => self.mono.contains(&cp) || self.sans.contains(&cp),
                };
                if !ok {
                    missing.insert(c);
                }
            }
        }
        missing
    }

    /// Characters in plain strings (island copy) that neither stack renders.
    pub fn uncovered_text(&self, text: &str) -> BTreeSet<char> {
        text.chars()
            .filter(|c| !c.is_whitespace() && !c.is_control())
            .filter(|c| !self.sans.contains(&(*c as u32)) && !self.mono.contains(&(*c as u32)))
            .collect()
    }
}

#[derive(Clone)]
struct FaceInfo {
    upm: f64,
    ascender: f64,
    descender: f64,
    line_gap: f64,
    avg_width: f64,
    codepoints: Vec<u32>,
}

/// Decode just enough of a WOFF2 file to read `cmap`, `head`, `hhea`, `maxp` and `OS/2`.
/// Those tables are never transformed, so they come straight out of the brotli stream.
fn inspect_woff2(data: &[u8]) -> Result<FaceInfo> {
    const KNOWN: [&[u8; 4]; 63] = [
        b"cmap", b"head", b"hhea", b"hmtx", b"maxp", b"name", b"OS/2", b"post", b"cvt ", b"fpgm", b"glyf", b"loca", b"prep", b"CFF ", b"VORG",
        b"EBDT", b"EBLC", b"gasp", b"hdmx", b"kern", b"LTSH", b"PCLT", b"VDMX", b"vhea", b"vmtx", b"BASE", b"GDEF", b"GPOS", b"GSUB", b"EBSC",
        b"JSTF", b"MATH", b"CBDT", b"CBLC", b"COLR", b"CPAL", b"SVG ", b"sbix", b"acnt", b"avar", b"bdat", b"bloc", b"bsln", b"cvar", b"fdsc",
        b"feat", b"fmtx", b"fvar", b"gvar", b"hsty", b"just", b"lcar", b"mort", b"morx", b"opbd", b"prop", b"trak", b"Zapf", b"Silf", b"Glat",
        b"Gloc", b"Feat", b"Sill",
    ];
    let be16 = |o: usize| -> Result<u16> { Ok(u16::from_be_bytes(data.get(o..o + 2).context("truncated")?.try_into()?)) };
    let be32 = |o: usize| -> Result<u32> { Ok(u32::from_be_bytes(data.get(o..o + 4).context("truncated")?.try_into()?)) };
    if data.get(..4) != Some(&b"wOF2"[..]) {
        bail!("not a WOFF2 file");
    }
    let num_tables = be16(12)? as usize;
    let compressed_len = be32(20)? as usize;

    let mut pos = 48;
    let mut tables: Vec<([u8; 4], usize)> = Vec::with_capacity(num_tables);
    for _ in 0..num_tables {
        let flags = *data.get(pos).context("truncated directory")?;
        pos += 1;
        let tag: [u8; 4] = if flags & 0x3F == 0x3F {
            let t = data.get(pos..pos + 4).context("truncated tag")?.try_into()?;
            pos += 4;
            t
        } else {
            *KNOWN[(flags & 0x3F) as usize]
        };
        let orig_len = base128(data, &mut pos)?;
        let version = flags >> 6;
        let transformed = if &tag == b"glyf" || &tag == b"loca" { version == 0 } else { version != 0 };
        let len = if transformed { base128(data, &mut pos)? } else { orig_len };
        tables.push((tag, len as usize));
    }

    let stream = data.get(pos..pos + compressed_len).context("truncated table stream")?;
    let mut raw = Vec::new();
    brotli::BrotliDecompress(&mut &stream[..], &mut raw).context("brotli")?;

    // Tables sit back to back in the decompressed stream, in directory order, without padding.
    let slice = |name: &[u8; 4]| -> Option<&[u8]> {
        let mut o = 0;
        for (tag, len) in &tables {
            if tag == name {
                return raw.get(o..o + len);
            }
            o += len;
        }
        None
    };

    let (head, hhea, maxp) = (slice(b"head").context("no head")?, slice(b"hhea").context("no hhea")?, slice(b"maxp").context("no maxp")?);
    let os2 = slice(b"OS/2");
    let tables_in = ttf_parser::RawFaceTables { head, hhea, maxp, cmap: slice(b"cmap"), os2, ..Default::default() };
    let face = ttf_parser::Face::from_raw_tables(tables_in).map_err(|e| anyhow::anyhow!("face: {e}"))?;

    let mut codepoints = vec![];
    if let Some(cmap) = face.tables().cmap {
        for sub in cmap.subtables {
            if sub.is_unicode() {
                sub.codepoints(|cp| {
                    if sub.glyph_index(cp).is_some_and(|g| g.0 != 0) {
                        codepoints.push(cp);
                    }
                });
            }
        }
    }
    codepoints.sort_unstable();
    codepoints.dedup();

    let avg_width = os2.and_then(|t| t.get(2..4)).map(|b| i16::from_be_bytes([b[0], b[1]]) as f64).unwrap_or(500.0);
    Ok(FaceInfo {
        upm: face.units_per_em() as f64,
        ascender: face.ascender() as f64,
        descender: face.descender() as f64,
        line_gap: face.line_gap() as f64,
        avg_width,
        codepoints,
    })
}

fn base128(data: &[u8], pos: &mut usize) -> Result<u32> {
    let mut acc: u32 = 0;
    for i in 0..5 {
        let b = *data.get(*pos).context("truncated UIntBase128")?;
        *pos += 1;
        if i == 0 && b == 0x80 {
            bail!("UIntBase128 with leading zero");
        }
        if acc & 0xFE00_0000 != 0 {
            bail!("UIntBase128 overflow");
        }
        acc = (acc << 7) | u32::from(b & 0x7F);
        if b & 0x80 == 0 {
            return Ok(acc);
        }
    }
    bail!("UIntBase128 longer than 5 bytes")
}

fn parse_unicode_range(s: &str) -> Result<Vec<(u32, u32)>> {
    s.split(',')
        .map(|part| {
            let p = part.trim().trim_start_matches("U+");
            let (a, b) = p.split_once('-').unwrap_or((p, p));
            Ok((u32::from_str_radix(a, 16)?, u32::from_str_radix(b, 16)?))
        })
        .collect()
}

/// Text content of an HTML document split into sans and mono runs. Mono context: `code`, `pre`,
/// `kbd`, `samp`, or any element whose class list has one of our mono classes.
fn text_runs(html: &str) -> Vec<(String, Role)> {
    const MONO_CLASSES: &[&str] = &["num", "mono", "source", "badge", "instrument__id", "callout__xy", "chip--mono", "code"];
    const VOID: &[&str] = &["area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "source", "track", "wbr"];
    let mut runs = vec![];
    let mut stack: Vec<(String, bool)> = vec![];
    let mut rest = html;
    let mut skip_until: Option<&str> = None;
    while !rest.is_empty() {
        if let Some(end_tag) = skip_until {
            match rest.find(end_tag) {
                Some(i) => {
                    rest = &rest[i + end_tag.len()..];
                    skip_until = None;
                }
                None => break,
            }
            continue;
        }
        let Some(lt) = rest.find('<') else {
            push_text(&mut runs, rest, &stack);
            break;
        };
        push_text(&mut runs, &rest[..lt], &stack);
        rest = &rest[lt..];
        if rest.starts_with("<!--") {
            rest = rest.find("-->").map(|i| &rest[i + 3..]).unwrap_or("");
            continue;
        }
        let gt = rest.find('>').unwrap_or(rest.len() - 1);
        let tag = &rest[1..gt];
        rest = &rest[gt + 1..];
        if let Some(name) = tag.strip_prefix('/') {
            let name = name.trim().to_ascii_lowercase();
            if let Some(i) = stack.iter().rposition(|(n, _)| *n == name) {
                stack.truncate(i);
            }
            continue;
        }
        if tag.starts_with('!') {
            continue;
        }
        let name: String = tag.split(|c: char| c.is_whitespace() || c == '/').next().unwrap_or("").to_ascii_lowercase();
        match name.as_str() {
            "script" => skip_until = Some("</script>"),
            "style" => skip_until = Some("</style>"),
            "svg" if !tag.contains("data-text") => skip_until = Some("</svg>"),
            _ => {}
        }
        if skip_until.is_some() || VOID.contains(&name.as_str()) || tag.ends_with('/') {
            continue;
        }
        let class_mono = attr(tag, "class").is_some_and(|c| c.split_whitespace().any(|t| MONO_CLASSES.contains(&t)));
        let mono = matches!(name.as_str(), "code" | "pre" | "kbd" | "samp") || class_mono;
        stack.push((name, mono));
    }
    runs
}

fn push_text(runs: &mut Vec<(String, Role)>, raw: &str, stack: &[(String, bool)]) {
    if raw.trim().is_empty() {
        return;
    }
    let role = if stack.iter().any(|(_, m)| *m) { Role::Mono } else { Role::Sans };
    runs.push((decode_entities(raw), role));
}

fn attr<'a>(tag: &'a str, name: &str) -> Option<&'a str> {
    let key = format!("{name}=\"");
    let start = tag.find(&key)? + key.len();
    let end = tag[start..].find('"')? + start;
    Some(&tag[start..end])
}

fn decode_entities(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(i) = rest.find('&') {
        out.push_str(&rest[..i]);
        rest = &rest[i..];
        let Some(semi) = rest.find(';') else { break };
        let ent = &rest[1..semi];
        let decoded = match ent {
            "amp" => Some('&'),
            "lt" => Some('<'),
            "gt" => Some('>'),
            "quot" => Some('"'),
            "apos" | "#39" => Some('\''),
            "nbsp" => Some('\u{A0}'),
            e if e.starts_with("#x") => u32::from_str_radix(&e[2..], 16).ok().and_then(char::from_u32),
            e if e.starts_with('#') => e[1..].parse().ok().and_then(char::from_u32),
            _ => None,
        };
        match decoded {
            Some(c) => {
                out.push(c);
                rest = &rest[semi + 1..];
            }
            None => {
                out.push('&');
                rest = &rest[1..];
            }
        }
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base128_decodes() {
        let mut p = 0;
        assert_eq!(base128(&[0x3F], &mut p).unwrap(), 63);
        let mut p = 0;
        assert_eq!(base128(&[0x81, 0x00], &mut p).unwrap(), 128);
        let mut p = 0;
        assert!(base128(&[0x80, 0x01], &mut p).is_err());
    }

    #[test]
    fn mono_context_is_tracked() {
        let runs = text_runs(r#"<p>Hi <span class="num">42<span class="num__unit">µs</span></span> there</p><script>x<y</script>"#);
        let mono: String = runs.iter().filter(|(_, r)| *r == Role::Mono).map(|(t, _)| t.as_str()).collect();
        let sans: String = runs.iter().filter(|(_, r)| *r == Role::Sans).map(|(t, _)| t.as_str()).collect();
        assert_eq!(mono, "42µs");
        assert_eq!(sans, "Hi  there");
    }

    #[test]
    fn shipped_fonts_cover_tajik_letters() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let bytes = std::fs::read(root.join("assets/fonts/onest-cyrillic-ext-wght-normal.woff2")).unwrap();
        let info = inspect_woff2(&bytes).unwrap();
        for c in "ҒғӢӣҚқӮӯҲҳҶҷ".chars() {
            assert!(info.codepoints.contains(&(c as u32)), "Onest is missing {c}");
        }
        let bytes = std::fs::read(root.join("assets/fonts/space-grotesk-latin-wght-normal.woff2")).unwrap();
        let info = inspect_woff2(&bytes).unwrap();
        assert!(info.codepoints.contains(&('µ' as u32)));
        assert!(!info.codepoints.contains(&('Ж' as u32)), "Space Grotesk has no Cyrillic");
        assert_eq!(info.upm, 1000.0);
    }
}
