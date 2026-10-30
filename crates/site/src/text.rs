//! curl mode (spec §7.4): plain-text pages with 24-bit ANSI colour. The mark is rasterised from
//! the brand kit's SVG with resvg and drawn with half-block characters, so the terminal logo is
//! the real logo. `{{tokens}}` are filled by the server per request (CONTRACTS §4.6).

use crate::config::Contact;
use crate::i18n::Strings;
use anyhow::{Context, Result, anyhow};
use std::fmt::Write as _;
use std::path::Path;

const STARLIGHT: (u8, u8, u8) = (244, 245, 247);
const MIST: (u8, u8, u8) = (201, 206, 216);
const BLUE: (u8, u8, u8) = (61, 123, 255);
const RUST: (u8, u8, u8) = (226, 85, 45);

fn fg((r, g, b): (u8, u8, u8)) -> String {
    format!("\x1b[38;2;{r};{g};{b}m")
}
fn bg((r, g, b): (u8, u8, u8)) -> String {
    format!("\x1b[48;2;{r};{g};{b}m")
}
const RESET: &str = "\x1b[0m";

type Rgb = (u8, u8, u8);
/// One terminal cell: the half-block character, its foreground and (optional) background.
type Cell = (char, Option<Rgb>, Option<Rgb>);
const BOLD: &str = "\x1b[1m";

/// Rasterise the simplified mark into `cols` × (2 × rows) cells of brand colours.
pub fn mark_ansi(root: &Path, cols: u32) -> Result<String> {
    let svg = std::fs::read(root.join("assets/brand/logo/mintaka-mark-simplified-on-dark.svg")).context("reading mark")?;
    let tree = resvg::usvg::Tree::from_data(&svg, &resvg::usvg::Options::default()).map_err(|e| anyhow!("usvg: {e}"))?;
    let size = tree.size();
    let ss = 6; // supersampling factor per output pixel
    let w = cols * ss;
    let px_rows = ((size.height() / size.width()) * cols as f32).round() as u32;
    let px_rows = px_rows + px_rows % 2;
    let h = px_rows * ss;
    let mut pixmap = resvg::tiny_skia::Pixmap::new(w, h).context("pixmap")?;
    let scale = w as f32 / size.width();
    resvg::render(&tree, resvg::tiny_skia::Transform::from_scale(scale, scale), &mut pixmap.as_mut());

    // Downsample: a cell pixel is filled when ≥ 45 % covered; its colour is the dominant brand colour.
    let palette = [STARLIGHT, RUST, BLUE];
    let mut grid = vec![None::<(u8, u8, u8)>; (cols * px_rows) as usize];
    for py in 0..px_rows {
        for px in 0..cols {
            let mut votes = [0u32; 3];
            let mut covered = 0u32;
            for sy in 0..ss {
                for sx in 0..ss {
                    let p = pixmap.pixel(px * ss + sx, py * ss + sy).map(|p| p.demultiply());
                    if let Some(c) = p.filter(|c| c.alpha() > 127) {
                        covered += 1;
                        let nearest = palette
                            .iter()
                            .enumerate()
                            .min_by_key(|(_, (r, g, b))| {
                                let d = |a: u8, b: u8| (a as i32 - b as i32).pow(2);
                                d(c.red(), *r) + d(c.green(), *g) + d(c.blue(), *b)
                            })
                            .map(|(i, _)| i)
                            .unwrap_or(0);
                        votes[nearest] += 1;
                    }
                }
            }
            if covered * 100 >= ss * ss * 45 {
                // Small coloured stars win over the large white shape they sit next to.
                let pick = if votes[1] * 3 >= covered { 1 } else if votes[2] * 3 >= covered { 2 } else { 0 };
                grid[(py * cols + px) as usize] = Some(palette[pick]);
            }
        }
    }

    // Emit a colour escape only when the colour changes: the file stays a fraction of the size.
    let mut out = String::new();
    for row in 0..px_rows / 2 {
        let cells: Vec<Cell> = (0..cols)
            .map(|x| {
                let top = grid[((row * 2) * cols + x) as usize];
                let bot = grid[((row * 2 + 1) * cols + x) as usize];
                match (top, bot) {
                    (None, None) => (' ', None, None),
                    (Some(t), None) => ('▀', Some(t), None),
                    (None, Some(b)) => ('▄', Some(b), None),
                    (Some(t), Some(b)) if t == b => ('█', Some(t), None),
                    (Some(t), Some(b)) => ('▀', Some(t), Some(b)),
                }
            })
            .collect();
        let used = cells.iter().rposition(|c| c.0 != ' ').map_or(0, |i| i + 1);
        let mut line = String::from("  ");
        let (mut cur_fg, mut cur_bg) = (None, None);
        for &(ch, f, b) in &cells[..used] {
            if ch != ' ' && f != cur_fg {
                line.push_str(&fg(f.unwrap_or(STARLIGHT)));
                cur_fg = f;
            }
            if b != cur_bg {
                match b {
                    Some(c) => line.push_str(&bg(c)),
                    None => line.push_str("\x1b[49m"),
                }
                cur_bg = b;
            }
            line.push(ch);
        }
        if used > 0 {
            line.push_str(RESET);
        }
        out.push_str(&line);
        out.push('\n');
    }
    Ok(out)
}

