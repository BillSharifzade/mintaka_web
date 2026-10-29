//! The Lab (spec §S11): long-form posts written in Markdown (`content/{lang}/lab/*.md`) and
//! rendered at build time. Mathematics becomes MathML and code is highlighted here, so Lab pages
//! ship no JavaScript at all (spec §9.5). Markdown: pulldown-cmark (a pull parser, so code blocks
//! and formulas are intercepted as events); LaTeX → MathML Core: pulldown-latex.
//!
//! A post starts with TOML front matter between `+++` lines:
//!
//! ```toml
//! slug = "contraction-hierarchies"   # the URL: /{lang}/lab/{slug}/ — the same in every language
//! title = "Contraction Hierarchies on Dushanbe's roads"
//! pillar = "efficiency"              # efficiency | security | highload | design
//! date = 2026-10-29                  # first published
//! updated = 2026-10-29               # optional; defaults to `date`
//! summary = "…"                      # the lead, the meta description and the feed summary
//! repo = "crates/routing"            # the code to reproduce it: a path in the repository
//! ```
//!
//! Markdown extras, all resolved at build time:
//! * `$…$` and `$$…$$`: LaTeX → MathML, with the source as `alttext`. A paragraph that is only
//!   a `$$…$$` formula becomes a formula block.
//! * ```` ```rust include="crates/routing/src/search.rs" from="fn ch(" ````: the code is read from
//!   the repository — from the one line containing `from` to the closing brace at its
//!   indentation (or to the first line containing `to`) — and links to those lines on GitHub.
//!   Code in a post cannot drift from the code it describes: a missing anchor fails the build.
//! * `[text](repo:crates/server/src/tls.rs)`: a link into the repository; the path must exist.
//! * Tables: a right-aligned column (`---:`) holds numbers; a `Table: …` paragraph right after a
//!   table becomes its caption, and every table needs one (it names the scrollable region).
//!
//! Checks that fail the build: an `h1` or a skipped level in a post (the page has one `h1`, the
//! title), a duplicate or dangling `#id`, an inline `style` or a `<script>` (the CSP blocks the
//! first; Lab pages promise none of the second), a formula that does not parse.

use crate::config::SiteConfig;
use crate::highlight;
use crate::i18n::Strings;
use crate::util::escape_xml;
use anyhow::{Context, Result, anyhow, bail, ensure};
use pulldown_cmark::{Alignment, CodeBlockKind, CowStr, Event, Options, Parser, Tag, TagEnd};
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

/// Ids the page templates use around a post body; headings must not take them.
const RESERVED_IDS: &[&str] = &["main", "post-title", "repo-title", "lab-title", "promises-title"];

/// Longest code excerpt a post may include. Longer than this, link to the file instead.
const MAX_EXCERPT_LINES: usize = 60;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Pillar {
    Efficiency,
    Security,
    Highload,
    Design,
}

impl Pillar {
    pub fn id(self) -> &'static str {
        match self {
            Pillar::Efficiency => "efficiency",
            Pillar::Security => "security",
            Pillar::Highload => "highload",
            Pillar::Design => "design",
        }
    }

    /// The star that stands for the pillar on the page (Rigel, Saiph, Betelgeuse, Meissa).
    pub fn star(self) -> &'static str {
        match self {
            Pillar::Efficiency => "rigel",
            Pillar::Security => "saiph",
            Pillar::Highload => "betelgeuse",
            Pillar::Design => "meissa",
        }
    }

    /// The message id of its name (`pillar-highload`).
    pub fn label_key(self) -> String {
        format!("pillar-{}", self.id())
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FrontMatter {
    slug: String,
    title: String,
    pillar: Pillar,
    date: toml::value::Datetime,
    updated: Option<toml::value::Datetime>,
    summary: String,
    repo: String,
}

pub struct Post {
    pub slug: String,
    pub title: String,
    pub pillar: Pillar,
    /// First published, `YYYY-MM-DD`.
    pub date: String,
    /// Last changed in substance, `YYYY-MM-DD` (`date` unless the front matter says otherwise).
    pub updated: String,
    pub summary: String,
    /// The code behind the post, a path in the repository (`crates/routing`).
    pub repo: String,
    /// The same path as a URL on the repository host.
    pub repo_url: String,
    /// The body as the page shows it.
    pub html: String,
    /// The body for the Atom feed: plain code blocks, absolute links.
    pub feed_html: String,
    /// Words of prose, code excluded (reported by the build).
    pub words: usize,
}

impl Post {
    /// `/en/lab/contraction-hierarchies/`
    pub fn path(&self, lang: &str) -> String {
        format!("/{lang}/lab/{}/", self.slug)
    }
}

/// `/{lang}/lab/`
pub fn index_path(lang: &str) -> String {
    format!("/{lang}/lab/")
}

/// `/{lang}/lab/feed.xml`
pub fn feed_path(lang: &str) -> String {
    format!("/{lang}/lab/feed.xml")
}

/// Links into the public repository (`content/site.toml` → `[lab]`), checked against the working
/// tree at build time: a post can only link to files that exist where it was built.
pub struct Repo<'a> {
    root: &'a Path,
    url: &'a str,
    branch: &'a str,
}

impl<'a> Repo<'a> {
    pub fn new(root: &'a Path, cfg: &'a SiteConfig) -> Self {
        Repo { root, url: cfg.lab.repo.trim_end_matches('/'), branch: &cfg.lab.branch }
    }

    /// A file → `{repo}/blob/{branch}/{path}`, a directory → `{repo}/tree/{branch}/{path}`.
    pub fn link(&self, path: &str) -> Result<String> {
        let clean = self.check(path)?;
        let kind = if self.root.join(&clean).is_dir() { "tree" } else { "blob" };
        Ok(format!("{}/{kind}/{}/{clean}", self.url, self.branch))
    }

