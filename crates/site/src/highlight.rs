//! A small highlighter for the code on the page and in the Lab. Runs at build time, so pages
//! ship no highlighting JavaScript. Token classes: `k` keyword, `t` type, `c` comment,
//! `s` string, `n` number, `m` macro, `o` the `?` operator, `a` attribute.
//!
//! Four languages, each a single pass over characters: Rust, shell, TOML and Fluent. That is
//! every language the site shows; a general-purpose highlighter (syntect, tree-sitter) would add
//! megabytes of grammars and seconds of build time for colours this page draws in four tokens.

use crate::util::escape_xml;

const KEYWORDS: &[&str] = &[
    "as", "async", "await", "break", "const", "continue", "crate", "dyn", "else", "enum", "extern", "fn", "for", "if", "impl", "in", "let",
    "loop", "match", "mod", "move", "mut", "pub", "ref", "return", "self", "Self", "static", "struct", "super", "trait", "type", "unsafe",
    "use", "where", "while", "true", "false",
];

/// Highlight `src` written in `lang`; `None` for a language this module doesn't know.
pub fn code(lang: &str, src: &str) -> Option<String> {
    match lang {
        "rust" | "rs" => Some(rust(src)),
        "sh" | "shell" | "bash" | "console" => Some(shell(src)),
        "toml" => Some(toml(src)),
        "ftl" | "fluent" => Some(fluent(src)),
        _ => None,
    }
}

/// Appends `text` as a token of `class` (or as plain text for an empty class).
fn span(out: &mut String, class: &str, text: &str) {
    if text.is_empty() {
        return;
    }
    if class.is_empty() {
        out.push_str(&escape_xml(text));
        return;
    }
    out.push_str("<span class=\"tok-");
    out.push_str(class);
    out.push_str("\">");
    out.push_str(&escape_xml(text));
    out.push_str("</span>");
}

fn collect(chars: &[char], from: usize, to: usize) -> String {
    chars[from..to.min(chars.len())].iter().collect()
}

/// Index just past a `"…"` string starting at `i` (backslash escapes allowed).
fn string_end(chars: &[char], i: usize, quote: char) -> usize {
    let mut end = i + 1;
    while end < chars.len() && chars[end] != quote {
        end += if chars[end] == '\\' { 2 } else { 1 };
    }
    (end + 1).min(chars.len())
}

pub fn rust(src: &str) -> String {
    let mut out = String::with_capacity(src.len() * 2);
    let chars: Vec<char> = src.chars().collect();
    let at = |i: usize| chars.get(i).copied();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c == '/' && at(i + 1) == Some('/') {
            let end = chars[i..].iter().position(|&c| c == '\n').map_or(chars.len(), |p| i + p);
            span(&mut out, "c", &collect(&chars, i, end));
            i = end;
        } else if c == '/' && at(i + 1) == Some('*') {
            let end = (i + 2..chars.len().saturating_sub(1)).find(|&j| chars[j] == '*' && chars[j + 1] == '/').map_or(chars.len(), |j| j + 2);
            span(&mut out, "c", &collect(&chars, i, end));
            i = end;
        } else if c == 'r' && (at(i + 1) == Some('"') || (at(i + 1) == Some('#') && matches!(at(i + 2), Some('"' | '#')))) {
            // Raw string: r"…", r#"…"#, r##"…"##.
            let hashes = chars[i + 1..].iter().take_while(|&&c| c == '#').count();
            let open = i + 1 + hashes;
            let close: String = std::iter::once('"').chain(std::iter::repeat_n('#', hashes)).collect();
            let rest = collect(&chars, open + 1, chars.len());
            let end = rest.find(&close).map_or(chars.len(), |p| open + 1 + rest[..p].chars().count() + close.chars().count());
            span(&mut out, "s", &collect(&chars, i, end));
            i = end;
        } else if c == '"' {
            let end = string_end(&chars, i, '"');
            span(&mut out, "s", &collect(&chars, i, end));
            i = end;
        } else if c == '\'' {
            // A char literal ('x', '\n', '\'') or a lifetime ('a, 'static).
            let end = if at(i + 1) == Some('\\') {
                string_end(&chars, i, '\'')
            } else if at(i + 2) == Some('\'') {
                i + 3
            } else {
                0
            };
            if end > 0 {
                span(&mut out, "s", &collect(&chars, i, end));
                i = end;
            } else {
                let end = chars[i + 1..].iter().position(|c| !(c.is_alphanumeric() || *c == '_')).map_or(chars.len(), |p| i + 1 + p);
                span(&mut out, "t", &collect(&chars, i, end));
                i = end;
            }
        } else if c == '#' && (at(i + 1) == Some('[') || (at(i + 1) == Some('!') && at(i + 2) == Some('['))) {
            let mut depth = 0;
            let mut end = chars.len();
            for (j, &ch) in chars.iter().enumerate().skip(i) {
                match ch {
                    '[' => depth += 1,
                    ']' => {
                        depth -= 1;
                        if depth == 0 {
                            end = j + 1;
                            break;
                        }
                    }
                    _ => {}
                }
            }
            span(&mut out, "a", &collect(&chars, i, end));
            i = end;
        } else if c.is_ascii_digit() {
            let mut end = i;
            while end < chars.len() {
                let ch = chars[end];
                // `1.5` is one number; `0..n` is a number and a range.
                let dot = ch == '.' && at(end + 1).is_some_and(|n| n.is_ascii_digit());
                if !(ch.is_ascii_alphanumeric() || ch == '_' || dot) {
                    break;
                }
                end += 1;
            }
            span(&mut out, "n", &collect(&chars, i, end));
            i = end;
        } else if c.is_alphabetic() || c == '_' {
            let end = chars[i..].iter().position(|c| !(c.is_alphanumeric() || *c == '_')).map_or(chars.len(), |p| i + p);
            let word = collect(&chars, i, end);
            if at(end) == Some('!') && at(end + 1) != Some('=') {
                span(&mut out, "m", &format!("{word}!"));
                i = end + 1;
                continue;
            }
            let class = if KEYWORDS.contains(&word.as_str()) {
                "k"
            } else if word.chars().next().is_some_and(char::is_uppercase) {
                "t"
            } else {
                ""
            };
            span(&mut out, class, &word);
            i = end;
        } else if c == '?' {
            span(&mut out, "o", "?");
            i += 1;
        } else {
            span(&mut out, "", &c.to_string());
            i += 1;
        }
    }
    out
}

