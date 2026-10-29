//! Strings. Page copy: `content/{lang}/page.ftl`, rendered at build time with Fluent.
//! Island copy: `content/{lang}/islands/*.ftl`, exported as a flat JSON map for the browser.
//!
//! A missing message is recorded, rendered as a visible `⟦id⟧` marker, and fails the build at the
//! end — in every language that is enabled.

use anyhow::{Context, Result, bail};
use fluent_bundle::{FluentArgs, FluentBundle, FluentResource, FluentValue};
use fluent_syntax::ast;
use maud::{Markup, PreEscaped};
use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use unic_langid::LanguageIdentifier;

pub struct Strings {
    pub lang: String,
    bundle: FluentBundle<FluentResource>,
    missing: RefCell<BTreeSet<String>>,
    errors: RefCell<Vec<String>>,
}

impl Strings {
    pub fn load(root: &Path, lang: &str) -> Result<Self> {
        let langid: LanguageIdentifier = lang.parse().with_context(|| format!("language id {lang:?}"))?;
        let mut bundle = FluentBundle::new(vec![langid]);
        // Everything here is left-to-right; isolation marks would only add bytes.
        bundle.set_use_isolating(false);
        let path = root.join("content").join(lang).join("page.ftl");
        let source = std::fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
        let resource = FluentResource::try_new(source)
            .map_err(|(_, errs)| anyhow::anyhow!("{}: {} Fluent syntax error(s): {:?}", path.display(), errs.len(), errs))?;
        bundle
            .add_resource(resource)
            .map_err(|errs| anyhow::anyhow!("{}: duplicate messages: {errs:?}", path.display()))?;
        Ok(Self { lang: lang.to_owned(), bundle, missing: Default::default(), errors: Default::default() })
    }

    /// A plain-text message.
    pub fn t(&self, id: &str) -> String {
        self.format(id, None)
    }

    /// A message with variables: `s.ta("physics-floor", &[("ms", 47.into())])`.
    pub fn ta(&self, id: &str, args: &[(&str, FluentValue)]) -> String {
        let mut fargs = FluentArgs::new();
        for (k, v) in args {
            fargs.set(*k, v.clone());
        }
        self.format(id, Some(&fargs))
    }

    /// A message with a tiny inline syntax, escaped first: `` `code` `` and `**strong**`.
    pub fn rich(&self, id: &str) -> Markup {
        rich_markup(&self.t(id))
    }

    pub fn has(&self, id: &str) -> bool {
        self.bundle.get_message(id).and_then(|m| m.value()).is_some()
    }

    fn format(&self, id: &str, args: Option<&FluentArgs>) -> String {
        let Some(pattern) = self.bundle.get_message(id).and_then(|m| m.value()) else {
            self.missing.borrow_mut().insert(id.to_owned());
            return format!("⟦{id}⟧");
        };
        let mut errs = vec![];
        let out = self.bundle.format_pattern(pattern, args, &mut errs).into_owned();
        if !errs.is_empty() {
            self.errors.borrow_mut().push(format!("{id}: {errs:?}"));
        }
        out
    }

    /// Fail if any template asked for a message this language doesn't have.
    pub fn finish(&self) -> Result<()> {
        let missing = self.missing.borrow();
        let errors = self.errors.borrow();
        if missing.is_empty() && errors.is_empty() {
            return Ok(());
        }
        let mut msg = format!("strings for {:?} are incomplete", self.lang);
        if !missing.is_empty() {
            msg += &format!("\n  missing: {}", missing.iter().cloned().collect::<Vec<_>>().join(", "));
        }
        for e in errors.iter() {
            msg += &format!("\n  error: {e}");
        }
        bail!(msg)
    }
}