    /// The repository-relative path, if it names something that exists.
    fn check(&self, path: &str) -> Result<String> {
        let clean = path.trim_matches('/');
        ensure!(
            !clean.is_empty() && clean.split('/').all(|p| !p.is_empty() && p != "." && p != ".."),
            "repository path {path:?} must be plain and relative (`crates/routing/src/search.rs`)"
        );
        ensure!(self.root.join(clean).exists(), "repository path {path:?} does not exist");
        Ok(clean.to_owned())
    }

    fn read(&self, path: &str) -> Result<(String, String)> {
        let clean = self.check(path)?;
        let text = std::fs::read_to_string(self.root.join(&clean)).with_context(|| format!("reading {clean}"))?;
        Ok((clean, text))
    }
}

/// The post files of a language, in file-name order.
pub fn sources(root: &Path, lang: &str) -> Vec<PathBuf> {
    let dir = root.join("content").join(lang).join("lab");
    let mut files: Vec<PathBuf> = match std::fs::read_dir(&dir) {
        Ok(rd) => rd.filter_map(|e| e.ok().map(|e| e.path())).filter(|p| p.extension().is_some_and(|x| x == "md")).collect(),
        Err(_) => vec![],
    };
    files.sort();
    files
}

/// Every post of a language, newest first (same date: file-name order).
pub fn load(root: &Path, lang: &str, cfg: &SiteConfig, s: &Strings) -> Result<Vec<Post>> {
    let repo = Repo::new(root, cfg);
    let mut posts: Vec<Post> = vec![];
    for file in sources(root, lang) {
        let text = std::fs::read_to_string(&file).with_context(|| format!("reading {}", file.display()))?;
        let post = parse(&text, &repo, &cfg.origin, lang, s).with_context(|| format!("Lab post {}", file.display()))?;
        if posts.iter().any(|p| p.slug == post.slug) {
            bail!("Lab post {}: slug {:?} is used twice", file.display(), post.slug);
        }
        posts.push(post);
    }
    posts.sort_by(|a, b| b.date.cmp(&a.date));
    Ok(posts)
}

fn parse(text: &str, repo: &Repo, origin: &str, lang: &str, s: &Strings) -> Result<Post> {
    let text = text.replace("\r\n", "\n");
    let rest = text.strip_prefix("+++\n").context("a post starts with TOML front matter between `+++` lines")?;
    let end = rest.find("\n+++\n").context("the front matter is not closed with a `+++` line")?;
    let fm: FrontMatter = toml::from_str(&rest[..end]).context("front matter")?;
    let body = &rest[end + 5..];

    let slug_ok = !fm.slug.is_empty()
        && fm.slug.split('-').all(|part| !part.is_empty() && part.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit()));
    ensure!(slug_ok, "slug {:?}: lower-case letters, digits and single dashes only", fm.slug);
    ensure!(!fm.title.trim().is_empty(), "the title is empty");
    ensure!(!fm.summary.trim().is_empty(), "the summary is empty");
    let date = plain_date(&fm.date, "date")?;
    let updated = match &fm.updated {
        Some(d) => plain_date(d, "updated")?,
        None => date.clone(),
    };
    ensure!(updated >= date, "updated ({updated}) is before date ({date})");
    let repo_url = repo.link(&fm.repo).context("front matter `repo`")?;

    let path = format!("/{lang}/lab/{}/", fm.slug);
    let page = render(body, &Render { repo, s, feed: None })?;
    let feed = render(body, &Render { repo, s, feed: Some((origin, &path)) })?;
    Ok(Post {
        slug: fm.slug,
        title: fm.title.trim().to_owned(),
        pillar: fm.pillar,
        date,
        updated,
        summary: fm.summary.split_whitespace().collect::<Vec<_>>().join(" "),
        repo: fm.repo.trim_matches('/').to_owned(),
        repo_url,
        html: page.html,
        feed_html: feed.html,
        words: page.words,
    })
}

fn plain_date(d: &toml::value::Datetime, field: &str) -> Result<String> {
    match (&d.date, &d.time, &d.offset) {
        (Some(date), None, None) => Ok(format!("{:04}-{:02}-{:02}", date.year, date.month, date.day)),
        _ => bail!("`{field}` must be a plain date such as 2026-10-29"),
    }
}

/// How a body is rendered: for the page, or for the feed (`(origin, the post's path)`), where
/// links become absolute and code is left unhighlighted.
struct Render<'a> {
    repo: &'a Repo<'a>,
    s: &'a Strings,
    feed: Option<(&'a str, &'a str)>,
}

struct Body {
    html: String,
    words: usize,
}