/// Shell commands: comments, quoted strings, and the command word of each command.
pub fn shell(src: &str) -> String {
    let mut out = String::with_capacity(src.len() * 2);
    let mut continued = false;
    for (n, line) in src.split('\n').enumerate() {
        if n > 0 {
            out.push('\n');
        }
        let chars: Vec<char> = line.chars().collect();
        let mut i = 0;
        let mut expect_command = !continued;
        while i < chars.len() {
            let c = chars[i];
            if c == '#' && (i == 0 || chars[i - 1].is_whitespace()) {
                span(&mut out, "c", &collect(&chars, i, chars.len()));
                i = chars.len();
            } else if c == '"' || c == '\'' {
                let end = string_end(&chars, i, c);
                span(&mut out, "s", &collect(&chars, i, end));
                i = end;
                expect_command = false;
            } else if c.is_whitespace() {
                span(&mut out, "", &c.to_string());
                i += 1;
            } else if matches!(c, '|' | ';' | '&' | '(' | ')') {
                span(&mut out, "", &c.to_string());
                expect_command = true;
                i += 1;
            } else {
                let end = chars[i..].iter().position(|c| c.is_whitespace() || matches!(c, '|' | ';' | '&' | '"' | '\'' | ')')).map_or(chars.len(), |p| i + p);
                let word = collect(&chars, i, end);
                // An environment assignment (`RUST_LOG=info cmd`) keeps the command word to come.
                let assignment = word.contains('=') && !word.starts_with('-');
                span(&mut out, if expect_command && !assignment { "k" } else { "" }, &word);
                expect_command &= assignment;
                i = end;
            }
        }
        continued = line.trim_end().ends_with('\\');
    }
    out
}

/// TOML: comments, `[tables]`, keys, strings, numbers and booleans.
pub fn toml(src: &str) -> String {
    let mut out = String::with_capacity(src.len() * 2);
    for (n, line) in src.split('\n').enumerate() {
        if n > 0 {
            out.push('\n');
        }
        let trimmed = line.trim_start();
        let indent = &line[..line.len() - trimmed.len()];
        span(&mut out, "", indent);
        if trimmed.starts_with('#') {
            span(&mut out, "c", trimmed);
        } else if trimmed.starts_with('[') {
            span(&mut out, "a", trimmed);
        } else if let Some(eq) = trimmed.find('=') {
            span(&mut out, "t", &trimmed[..eq]);
            span(&mut out, "", "=");
            toml_value(&mut out, &trimmed[eq + 1..]);
        } else {
            span(&mut out, "", trimmed);
        }
    }
    out
}

fn toml_value(out: &mut String, value: &str) {
    let chars: Vec<char> = value.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c == '"' || c == '\'' {
            let end = string_end(&chars, i, c);
            span(out, "s", &collect(&chars, i, end));
            i = end;
        } else if c == '#' {
            span(out, "c", &collect(&chars, i, chars.len()));
            i = chars.len();
        } else if c.is_ascii_alphanumeric() {
            let end = chars[i..].iter().position(|c| !(c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '-' | ':'))).map_or(chars.len(), |p| i + p);
            let word = collect(&chars, i, end);
            let class = if word == "true" || word == "false" { "k" } else if c.is_ascii_digit() { "n" } else { "" };
            span(out, class, &word);
            i = end;
        } else {
            span(out, "", &c.to_string());
            i += 1;
        }
    }
}