/// Escape, then turn `` `x` `` into `<code>x</code>` and `**x**` into `<strong>x</strong>`.
pub fn rich_markup(text: &str) -> Markup {
    let escaped = crate::util::escape_xml(text);
    let mut out = String::with_capacity(escaped.len() + 16);
    let mut rest = escaped.as_str();
    while let Some(i) = rest.find(['`', '*']) {
        let (before, tail) = rest.split_at(i);
        out.push_str(before);
        if let Some(inner) = tail.strip_prefix('`') {
            if let Some(end) = inner.find('`') {
                out.push_str("<code>");
                out.push_str(&inner[..end]);
                out.push_str("</code>");
                rest = &inner[end + 1..];
                continue;
            }
        } else if let Some(inner) = tail.strip_prefix("**") {
            if let Some(end) = inner.find("**") {
                out.push_str("<strong>");
                out.push_str(&inner[..end]);
                out.push_str("</strong>");
                rest = &inner[end + 2..];
                continue;
            }
        }
        out.push_str(&tail[..1]);
        rest = &tail[1..];
    }
    out.push_str(rest);
    PreEscaped(out)
}

/// Every message in `content/{lang}/islands/*.ftl` as `id → pattern`, with variables written as
/// `{name}` (see `web/shared/i18n.ts`). Terms are inlined. One selector per message is allowed —
/// typically plurals — and each variant is exported as `id[key]` (`hud-requests[one]`), with the
/// default variant also under the plain `id`; the browser picks with `Intl.PluralRules`.
pub fn island_strings(root: &Path, lang: &str) -> Result<BTreeMap<String, String>> {
    strings_in(&ftl_files(&root.join("content").join(lang).join("islands")))
}

/// The bundles in `content/{lang}/islands/lazy/<name>.ftl`, by name: long texts read only after an
/// explicit action, shipped as their own JSON file rather than inline on every page view.
pub fn lazy_island_strings(root: &Path, lang: &str) -> Result<BTreeMap<String, BTreeMap<String, String>>> {
    let mut out = BTreeMap::new();
    for file in ftl_files(&root.join("content").join(lang).join("islands/lazy")) {
        let name = file.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
        out.insert(name, strings_in(std::slice::from_ref(&file))?);
    }
    Ok(out)
}

/// The `.ftl` files directly in `dir`, sorted (none if it doesn't exist).
fn ftl_files(dir: &Path) -> Vec<std::path::PathBuf> {
    let mut files: Vec<_> = match std::fs::read_dir(dir) {
        Ok(rd) => rd.filter_map(|e| e.ok().map(|e| e.path())).filter(|p| p.extension().is_some_and(|x| x == "ftl")).collect(),
        Err(_) => vec![],
    };
    files.sort();
    files
}

fn strings_in(files: &[std::path::PathBuf]) -> Result<BTreeMap<String, String>> {
    let mut terms: BTreeMap<String, ast::Pattern<String>> = BTreeMap::new();
    let mut messages: Vec<(String, ast::Pattern<String>, String)> = vec![];
    for file in files {
        let source = std::fs::read_to_string(file)?;
        let res = fluent_syntax::parser::parse(source)
            .map_err(|(_, errs)| anyhow::anyhow!("{}: Fluent syntax errors: {errs:?}", file.display()))?;
        for entry in res.body {
            match entry {
                ast::Entry::Message(m) => {
                    if let Some(value) = m.value {
                        messages.push((m.id.name, value, file.display().to_string()));
                    }
                }
                ast::Entry::Term(t) => {
                    terms.insert(t.id.name, t.value);
                }
                _ => {}
            }
        }
    }

    let mut out = BTreeMap::new();
    for (id, pattern, file) in messages {
        for (variant, text) in expand(&pattern, &terms).with_context(|| format!("{file}: message {id}"))? {
            let key = match variant {
                Some(v) => format!("{id}[{v}]"),
                None => id.clone(),
            };
            if out.insert(key, text).is_some() {
                bail!("{file}: duplicate island message {id}");
            }
        }
    }
    Ok(out)
}