fn render(markdown: &str, r: &Render) -> Result<Body> {
    let options = Options::ENABLE_TABLES | Options::ENABLE_MATH | Options::ENABLE_SMART_PUNCTUATION | Options::ENABLE_HEADING_ATTRIBUTES;
    let events: Vec<Event> = Parser::new_ext(markdown, options).collect();
    let mut out: Vec<Event> = Vec::with_capacity(events.len());
    let mut ids: BTreeSet<String> = BTreeSet::new();
    let mut fragments: Vec<String> = vec![];
    let mut last_level = 1;
    let mut words = 0;
    let mut tables = 0;
    let mut table = TableState::default();
    let mut skip_from = usize::MAX;
    let mut skip_to = 0;

    let mut i = 0;
    while i < events.len() {
        if (skip_from..skip_to).contains(&i) {
            i = skip_to;
            continue;
        }
        let event = &events[i];
        match event {
            Event::Start(Tag::Heading { level, id, .. }) => {
                let n = *level as usize;
                ensure!(n >= 2, "use `##` and deeper for headings: the page's one h1 is the post title");
                ensure!(n <= last_level + 1, "headings skip a level (h{last_level} → h{n})");
                last_level = n;
                let end = matching_end(&events, i);
                let slug = match id {
                    Some(id) => id.to_string(),
                    None => slugify(&plain_text(&events[i + 1..end])),
                };
                ensure!(!slug.is_empty(), "a heading has no text to make an id from");
                ensure!(!RESERVED_IDS.contains(&slug.as_str()), "heading id {slug:?} is used by the page template");
                ensure!(ids.insert(slug.clone()), "two headings would share the id {slug:?} (set one with `{{#id}}`)");
                out.push(Event::Html(format!("<h{n} id=\"{slug}\">").into()));
            }
            Event::End(TagEnd::Heading(level)) => out.push(Event::Html(format!("</{level}>\n").into())),

            Event::Start(Tag::CodeBlock(kind)) => {
                let end = matching_end(&events, i);
                let source: String = events[i + 1..end].iter().map(|e| if let Event::Text(t) = e { t.as_ref() } else { "" }).collect();
                let info = match kind {
                    CodeBlockKind::Fenced(info) => info.as_ref(),
                    CodeBlockKind::Indented => "",
                };
                out.push(Event::Html(code_block(info, &source, r)?.into()));
                i = end + 1;
                continue;
            }

            // A paragraph that is only a display formula becomes a formula block.
            Event::Start(Tag::Paragraph)
                if matches!(events.get(i + 1), Some(Event::DisplayMath(_))) && matches!(events.get(i + 2), Some(Event::End(TagEnd::Paragraph))) =>
            {
                let Some(Event::DisplayMath(tex)) = events.get(i + 1) else { unreachable!() };
                let math = mathml(tex, true)?;
                let html = match r.feed {
                    Some(_) => format!("<p>{math}</p>\n"),
                    None => format!("<div class=\"post__formula\" tabindex=\"0\" role=\"group\" aria-label=\"{}\">{math}</div>\n", escape_xml(&r.s.t("lab-formula"))),
                };
                out.push(Event::Html(html.into()));
                i += 3;
                continue;
            }

            Event::Start(Tag::Table(aligns)) => {
                let end = matching_end(&events, i);
                // The caption: a paragraph starting with "Table: " right after the table.
                let caption_start = end + 1;
                let is_caption = matches!(events.get(caption_start), Some(Event::Start(Tag::Paragraph)))
                    && matches!(events.get(caption_start + 1), Some(Event::Text(t)) if t.starts_with("Table: "));
                ensure!(is_caption, "every table needs a caption: a paragraph starting with `Table: ` right after it");
                let caption_end = matching_end(&events, caption_start);
                tables += 1;
                let id = format!("table-{tables}");
                ensure!(ids.insert(id.clone()), "heading id {id:?} collides with a table caption");
                out.push(Event::Html(match r.feed {
                    Some(_) => "<table>".to_owned().into(),
                    None => format!("<div class=\"post__table\" tabindex=\"0\" role=\"group\" aria-labelledby=\"{id}\"><table>").into(),
                }));
                out.push(Event::Html(format!("<caption id=\"{id}\">").into()));
                for (k, e) in events[caption_start + 1..caption_end].iter().enumerate() {
                    let e = match (k, e) {
                        (0, Event::Text(t)) => Event::Text(t.trim_start_matches("Table: ").to_owned().into()),
                        _ => e.clone(),
                    };
                    words += word_count(&e);
                    out.push(inline(e, r, &mut fragments)?);
                }
                out.push(Event::Html("</caption>".into()));
                table = TableState { aligns: aligns.clone(), ..Default::default() };
                (skip_from, skip_to) = (caption_start, caption_end + 1);
            }
            Event::End(TagEnd::Table) => out.push(Event::Html(if r.feed.is_some() { "</tbody></table>\n" } else { "</tbody></table></div>\n" }.into())),
            Event::Start(Tag::TableHead) => {
                table.head = true;
                table.col = 0;
                out.push(Event::Html("<thead><tr>".into()));
            }
            Event::End(TagEnd::TableHead) => {
                table.head = false;
                out.push(Event::Html("</tr></thead><tbody>".into()));
            }
            Event::Start(Tag::TableRow) => {
                table.col = 0;
                out.push(Event::Html("<tr>".into()));
            }
            Event::End(TagEnd::TableRow) => out.push(Event::Html("</tr>".into())),
            Event::Start(Tag::TableCell) => {
                let num = if table.aligns.get(table.col) == Some(&Alignment::Right) { " class=\"num\"" } else { "" };
                let tag = if table.head {
                    format!("<th scope=\"col\"{num}>")
                } else if table.col == 0 {
                    format!("<th scope=\"row\"{num}>")
                } else {
                    format!("<td{num}>")
                };
                out.push(Event::Html(tag.into()));
            }
            Event::End(TagEnd::TableCell) => {
                out.push(Event::Html(if table.head || table.col == 0 { "</th>" } else { "</td>" }.into()));
                table.col += 1;
            }

            Event::Start(Tag::Image { .. }) => bail!("images are not supported in Lab posts yet"),
            Event::Html(html) | Event::InlineHtml(html) => {
                let lower = html.to_ascii_lowercase();
                ensure!(!lower.contains("style="), "inline `style` in raw HTML (the CSP blocks it): {html}");
                ensure!(!lower.contains("<script"), "a <script> in a Lab post (Lab pages ship no JavaScript)");
                out.push(event.clone());
            }
            e => {
                words += word_count(e);
                out.push(inline(e.clone(), r, &mut fragments)?);
            }
        }
        i += 1;
    }

    for f in &fragments {
        ensure!(ids.contains(f), "link to #{f}, but no heading or table has that id");
    }
    let mut html = String::with_capacity(markdown.len() * 2);
    pulldown_cmark::html::push_html(&mut html, out.into_iter());
    Ok(Body { html, words })
}

