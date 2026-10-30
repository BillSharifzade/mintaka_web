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
    /// While a page renders as one variant of the A/B test: (the control's id, the variant's).
    variant: RefCell<Option<(String, String)>>,
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
        Ok(Self { lang: lang.to_owned(), bundle, missing: Default::default(), errors: Default::default(), variant: Default::default() })
    }

    /// Runs `render` with message `control` read as message `variant` wherever it is used: one
    /// variant of the A/B test (spec §12). With `variant == control` it renders the control, still
    /// marked as part of the test ([`Strings::variant`]).
    pub fn as_variant<T>(&self, control: &str, variant: &str, render: impl FnOnce() -> T) -> T {
        self.variant.replace(Some((control.to_owned(), variant.to_owned())));
        let out = render();
        self.variant.replace(None);
        out
    }

    /// The A/B test variant being rendered, if any.
    pub fn variant(&self) -> Option<String> {
        self.variant.borrow().as_ref().map(|(_, v)| v.clone())
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
        let swapped = self.variant.borrow().as_ref().filter(|(control, _)| control == id).map(|(_, v)| v.clone());
        let id = swapped.as_deref().unwrap_or(id);
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

    /// A whole number grouped the way this language writes it (CLDR, as `Intl.NumberFormat` does
    /// in the browser): `4,800` in English, `4 800` with a no-break space in Russian and Tajik.
    pub fn int(&self, n: u64) -> String {
        group_digits(&n.to_string(), self.separators().0)
    }

    /// `x` with exactly `digits` decimals in this language: `15.3` in English, `15,3` in Russian
    /// and Tajik.
    pub fn dec(&self, x: f64, digits: usize) -> String {
        let (group, point) = self.separators();
        let text = format!("{x:.digits$}");
        let (int, frac) = text.split_once('.').unwrap_or((&text, ""));
        let (sign, int) = int.strip_prefix('-').map_or(("", int), |i| ("-", i));
        let int = group_digits(int, group);
        if frac.is_empty() { format!("{sign}{int}") } else { format!("{sign}{int}{point}{frac}") }
    }

    /// A unit symbol as this language writes it (web/shared/format.ts has the same table).
    pub fn unit<'a>(&self, symbol: &'a str) -> &'a str {
        unit_symbol(&self.lang, symbol)
    }

    /// (group separator, decimal separator) for this language.
    fn separators(&self) -> (char, char) {
        match self.lang.as_str() {
            "ru" | "tg" => ('\u{A0}', ','),
            _ => (',', '.'),
        }
    }
}

/// SI symbols in Cyrillic for Russian (CLDR's) and Tajik (the same, as Tajik technical writing
/// uses them); every other language keeps the international symbol.
pub fn unit_symbol<'a>(lang: &str, symbol: &'a str) -> &'a str {
    match (lang, symbol) {
        ("ru" | "tg", "µs") => "мкс",
        ("ru" | "tg", "ms") => "мс",
        ("ru" | "tg", "s") => "с",
        ("ru" | "tg", "B") => "Б",
        ("ru" | "tg", "kB") => "кБ",
        ("ru" | "tg", "MB") => "МБ",
        ("ru" | "tg", "km") => "км",
        _ => symbol,
    }
}