/// A pattern with at most one selector → `(variant key, text)` pairs; `None` is the default.
fn expand(pattern: &ast::Pattern<String>, terms: &BTreeMap<String, ast::Pattern<String>>) -> Result<Vec<(Option<String>, String)>> {
    let select = pattern.elements.iter().position(|el| {
        matches!(el, ast::PatternElement::Placeable { expression: ast::Expression::Select { .. } })
    });
    let Some(at) = select else {
        return Ok(vec![(None, flatten(pattern, terms)?)]);
    };
    let (before, after) = (&pattern.elements[..at], &pattern.elements[at + 1..]);
    let ast::PatternElement::Placeable { expression: ast::Expression::Select { selector, variants } } = &pattern.elements[at] else {
        unreachable!()
    };
    if !matches!(selector, ast::InlineExpression::VariableReference { .. }) {
        bail!("an island selector must select on a variable, e.g. {{ $n -> … }}");
    }
    let part = |els: &[ast::PatternElement<String>]| flatten(&ast::Pattern { elements: els.to_vec() }, terms);
    let (head, tail) = (part(before)?, part(after)?);
    let mut out = vec![];
    for v in variants {
        let key = match &v.key {
            ast::VariantKey::Identifier { name } => name.clone(),
            ast::VariantKey::NumberLiteral { value } => value.clone(),
        };
        let text = format!("{head}{}{tail}", flatten(&v.value, terms)?);
        if v.default {
            out.push((None, text.clone()));
        }
        out.push((Some(key), text));
    }
    Ok(out)
}

fn flatten(pattern: &ast::Pattern<String>, terms: &BTreeMap<String, ast::Pattern<String>>) -> Result<String> {
    let mut s = String::new();
    for el in &pattern.elements {
        match el {
            ast::PatternElement::TextElement { value } => s.push_str(value),
            ast::PatternElement::Placeable { expression } => match expression {
                ast::Expression::Inline(inline) => match inline {
                    ast::InlineExpression::VariableReference { id } => {
                        s.push('{');
                        s.push_str(&id.name);
                        s.push('}');
                    }
                    ast::InlineExpression::StringLiteral { value } => s.push_str(value),
                    ast::InlineExpression::NumberLiteral { value } => s.push_str(value),
                    ast::InlineExpression::TermReference { id, .. } => {
                        let term = terms.get(&id.name).ok_or_else(|| anyhow::anyhow!("unknown term -{}", id.name))?;
                        s.push_str(&flatten(term, terms)?);
                    }
                    other => bail!("unsupported expression in an island string: {other:?}"),
                },
                ast::Expression::Select { .. } => bail!("only one selector per island message is supported"),
            },
        }
    }
    Ok(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rich_markup_escapes_then_formats() {
        let m = rich_markup("Use `<script>` and **0** cookies & trackers");
        assert_eq!(m.into_string(), "Use <code>&lt;script&gt;</code> and <strong>0</strong> cookies &amp; trackers");
    }

    #[test]
    fn rich_markup_leaves_lone_markers() {
        assert_eq!(rich_markup("2 * 3 = 6").into_string(), "2 * 3 = 6");
    }
}

#[cfg(test)]
mod island_tests {
    use super::*;

    #[test]
    fn plurals_become_one_key_per_variant() {
        let dir = std::env::temp_dir().join(format!("mintaka-i18n-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("content/xx/islands")).unwrap();
        std::fs::write(
            dir.join("content/xx/islands/t.ftl"),
            "t-req = Sent { $n ->\n    [one] { $n } request\n   *[other] { $n } requests\n} today\nt-plain = Hi { $who }\n",
        )
        .unwrap();
        let m = island_strings(&dir, "xx").unwrap();
        assert_eq!(m["t-req[one]"], "Sent {n} request today");
        assert_eq!(m["t-req[other]"], "Sent {n} requests today");
        assert_eq!(m["t-req"], "Sent {n} requests today");
        assert_eq!(m["t-plain"], "Hi {who}");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn lazy_bundles_stay_out_of_the_inline_strings() {
        let dir = std::env::temp_dir().join(format!("mintaka-i18n-lazy-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("content/xx/islands/lazy")).unwrap();
        std::fs::write(dir.join("content/xx/islands/t.ftl"), "t-fail = It did not load\n").unwrap();
        std::fs::write(dir.join("content/xx/islands/lazy/t.ftl"), "t-long = A long text for { $who }\n").unwrap();
        let inline = island_strings(&dir, "xx").unwrap();
        assert_eq!(inline.keys().collect::<Vec<_>>(), ["t-fail"]);
        let lazy = lazy_island_strings(&dir, "xx").unwrap();
        assert_eq!(lazy["t"]["t-long"], "A long text for {who}");
        assert!(lazy_island_strings(&dir, "yy").unwrap().is_empty(), "no lazy directory, no bundles");
        let _ = std::fs::remove_dir_all(dir);
    }
}