#[derive(Default)]
struct TableState {
    aligns: Vec<Alignment>,
    head: bool,
    col: usize,
}

/// Inline events: formulas → MathML, `repo:` links → the repository, feed links → absolute.
fn inline<'a>(e: Event<'a>, r: &Render, fragments: &mut Vec<String>) -> Result<Event<'a>> {
    Ok(match e {
        Event::InlineMath(tex) => Event::InlineHtml(mathml(&tex, false)?.into()),
        Event::DisplayMath(tex) => Event::InlineHtml(mathml(&tex, true)?.into()),
        Event::Start(Tag::Link { link_type, dest_url, title, id }) => {
            let dest: CowStr = if let Some(path) = dest_url.strip_prefix("repo:") {
                r.repo.link(path)?.into()
            } else if let Some(fragment) = dest_url.strip_prefix('#') {
                fragments.push(fragment.to_owned());
                match r.feed {
                    Some((origin, page)) => format!("{origin}{page}{dest_url}").into(),
                    None => dest_url,
                }
            } else if dest_url.starts_with('/') {
                match r.feed {
                    Some((origin, _)) => format!("{origin}{dest_url}").into(),
                    None => dest_url,
                }
            } else {
                ensure!(dest_url.starts_with("https://"), "link {dest_url:?}: use https://, a site path, #id or repo:path");
                dest_url
            };
            Event::Start(Tag::Link { link_type, dest_url: dest, title, id })
        }
        other => other,
    })
}

/// The index of the `End` event matching the `Start` at `i`.
fn matching_end(events: &[Event], i: usize) -> usize {
    let mut depth = 0usize;
    for (j, e) in events.iter().enumerate().skip(i) {
        match e {
            Event::Start(_) => depth += 1,
            Event::End(_) => {
                depth -= 1;
                if depth == 0 {
                    return j;
                }
            }
            _ => {}
        }
    }
    events.len().saturating_sub(1)
}

fn plain_text(events: &[Event]) -> String {
    events
        .iter()
        .map(|e| match e {
            Event::Text(t) | Event::Code(t) | Event::InlineMath(t) => t.as_ref(),
            Event::SoftBreak | Event::HardBreak => " ",
            _ => "",
        })
        .collect()
}

fn word_count(e: &Event) -> usize {
    match e {
        Event::Text(t) | Event::Code(t) => t.split_whitespace().count(),
        _ => 0,
    }
}

/// "The bidirectional, upward query" → `the-bidirectional-upward-query`.
fn slugify(text: &str) -> String {
    let mut out = String::new();
    for c in text.chars().flat_map(char::to_lowercase) {
        if c.is_alphanumeric() {
            out.push(c);
        } else if !out.is_empty() && !out.ends_with('-') {
            out.push('-');
        }
    }
    out.trim_end_matches('-').to_owned()
}

/// A fenced code block: `lang` then `key="value"` attributes (`include`, `from`, `to`, `title`).
fn code_block(info: &str, inline_source: &str, r: &Render) -> Result<String> {
    let (lang, attrs) = parse_info(info)?;
    let canonical = match lang.as_str() {
        "rust" | "rs" => "rust",
        "sh" | "shell" | "bash" | "console" => "sh",
        "toml" => "toml",
        "ftl" | "fluent" => "ftl",
        "text" | "txt" | "output" => "text",
        // A fence without a language is usually a fence that went wrong (an info string with a
        // backtick does not open one), so the language is required.
        "" => bail!("a code block without a language: write ```rust, ```sh, ```toml, ```ftl or ```text"),
        other => bail!("code language {other:?}: highlight.rs knows rust, sh, toml, ftl and text"),
    };
    for key in attrs.keys() {
        ensure!(["include", "from", "to", "title"].contains(&key.as_str()), "unknown code block attribute {key:?}");
    }
    let s = r.s;
    let lang_name = s.t(&format!("lab-lang-{canonical}"));
    let (source, file) = match attrs.get("include") {
        Some(path) => {
            ensure!(inline_source.trim().is_empty(), "a code block with `include` must be empty");
            let from = attrs.get("from").context("`include` needs `from`: text on the excerpt's first line")?;
            let (clean, text) = r.repo.read(path)?;
            let (source, a, b) = excerpt(&text, from, attrs.get("to").map(String::as_str)).with_context(|| format!("including {clean}"))?;
            let lines = s.ta("lab-code-lines", &[("from", a.to_string().into()), ("to", b.to_string().into())]);
            let url = format!("{}#L{a}-L{b}", r.repo.link(&clean)?);
            (source, Some((clean, lines, url)))
        }
        None => (inline_source.trim_end_matches('\n').to_owned(), None),
    };
    ensure!(!source.trim().is_empty(), "an empty code block");

    if r.feed.is_some() {
        let mut html = String::new();
        if let Some((clean, lines, url)) = &file {
            let _ = writeln!(html, "<p><a href=\"{}\">{}, {}</a></p>", escape_xml(url), escape_xml(clean), escape_xml(lines));
        }
        let _ = writeln!(html, "<pre><code>{}</code></pre>", escape_xml(&source));
        return Ok(html);
    }

    let code = highlight::code(canonical, &source).unwrap_or_else(|| escape_xml(&source));
    let (bar, label) = match (&file, attrs.get("title")) {
        (Some((clean, lines, url)), _) => (
            format!(
                "<a class=\"post__code-file mono\" href=\"{}\">{}</a> <span class=\"post__code-lines\">{}</span>",
                escape_xml(url),
                escape_xml(clean),
                escape_xml(lines)
            ),
            s.ta("lab-code-titled", &[("lang", lang_name.as_str().into()), ("title", format!("{clean}, {lines}").into())]),
        ),
        (None, Some(title)) => (
            format!("<span class=\"post__code-title\">{}</span>", escape_xml(title)),
            s.ta("lab-code-titled", &[("lang", lang_name.as_str().into()), ("title", title.as_str().into())]),
        ),
        (None, None) => (String::new(), s.ta("lab-code", &[("lang", lang_name.as_str().into())])),
    };
    Ok(format!(
        "<figure class=\"post__code\"><figcaption class=\"post__code-bar\"><span class=\"post__code-lang\">{}</span>{}{bar}</figcaption><pre class=\"post__pre\" tabindex=\"0\" role=\"group\" aria-label=\"{}\"><code>{code}</code></pre></figure>\n",
        escape_xml(&lang_name),
        if bar.is_empty() { "" } else { " " },
        escape_xml(&label),
    ))
}