pub struct Pages {
    pub index: String,
    pub pillars: String,
    pub contact: String,
}

/// `host` is where the site is, without the scheme: `mintaka.tj`, or `host/prefix` under a URL
/// prefix. `lang_path` is `/{lang}` for every language but the default one, so the `curl` hints
/// keep the reader in their language (curl sends no `Accept-Language`). `/proof` has one language
/// only.
pub fn pages(root: &Path, s: &Strings, host: &str, lang_path: &str, contact: &Contact) -> Result<Pages> {
    let art = mark_ansi(root, 22)?;
    let (w, m, b, r) = (fg(STARLIGHT), fg(MIST), fg(BLUE), fg(RUST));
    let footer = format!(
        "\n  {m}{} {w}{{{{server_time}}}}{m} · {{{{tls}}}} · {{{{http}}}} · {}{RESET}\n  {m}{} {{{{build}}}}{RESET}\n\n",
        s.t("curl-served"),
        s.t("curl-cookies"),
        s.t("curl-build")
    );
    let pillar_rows = [
        ("pillar-efficiency", "curl-fast", format!("{lang_path}/pillars")),
        ("pillar-security", "curl-secure", "/proof".to_owned()),
        ("pillar-highload", "curl-scaled", format!("{lang_path}/contact")),
        ("pillar-design", "curl-intuitive", String::new()),
    ];
    let mut index = format!("\n{art}\n  {BOLD}{w}mintaka{RESET}{m} — {}{RESET}\n  {m}{}{RESET}\n\n", s.t("curl-tagline"), s.t("curl-sub"));
    // Columns fit the longest name and line of this language (24 and 26 in English).
    let name_w = pillar_rows.iter().map(|(name, _, _)| s.t(name).chars().count()).max().unwrap_or(0) + 2;
    let line_w = pillar_rows.iter().map(|(_, line, _)| s.t(line).chars().count()).max().unwrap_or(0) + 3;
    for (name, line, path) in &pillar_rows {
        let hint = if path.is_empty() { String::new() } else { format!("{b}curl {host}{path}{RESET}") };
        let _ = writeln!(index, "  {w}{:<name_w$}{m}{:<line_w$}{RESET}{hint}", s.t(name).to_lowercase(), s.t(line));
    }
    index.push_str(&footer);

    let mut pillars = format!("\n  {BOLD}{w}{}{RESET}\n\n", s.t("pillars-title"));
    for (name, star, color) in [
        ("pillar-efficiency", "star-rigel", &w),
        ("pillar-security", "star-saiph", &w),
        ("pillar-highload", "star-betelgeuse", &r),
        ("pillar-design", "star-meissa", &w),
    ] {
        let _ = writeln!(pillars, "  {color}●{RESET} {BOLD}{w}{}{RESET} {m}· {}{RESET}\n    {w}{}{RESET}\n", s.t(name), s.t(star), s.t(&format!("{name}-line")));
    }
    pillars.push_str(&footer);

    let telegram = if contact.telegram.is_empty() { String::new() } else { format!("@{}", contact.telegram) };
    let mut direct = String::new();
    for (label, value) in [("email", contact.email.as_str()), ("telegram", telegram.as_str()), ("phone", contact.phone.as_str())] {
        if !value.is_empty() {
            let _ = writeln!(direct, "  {m}{label:<9}{RESET}{b}{value}{RESET}");
        }
    }
    let contact = format!("\n  {BOLD}{w}{}{RESET}\n\n{direct}  {m}web      {RESET}{b}https://{host}{lang_path}/#contact{RESET}\n{footer}", s.t("ct-title"));
    Ok(Pages { index, pillars, contact })
}