/// "4800" → "4,800" with `sep` between groups of three (CLDR: groups from four digits up).
fn group_digits(digits: &str, sep: char) -> String {
    let mut out = String::with_capacity(digits.len() + digits.len() / 3 * 2);
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(sep);
        }
        out.push(c);
    }
    out
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
        } else if let Some(inner) = tail.strip_prefix("**")
            && let Some(end) = inner.find("**")
        {
            out.push_str("<strong>");
            out.push_str(&inner[..end]);
            out.push_str("</strong>");
            rest = &inner[end + 2..];
            continue;
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
/// default variant under the plain `id` (a plural category saying the same as the default is left
/// out: the browser falls back to `id`); the browser picks with `Intl.PluralRules`.
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

/// Check a translation against the source language (CONTRACTS §10.1 rule 7): every FTL file of
/// the source (`page.ftl`, `islands/*.ftl`, `islands/lazy/*.ftl`) with every message and the same
/// attributes, plural variants only under CLDR category names or numbers, and no variable the
/// source doesn't pass (the code would never fill it in). Fails on any of those. Comes back as
/// warnings: a variable the translation leaves out (allowed — the Russian reply-time promise is
/// written out rather than built from an English phrase) and messages or files the source
/// doesn't have (dead copy).
pub fn check_translation(root: &Path, source: &str, lang: &str) -> Result<Vec<String>> {
    let content = root.join("content");
    let files = |l: &str| -> BTreeSet<String> {
        let dir = content.join(l);
        let all = ftl_files(&dir).into_iter().chain(ftl_files(&dir.join("islands"))).chain(ftl_files(&dir.join("islands/lazy")));
        all.filter_map(|p| p.strip_prefix(&dir).ok().map(|r| r.to_string_lossy().into_owned())).collect()
    };
    let (want, have) = (files(source), files(lang));
    let mut errors = vec![];
    let mut warnings = vec![];
    for f in want.difference(&have) {
        errors.push(format!("content/{lang}/{f} is missing (content/{source}/{f} has it)"));
    }
    for f in have.difference(&want) {
        warnings.push(format!("content/{lang}/{f} has no counterpart in content/{source}/ (unused)"));
    }
    for f in want.intersection(&have) {
        let (src, tr) = (shapes(&content.join(source).join(f))?, shapes(&content.join(lang).join(f))?);
        let at = format!("content/{lang}/{f}");
        for id in src.keys().filter(|id| !tr.contains_key(*id)) {
            errors.push(format!("{at}: {id} is missing"));
        }
        // Harmless but dead: nothing asks for a message the source doesn't have.
        for id in tr.keys().filter(|id| !src.contains_key(*id)) {
            warnings.push(format!("{at}: {id} is not in content/{source}/{f} (unused)"));
        }
        for (id, t) in &tr {
            let Some(s) = src.get(id) else { continue };
            if s.value != t.value {
                errors.push(format!("{at}: {id} {} a value", if s.value { "needs" } else { "must not have" }));
            }
            if s.attributes != t.attributes {
                errors.push(format!("{at}: {id} has attributes {:?}, the source {:?}", t.attributes, s.attributes));
            }
            for v in t.variables.difference(&s.variables) {
                errors.push(format!("{at}: {id} uses ${v}, which the source never passes"));
            }
            for v in s.variables.difference(&t.variables) {
                warnings.push(format!("{at}: {id} leaves out ${v}"));
            }
            for k in t.keys.iter().filter(|k| !matches!(k.as_str(), "zero" | "one" | "two" | "few" | "many" | "other")) {
                errors.push(format!("{at}: {id} has a variant [{k}], which is no plural category (zero one two few many other) or number"));
            }
        }
    }
    if !errors.is_empty() {
        bail!("the {lang} strings don't match the {source} source:\n  {}", errors.join("\n  "));
    }
    Ok(warnings)
}

/// What a translation must keep of each message.
struct Shape {
    value: bool,
    attributes: BTreeSet<String>,
    variables: BTreeSet<String>,
    /// Named variant keys (`[one]`); numeric keys aren't listed.
    keys: Vec<String>,
}

fn shapes(file: &Path) -> Result<BTreeMap<String, Shape>> {
    let source = std::fs::read_to_string(file).with_context(|| format!("reading {}", file.display()))?;
    let res = fluent_syntax::parser::parse(source).map_err(|(_, errs)| anyhow::anyhow!("{}: Fluent syntax errors: {errs:?}", file.display()))?;
    let mut out = BTreeMap::new();
    for entry in res.body {
        let ast::Entry::Message(m) = entry else { continue };
        let mut shape = Shape { value: m.value.is_some(), attributes: BTreeSet::new(), variables: BTreeSet::new(), keys: vec![] };
        if let Some(p) = &m.value {
            walk_pattern(p, &mut shape);
        }
        for a in &m.attributes {
            shape.attributes.insert(a.id.name.clone());
            walk_pattern(&a.value, &mut shape);
        }
        out.insert(m.id.name, shape);
    }
    Ok(out)
}

fn walk_pattern(p: &ast::Pattern<String>, shape: &mut Shape) {
    for el in &p.elements {
        if let ast::PatternElement::Placeable { expression } = el {
            walk_expression(expression, shape);
        }
    }
}

fn walk_expression(e: &ast::Expression<String>, shape: &mut Shape) {
    match e {
        ast::Expression::Inline(i) => walk_inline(i, shape),
        ast::Expression::Select { selector, variants } => {
            walk_inline(selector, shape);
            for v in variants {
                if let ast::VariantKey::Identifier { name } = &v.key {
                    shape.keys.push(name.clone());
                }
                walk_pattern(&v.value, shape);
            }
        }
    }
}

fn walk_inline(i: &ast::InlineExpression<String>, shape: &mut Shape) {
    match i {
        ast::InlineExpression::VariableReference { id } => {
            shape.variables.insert(id.name.clone());
        }
        ast::InlineExpression::FunctionReference { arguments, .. } | ast::InlineExpression::TermReference { arguments: Some(arguments), .. } => {
            for a in &arguments.positional {
                walk_inline(a, shape);
            }
            for a in &arguments.named {
                walk_inline(&a.value, shape);
            }
        }
        ast::InlineExpression::Placeable { expression } => walk_expression(expression, shape),
        _ => {}
    }
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
    let mut categories = vec![];
    for v in variants {
        let text = format!("{head}{}{tail}", flatten(&v.value, terms)?);
        if v.default {
            out.push((None, text.clone()));
        }
        match &v.key {
            ast::VariantKey::Identifier { name } => categories.push((name.clone(), text)),
            ast::VariantKey::NumberLiteral { value } => out.push((Some(value.clone()), text)),
        }
    }
    // A plural category whose text is the default's adds nothing: the browser falls back to the
    // plain id (Russian often has two categories saying the same, `[other]` is usually the default).
    let default = out.iter().find(|(k, _)| k.is_none()).map(|(_, t)| t.clone());
    out.extend(categories.into_iter().filter(|(_, t)| Some(t) != default.as_ref()).map(|(k, t)| (Some(k), t)));
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

    #[test]
    fn a_variant_reads_in_place_of_the_control() {
        let dir = std::env::temp_dir().join(format!("mintaka-i18n-variant-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("content/xx")).unwrap();
        std::fs::write(dir.join("content/xx/page.ftl"), "title = Software at the edge of physics.\ntitle-b = As fast as the hardware allows.\nsub = Fast by proof.\n").unwrap();
        let s = Strings::load(&dir, "xx").unwrap();
        assert_eq!(s.variant(), None);
        let (title, sub, variant) = s.as_variant("title", "title-b", || (s.t("title"), s.t("sub"), s.variant()));
        assert_eq!((title.as_str(), sub.as_str(), variant.as_deref()), ("As fast as the hardware allows.", "Fast by proof.", Some("title-b")));
        assert_eq!(s.as_variant("title", "title", || s.t("title")), "Software at the edge of physics.");
        assert_eq!((s.t("title"), s.variant()), ("Software at the edge of physics.".to_owned(), None), "only while rendering");
        // A variant missing in a language fails the build like any other message.
        s.as_variant("title", "title-c", || s.t("title"));
        assert!(s.finish().unwrap_err().to_string().contains("title-c"));
        let _ = std::fs::remove_dir_all(dir);
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
        assert_eq!(m["t-req"], "Sent {n} requests today");
        assert!(!m.contains_key("t-req[other]"), "the default category is the plain id");
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

#[cfg(test)]
mod translation_tests {
    use super::*;

    #[test]
    fn numbers_and_units_follow_the_language() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let en = Strings::load(&root, "en").unwrap();
        assert_eq!(en.int(4800), "4,800");
        assert_eq!(en.int(999), "999");
        assert_eq!(en.dec(1234.56, 1), "1,234.6");
        assert_eq!(en.unit("km"), "km");
        for lang in ["ru", "tg"] {
            let s = Strings::load(&root, lang).unwrap();
            assert_eq!(s.int(4800), "4\u{a0}800", "{lang}");
            assert_eq!(s.int(1_234_567), "1\u{a0}234\u{a0}567", "{lang}");
            assert_eq!(s.dec(15.34, 1), "15,3", "{lang}");
            assert_eq!(s.dec(-0.05, 2), "-0,05", "{lang}");
            assert_eq!((s.unit("km"), s.unit("ms"), s.unit("µs"), s.unit("kB")), ("км", "мс", "мкс", "кБ"), "{lang}");
            assert_eq!(s.unit("×"), "×");
        }
    }

    #[test]
    fn translations_keep_the_source_shape() {
        let dir = std::env::temp_dir().join(format!("mintaka-i18n-shape-{}", std::process::id()));
        let write = |rel: &str, text: &str| {
            let p = dir.join(rel);
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::write(p, text).unwrap();
        };
        write("content/xx/page.ftl", "a = A { $n }\nb = B\n    .title = T\nc = C { $within }\n");
        write("content/xx/islands/t.ftl", "t-n = { $n ->\n    [one] { $n } thing\n   *[other] { $n } things\n}\n");
        write("content/yy/page.ftl", "a = Б { $n }\nb = В\n    .title = Т\nc = Ц\n");
        write("content/yy/islands/t.ftl", "t-n = { $n ->\n    [one] { $n } вещь\n    [few] { $n } вещи\n    [1] одна\n   *[many] { $n } вещей\n}\n");
        let warnings = check_translation(&dir, "xx", "yy").unwrap();
        assert_eq!(warnings, ["content/yy/page.ftl: c leaves out $within"]);

        write("content/yy/page.ftl", "a = Б { $m }\nb = В\nd = Д\n");
        write("content/yy/islands/t.ftl", "t-n = { $n ->\n    [one] { $n } вещь\n   *[others] { $n } вещи\n}\n");
        let err = check_translation(&dir, "xx", "yy").unwrap_err().to_string();
        for needle in ["c is missing", "b has attributes", "a uses $m", "variant [others]"] {
            assert!(err.contains(needle), "{needle:?} not in {err}");
        }
        std::fs::remove_file(dir.join("content/yy/islands/t.ftl")).unwrap();
        let err = check_translation(&dir, "xx", "yy").unwrap_err().to_string();
        assert!(err.contains("content/yy/islands/t.ftl is missing"), "{err}");
        let _ = std::fs::remove_dir_all(dir);
    }
}