/// `rust include="a b.rs" from="fn x("` → (`rust`, {include: "a b.rs", from: "fn x("}).
fn parse_info(info: &str) -> Result<(String, BTreeMap<String, String>)> {
    let info = info.trim();
    let (lang, mut rest) = info.split_once(char::is_whitespace).unwrap_or((info, ""));
    let mut attrs = BTreeMap::new();
    loop {
        rest = rest.trim_start();
        if rest.is_empty() {
            break;
        }
        let (key, after) = rest.split_once("=\"").with_context(|| format!("code block attributes are key=\"value\": {rest:?}"))?;
        let (value, after) = after.split_once('"').with_context(|| format!("unclosed quote in {rest:?}"))?;
        ensure!(attrs.insert(key.trim().to_owned(), value.to_owned()).is_none(), "attribute {key:?} given twice");
        rest = after;
    }
    Ok((lang.to_ascii_lowercase(), attrs))
}

/// The lines of `text` from the one line containing `from` to the first line (at or after it)
/// containing `to` — or, without `to`, to the closing `}` at `from`'s indentation. Common
/// indentation is removed. Returns the excerpt and its 1-based first and last line.
fn excerpt(text: &str, from: &str, to: Option<&str>) -> Result<(String, usize, usize)> {
    let lines: Vec<&str> = text.lines().collect();
    let starts: Vec<usize> = (0..lines.len()).filter(|&i| lines[i].contains(from)).collect();
    let start = match starts.as_slice() {
        [one] => *one,
        [] => bail!("no line contains {from:?}"),
        many => bail!("{} lines contain {from:?} (lines {:?}): make `from` unique", many.len(), many.iter().map(|i| i + 1).collect::<Vec<_>>()),
    };
    let end = match to {
        Some(to) => (start..lines.len()).find(|&i| lines[i].contains(to)).with_context(|| format!("no line after {from:?} contains {to:?}"))?,
        None => {
            let indent = &lines[start][..lines[start].len() - lines[start].trim_start().len()];
            (start + 1..lines.len())
                .find(|&i| lines[i].strip_prefix(indent).is_some_and(|rest| rest.starts_with('}')))
                .with_context(|| format!("no closing brace after {from:?} at its indentation (give `to`)"))?
        }
    };
    let count = end - start + 1;
    ensure!(count <= MAX_EXCERPT_LINES, "the excerpt from {from:?} is {count} lines (at most {MAX_EXCERPT_LINES}): narrow it, or link to the file");
    let slice = &lines[start..=end];
    let indent = slice.iter().filter(|l| !l.trim().is_empty()).map(|l| l.len() - l.trim_start().len()).min().unwrap_or(0);
    let body: Vec<&str> = slice.iter().map(|l| if l.trim().is_empty() { "" } else { &l[indent..] }).collect();
    Ok((body.join("\n"), start + 1, end + 1))
}

/// LaTeX → MathML Core, made safe to inline: no inline styles (the CSP would block them), text
/// escaped, multi-letter identifiers upright, and the source as `alttext`.
fn mathml(tex: &str, display: bool) -> Result<String> {
    use pulldown_latex::config::DisplayMode;
    let storage = pulldown_latex::Storage::new();
    let parser = pulldown_latex::Parser::new(tex, &storage);
    let config = pulldown_latex::RenderConfig { display_mode: if display { DisplayMode::Block } else { DisplayMode::Inline }, ..Default::default() };
    let mut raw = String::new();
    pulldown_latex::push_mathml(&mut raw, parser, config).map_err(|e| anyhow!("formula {tex:?}: {e}"))?;
    if let Some(at) = raw.find("<merror") {
        let message = raw[at..].split("<mtext>").nth(1).and_then(|m| m.lines().next()).unwrap_or("");
        bail!("formula {tex:?} does not parse: {message}");
    }
    ensure!(!raw.contains("style=\""), "formula {tex:?} needs an inline style, which the CSP blocks (no \\color, negative spaces or row spacing)");
    let alt = tex.split_whitespace().collect::<Vec<_>>().join(" ");
    let html = upright_names(&escape_math_text(&raw));
    Ok(html.replacen("<math ", &format!("<math alttext=\"{}\" ", escape_xml(&alt)), 1))
}

/// pulldown-latex writes `<` and `>` operators unescaped (`<mo><</mo>`); escape every `<` that
/// does not open a tag and every `>` outside one.
fn escape_math_text(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len() + 16);
    let mut in_tag = false;
    let mut chars = raw.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '<' if !in_tag && chars.peek().is_some_and(|n| n.is_ascii_alphabetic() || *n == '/') => {
                in_tag = true;
                out.push('<');
            }
            '<' if !in_tag => out.push_str("&lt;"),
            '>' if in_tag => {
                in_tag = false;
                out.push('>');
            }
            '>' => out.push_str("&gt;"),
            c => out.push(c),
        }
    }
    out
}