/// Fluent: comments, message ids, variables and variant keys.
pub fn fluent(src: &str) -> String {
    let mut out = String::with_capacity(src.len() * 2);
    for (n, line) in src.split('\n').enumerate() {
        if n > 0 {
            out.push('\n');
        }
        if line.starts_with('#') {
            span(&mut out, "c", line);
            continue;
        }
        let mut rest = line;
        // `message-id =` at the start of a line.
        if let Some(eq) = line.find(" =")
            && !line.starts_with(' ')
            && line[..eq].chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
        {
            span(&mut out, "t", &line[..eq]);
            rest = &line[eq..];
        }
        let chars: Vec<char> = rest.chars().collect();
        let mut i = 0;
        while i < chars.len() {
            let c = chars[i];
            if c == '$' {
                let end = chars[i + 1..].iter().position(|c| !(c.is_ascii_alphanumeric() || *c == '_' || *c == '-')).map_or(chars.len(), |p| i + 1 + p);
                span(&mut out, "a", &collect(&chars, i, end));
                i = end;
            } else if c == '[' || (c == '*' && chars.get(i + 1) == Some(&'[')) {
                let end = chars[i..].iter().position(|&c| c == ']').map_or(chars.len(), |p| i + p + 1);
                span(&mut out, "k", &collect(&chars, i, end));
                i = end;
            } else {
                span(&mut out, "", &c.to_string());
                i += 1;
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn highlights_tokens() {
        let html = rust("pub struct Email(String); // checked\nlet x = f()?;");
        assert!(html.contains(r#"<span class="tok-k">pub</span>"#));
        assert!(html.contains(r#"<span class="tok-t">Email</span>"#));
        assert!(html.contains(r#"<span class="tok-c">// checked</span>"#));
        assert!(html.contains(r#"<span class="tok-o">?</span>"#));
    }

    #[test]
    fn rust_literals_lifetimes_and_ranges() {
        let html = rust(r##"let q = '"'; let s: &'static str = r#"say "hi""#; for i in 0..n { x != y; }"##);
        // A quote inside a char literal does not open a string.
        assert!(html.contains(r#"<span class="tok-s">&#39;&quot;&#39;</span>"#), "{html}");
        assert!(html.contains(r#"<span class="tok-t">&#39;static</span>"#), "{html}");
        // A raw string ends at "#, not at the first quote inside it.
        assert!(html.contains(r#"<span class="tok-s">r#&quot;say &quot;hi&quot;&quot;#</span>"#), "{html}");
        assert!(html.contains(r#"<span class="tok-n">0</span>..n"#), "{html}");
        // `x != y` is a comparison, not a macro call.
        assert!(!html.contains("tok-m"), "{html}");
        let attr = rust("#[cfg(all(test, target_os = \"linux\"))]\nfn f() {}");
        assert!(attr.starts_with(r#"<span class="tok-a">#[cfg(all(test, target_os = &quot;linux&quot;))]</span>"#), "{attr}");
    }

    #[test]
    fn shell_marks_commands_strings_and_comments() {
        let html = shell("RUST_LOG=info cargo run -p server \\\n    --release # fast\ncurl -s 'http://x' | jq .tls");
        assert!(html.contains(r#"RUST_LOG=info <span class="tok-k">cargo</span> run"#), "{html}");
        // The continued line has no command word of its own.
        assert!(!html.contains(r#"<span class="tok-k">--release</span>"#), "{html}");
        assert!(html.contains(r#"<span class="tok-c"># fast</span>"#), "{html}");
        assert!(html.contains(r#"<span class="tok-s">&#39;http://x&#39;</span>"#), "{html}");
        assert!(html.contains(r#"| <span class="tok-k">jq</span>"#), "{html}");
    }

    #[test]
    fn toml_and_fluent() {
        let t = toml("[lab]\n# where links go\nrepo = \"https://example.org\"\nverified = true");
        assert!(t.contains(r#"<span class="tok-a">[lab]</span>"#));
        assert!(t.contains(r#"<span class="tok-t">repo </span>= <span class="tok-s">&quot;https"#), "{t}");
        assert!(t.contains(r#"<span class="tok-k">true</span>"#));
        let f = fluent("hud-requests = { $n ->\n    [one] { $n } request\n   *[other] { $n } requests\n}");
        assert!(f.starts_with(r#"<span class="tok-t">hud-requests</span>"#), "{f}");
        assert!(f.contains(r#"<span class="tok-a">$n</span>"#));
        assert!(f.contains(r#"<span class="tok-k">*[other]</span>"#));
        assert_eq!(code("python", "x"), None);
    }
}