/// MathML Core sets a one-letter `<mi>` in mathematical italic by mapping it to the Unicode math
/// alphabet (U+1D434 …), which none of our fonts have. The Lab stylesheet turns that mapping off
/// and slants `mi` itself; so names (`min`) and symbols (`′`) are marked upright here.
fn upright_names(html: &str) -> String {
    let mut out = String::with_capacity(html.len() + 64);
    let mut rest = html;
    while let Some(at) = rest.find("<mi>") {
        out.push_str(&rest[..at]);
        let after = &rest[at + 4..];
        let end = after.find("</mi>").unwrap_or(after.len());
        let content = &after[..end];
        // An entity (`&lt;`) starts with `&`, which is not a letter: such content stays upright.
        let mut chars = content.chars();
        let one_letter = matches!((chars.next(), chars.next()), (Some(c), None) if c.is_alphabetic());
        out.push_str(if one_letter { "<mi>" } else { "<mi mathvariant=\"normal\">" });
        rest = after;
    }
    out.push_str(rest);
    out
}

/// Lab URLs for the sitemap: each in the languages that have it, dated by its own content (the
/// index by its newest post).
pub fn sitemap_urls(posts: &BTreeMap<String, Vec<Post>>) -> Vec<crate::well_known::SitemapUrl> {
    let mut urls: Vec<crate::well_known::SitemapUrl> = vec![];
    for (lang, posts) in posts {
        let newest = posts.iter().map(|p| p.updated.clone()).max();
        let pages = newest.map(|d| ("/lab/".to_owned(), d)).into_iter().chain(posts.iter().map(|p| (format!("/lab/{}/", p.slug), p.updated.clone())));
        for (path, lastmod) in pages {
            match urls.iter_mut().find(|u| u.path == path) {
                Some(u) => {
                    u.langs.push(lang.clone());
                    u.lastmod = u.lastmod.clone().max(lastmod);
                }
                None => urls.push(crate::well_known::SitemapUrl { path, langs: vec![lang.clone()], lastmod }),
            }
        }
    }
    urls
}

/// The Atom feed (RFC 4287) of a language's posts. Every value is a function of the posts and the
/// configured origin — never of the build time — so the feed only changes when a post does.
pub fn atom(cfg: &SiteConfig, lang: &str, s: &Strings, posts: &[Post]) -> String {
    let origin = &cfg.origin;
    let rfc3339 = |date: &str| format!("{date}T00:00:00Z");
    let updated = posts.iter().map(|p| p.updated.as_str()).max().unwrap_or("1970-01-01");
    let x = escape_xml;
    let mut out = String::from("<?xml version=\"1.0\" encoding=\"utf-8\"?>\n");
    let _ = writeln!(out, "<feed xmlns=\"http://www.w3.org/2005/Atom\" xml:lang=\"{}\">", x(lang));
    let _ = writeln!(out, "  <id>{}</id>", x(&format!("{origin}{}", feed_path(lang))));
    let _ = writeln!(out, "  <title>{}</title>", x(&s.t("lab-feed-title")));
    let _ = writeln!(out, "  <subtitle>{}</subtitle>", x(&s.t("lab-lead")));
    let _ = writeln!(out, "  <link rel=\"self\" type=\"application/atom+xml\" href=\"{}\"/>", x(&format!("{origin}{}", feed_path(lang))));
    let _ = writeln!(out, "  <link rel=\"alternate\" type=\"text/html\" href=\"{}\"/>", x(&format!("{origin}{}", index_path(lang))));
    let _ = writeln!(out, "  <updated>{}</updated>", rfc3339(updated));
    let _ = writeln!(out, "  <author><name>{}</name><uri>{}</uri></author>", x(&cfg.company.name), x(&format!("{origin}/{lang}/")));
    let _ = writeln!(out, "  <icon>{}</icon>", x(&format!("{origin}/favicon.svg")));
    for p in posts {
        let url = format!("{origin}{}", p.path(lang));
        out.push_str("  <entry>\n");
        let _ = writeln!(out, "    <id>{}</id>", x(&url));
        let _ = writeln!(out, "    <title>{}</title>", x(&p.title));
        let _ = writeln!(out, "    <link rel=\"alternate\" type=\"text/html\" href=\"{}\"/>", x(&url));
        let _ = writeln!(out, "    <published>{}</published>", rfc3339(&p.date));
        let _ = writeln!(out, "    <updated>{}</updated>", rfc3339(&p.updated));
        let _ = writeln!(out, "    <category term=\"{}\" label=\"{}\"/>", p.pillar.id(), x(&s.t(&p.pillar.label_key())));
        let _ = writeln!(out, "    <summary>{}</summary>", x(&p.summary));
        let _ = writeln!(out, "    <content type=\"html\">{}</content>", x(&p.feed_html));
        out.push_str("  </entry>\n");
    }
    out.push_str("</feed>\n");
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn root() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
    }

    /// A scratch repository with one source file, and the real English strings and site config.
    struct Fixture {
        dir: PathBuf,
        cfg: SiteConfig,
        s: Strings,
    }

    impl Fixture {
        fn new(name: &str) -> Fixture {
            let dir = std::env::temp_dir().join(format!("mintaka-lab-{name}-{}", std::process::id()));
            std::fs::create_dir_all(dir.join("src")).unwrap();
            std::fs::write(
                dir.join("src/lib.rs"),
                "//! A crate.\n\npub fn pick(x: u32) -> u32 {\n    if x > 1 {\n        x\n    } else {\n        1\n    }\n}\n\npub fn other() {}\n",
            )
            .unwrap();
            Fixture { dir, cfg: SiteConfig::load(&root()).unwrap(), s: Strings::load(&root(), "en").unwrap() }
        }

        fn render(&self, md: &str) -> Result<String> {
            let repo = Repo { root: &self.dir, url: "https://github.com/o/r", branch: "main" };
            render(md, &Render { repo: &repo, s: &self.s, feed: None }).map(|b| b.html)
        }

        fn post(&self, md: &str) -> Result<Post> {
            let repo = Repo { root: &self.dir, url: "https://github.com/o/r", branch: "main" };
            parse(md, &repo, "https://mintaka.tj", "en", &self.s)
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.dir);
        }
    }

    const FRONT: &str = "+++\nslug = \"a-post\"\ntitle = \"A post\"\npillar = \"highload\"\ndate = 2026-10-29\nsummary = \"What it is & why.\"\nrepo = \"src\"\n+++\n";

    #[test]
    fn front_matter_is_parsed_and_checked() {
        let f = Fixture::new("front");
        let p = f.post(&format!("{FRONT}\n## Problem\n\nText.\n")).unwrap();
        assert_eq!((p.slug.as_str(), p.date.as_str(), p.updated.as_str()), ("a-post", "2026-10-29", "2026-10-29"));
        assert_eq!(p.pillar, Pillar::Highload);
        assert_eq!(p.repo_url, "https://github.com/o/r/tree/main/src");
        assert_eq!(p.path("en"), "/en/lab/a-post/");
        for (bad, why) in [
            ("slug = \"a-post\"", "slug = \"A_Post\""),
            ("pillar = \"highload\"", "pillar = \"speed\""),
            ("date = 2026-10-29", "date = \"yesterday\""),
            ("repo = \"src\"", "repo = \"../etc\""),
            ("repo = \"src\"", "repo = \"missing/dir\""),
            ("summary", "author = \"Someone\"\nsummary"),
        ] {
            let md = FRONT.replacen(bad, why, 1);
            assert!(f.post(&format!("{md}\nText.\n")).is_err(), "accepted {why:?}");
        }
        assert!(f.post("## No front matter\n").is_err());
    }

    #[test]
    fn headings_get_ids_and_keep_their_order() {
        let f = Fixture::new("headings");
        let html = f.render("## The problem\n\nSee [the method](#method-in-short).\n\n### Method, in short\n\n## Benchmark {#bench}\n").unwrap();
        assert!(html.contains(r#"<h2 id="the-problem">The problem</h2>"#), "{html}");
        assert!(html.contains(r##"<a href="#method-in-short">"##), "{html}");
        assert!(html.contains(r#"<h3 id="method-in-short">Method, in short</h3>"#), "{html}");
        assert!(html.contains(r#"<h2 id="bench">Benchmark</h2>"#), "{html}");
        assert!(f.render("# A second h1\n").is_err(), "an h1 in the body");
        assert!(f.render("## Two\n\n#### Four\n").is_err(), "a skipped level");
        assert!(f.render("## Same\n\n## Same\n").is_err(), "a duplicate id");
        assert!(f.render("[gone](#nowhere)\n").is_err(), "a dangling fragment");
        assert!(f.render("## Post title {#post-title}\n").is_err(), "a template id");
    }

    #[test]
    fn code_is_included_from_the_repository_and_highlighted() {
        let f = Fixture::new("code");
        let html = f.render("```rust include=\"src/lib.rs\" from=\"pub fn pick\"\n```\n").unwrap();
        // Lines 3–9: the function up to its closing brace, not the `} else {` inside it.
        assert!(html.contains(r#"href="https://github.com/o/r/blob/main/src/lib.rs#L3-L9""#), "{html}");
        assert!(html.contains("lines 3–9"), "{html}");
        assert!(html.contains(r#"<span class="tok-k">pub</span> <span class="tok-k">fn</span> pick"#), "{html}");
        assert!(!html.contains("other"), "{html}");
        assert!(html.contains(r#"<pre class="post__pre" tabindex="0" role="group" aria-label="Rust code: src/lib.rs, lines 3–9">"#), "{html}");
        let one = f.render("```rust include=\"src/lib.rs\" from=\"pub fn other\" to=\"pub fn other\"\n```\n").unwrap();
        assert!(one.contains("#L11-L11"), "{one}");
        let shell = f.render("```sh title=\"Run it\"\ncargo test -p site\n```\n").unwrap();
        assert!(shell.contains(r#"aria-label="Shell code: Run it""#), "{shell}");
        assert!(shell.contains(r#"<span class="tok-k">cargo</span>"#), "{shell}");
        assert!(f.render("```rust include=\"src/lib.rs\" from=\"fn\"\n```\n").is_err(), "an ambiguous anchor");
        assert!(f.render("```rust include=\"src/lib.rs\" from=\"fn gone\"\n```\n").is_err(), "a missing anchor");
        assert!(f.render("```rust include=\"src/nope.rs\" from=\"x\"\n```\n").is_err(), "a missing file");
        assert!(f.render("```cobol\nMOVE A TO B\n```\n").is_err(), "an unknown language");
        // A backtick in the info string means no fence opened; the stray fence after it has no language.
        assert!(f.render("```rust include=\"src/lib.rs\" from=\"`pick`\"\n```\n\nText.\n").is_err(), "a fence that did not open");
    }

    #[test]
    fn repo_links_must_exist() {
        let f = Fixture::new("links");
        let html = f.render("The [picker](repo:src/lib.rs) and [the crate](repo:src/).\n").unwrap();
        assert!(html.contains(r#"<a href="https://github.com/o/r/blob/main/src/lib.rs">picker</a>"#), "{html}");
        assert!(html.contains(r#"<a href="https://github.com/o/r/tree/main/src">the crate</a>"#), "{html}");
        assert!(f.render("[x](repo:src/gone.rs)\n").is_err());
        assert!(f.render("[x](http://example.org/)\n").is_err(), "plain http");
    }

    #[test]
    fn tables_have_captions_header_scopes_and_number_cells() {
        let f = Fixture::new("tables");
        let html = f.render("| Rate | p50 |\n|---|---:|\n| 20 000/s | 4 µs |\n\nTable: Server time, `app`.\n").unwrap();
        assert!(html.contains(r#"<div class="post__table" tabindex="0" role="group" aria-labelledby="table-1"><table><caption id="table-1">Server time, <code>app</code>.</caption>"#), "{html}");
        assert!(html.contains(r#"<thead><tr><th scope="col">Rate</th><th scope="col" class="num">p50</th></tr></thead>"#), "{html}");
        assert!(html.contains(r#"<tr><th scope="row">20 000/s</th><td class="num">4 µs</td></tr>"#), "{html}");
        assert!(!html.contains("style="), "{html}");
        assert!(!html.contains("<p>Table:"), "{html}");
        assert!(f.render("| a |\n|---|\n| 1 |\n").is_err(), "a table without a caption");
    }

    #[test]
    fn formulas_become_mathml_without_javascript() {
        let f = Fixture::new("math");
        let html = f.render("Inline $a < b$ and\n\n$$\nd(s, t) = \\min_{v} \\left( d_{\\uparrow}(s, v) + d_{\\downarrow}(v, t) \\right)\n$$\n").unwrap();
        assert!(html.contains(r#"<math alttext="a &lt; b" display="inline"><mi>a</mi><mo>&lt;</mo><mi>b</mi></math>"#), "{html}");
        assert!(html.contains(r#"<div class="post__formula" tabindex="0" role="group" aria-label="Formula"><math alttext="d(s, t) = \min_{v} \left( d_{\uparrow}(s, v) + d_{\downarrow}(v, t) \right)" display="block">"#), "{html}");
        assert!(html.contains(r#"<mi mathvariant="normal">min</mi>"#), "{html}");
        assert!(html.contains("<mi>d</mi>"), "{html}");
        // The formula block replaces the paragraph; MathML is well-formed XML.
        assert!(!html.contains("<p><div"), "{html}");
        for m in html.match_indices("<math ").map(|(i, _)| &html[i..i + html[i..].find("</math>").unwrap() + 7]) {
            roxmltree::Document::parse(&m.replace("<math ", "<math xmlns=\"http://www.w3.org/1998/Math/MathML\" ").replace("&nbsp;", "&#160;")).unwrap();
        }
        assert!(f.render("$\\notacommand$\n").is_err(), "a TeX error");
        assert!(f.render("$\\color{red} x$\n").is_err(), "an inline style");
    }

    #[test]
    fn raw_html_may_not_style_or_script() {
        let f = Fixture::new("raw");
        assert!(f.render("A <span lang=\"tg\">минтақа</span> here.\n").unwrap().contains(r#"<span lang="tg">минтақа</span>"#));
        assert!(f.render("A <span style=\"color: red\">x</span>.\n").is_err());
        assert!(f.render("<script>alert(1)</script>\n").is_err());
    }

    #[test]
    fn the_feed_is_well_formed_absolute_and_stable() {
        let f = Fixture::new("feed");
        let md = format!("{FRONT}\n## Problem\n\nSee [the home page](/en/) and [the problem](#problem): $x \\le y$.\n\n```rust include=\"src/lib.rs\" from=\"pub fn pick\"\n```\n");
        let post = f.post(&md).unwrap();
        let feed = atom(&f.cfg, "en", &f.s, std::slice::from_ref(&post));
        assert_eq!(feed, atom(&f.cfg, "en", &f.s, &[f.post(&md).unwrap()]), "the same input gives the same bytes");
        let doc = roxmltree::Document::parse(&feed).unwrap();
        let root = doc.root_element();
        assert_eq!(root.tag_name().namespace(), Some("http://www.w3.org/2005/Atom"));
        let child = |n: roxmltree::Node<'_, '_>, name: &str| n.children().find(|c| c.has_tag_name(name)).and_then(|c| c.text()).unwrap_or("").to_owned();
        let origin = &f.cfg.origin;
        assert_eq!(child(root, "id"), format!("{origin}/en/lab/feed.xml"));
        assert_eq!(child(root, "updated"), "2026-10-29T00:00:00Z");
        let entry = root.children().find(|c| c.has_tag_name("entry")).unwrap();
        assert_eq!(child(entry, "id"), format!("{origin}/en/lab/a-post/"));
        for field in ["published", "updated"] {
            let v = child(entry, field);
            assert!(v.len() == 20 && v.ends_with('Z') && v.as_bytes()[10] == b'T', "{field} is not RFC 3339: {v}");
        }
        assert_eq!(child(entry, "summary"), "What it is & why.");
        let content = child(entry, "content");
        assert!(content.contains(&format!("href=\"{origin}/en/\"")), "{content}");
        assert!(content.contains(&format!("href=\"{origin}/en/lab/a-post/#problem\"")), "{content}");
        assert!(content.contains("<pre><code>pub fn pick"), "plain code in the feed: {content}");
        assert!(content.contains("<math alttext=\"x \\le y\""), "{content}");
        for link in root.descendants().filter(|n| n.has_tag_name("link")) {
            assert!(link.attribute("href").unwrap().starts_with(origin.as_str()));
        }
    }

    #[test]
    fn the_launch_posts_build() {
        let root = root();
        let cfg = SiteConfig::load(&root).unwrap();
        let s = Strings::load(&root, "en").unwrap();
        let posts = load(&root, "en", &cfg, &s).unwrap();
        s.finish().unwrap();
        for p in &posts {
            assert!(!p.html.contains("<h1"), "{}", p.slug);
            assert!((800..=1800).contains(&p.words), "{}: {} words", p.slug, p.words);
        }
    }
}
