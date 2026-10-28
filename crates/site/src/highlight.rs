//! A small Rust highlighter for the code moments on the page. Runs at build time, so the page
//! ships no highlighting JavaScript. Token classes: `k` keyword, `t` type, `c` comment,
//! `s` string, `n` number, `m` macro, `o` the `?` operator, `a` attribute.

use crate::util::escape_xml;

const KEYWORDS: &[&str] = &[
    "as", "async", "await", "break", "const", "continue", "crate", "dyn", "else", "enum", "fn", "for", "if", "impl", "in", "let", "loop",
    "match", "mod", "move", "mut", "pub", "ref", "return", "self", "static", "struct", "super", "trait", "type", "unsafe", "use", "where",
    "while", "true", "false",
];

pub fn rust(src: &str) -> String {
    let mut out = String::with_capacity(src.len() * 2);
    let chars: Vec<char> = src.chars().collect();
    let mut i = 0;
    let span = |out: &mut String, class: &str, text: &str| {
        out.push_str("<span class=\"tok-");
        out.push_str(class);
        out.push_str("\">");
        out.push_str(&escape_xml(text));
        out.push_str("</span>");
    };
    while i < chars.len() {
        let c = chars[i];
        let rest: String = chars[i..].iter().take(2).collect();
        if rest == "//" {
            let end = chars[i..].iter().position(|&c| c == '\n').map_or(chars.len(), |p| i + p);
            span(&mut out, "c", &chars[i..end].iter().collect::<String>());
            i = end;
        } else if c == '"' {
            let mut end = i + 1;
            while end < chars.len() && chars[end] != '"' {
                end += if chars[end] == '\\' { 2 } else { 1 };
            }
            let end = (end + 1).min(chars.len());
            span(&mut out, "s", &chars[i..end].iter().collect::<String>());
            i = end;
        } else if c == '#' && chars.get(i + 1) == Some(&'[') {
            let end = chars[i..].iter().position(|&c| c == ']').map_or(chars.len(), |p| i + p + 1);
            span(&mut out, "a", &chars[i..end].iter().collect::<String>());
            i = end;
        } else if c.is_ascii_digit() {
            let end = chars[i..].iter().position(|c| !(c.is_ascii_alphanumeric() || *c == '_' || *c == '.')).map_or(chars.len(), |p| i + p);
            span(&mut out, "n", &chars[i..end].iter().collect::<String>());
            i = end;
        } else if c.is_alphabetic() || c == '_' {
            let end = chars[i..].iter().position(|c| !(c.is_alphanumeric() || *c == '_')).map_or(chars.len(), |p| i + p);
            let word: String = chars[i..end].iter().collect();
            if chars.get(end) == Some(&'!') {
                span(&mut out, "m", &format!("{word}!"));
                i = end + 1;
                continue;
            }
            if KEYWORDS.contains(&word.as_str()) {
                span(&mut out, "k", &word);
            } else if word.chars().next().is_some_and(char::is_uppercase) {
                span(&mut out, "t", &word);
            } else {
                out.push_str(&escape_xml(&word));
            }
            i = end;
        } else if c == '?' {
            span(&mut out, "o", "?");
            i += 1;
        } else {
            out.push_str(&escape_xml(&c.to_string()));
            i += 1;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    #[test]
    fn highlights_tokens() {
        let html = super::rust("pub struct Email(String); // checked\nlet x = f()?;");
        assert!(html.contains(r#"<span class="tok-k">pub</span>"#));
        assert!(html.contains(r#"<span class="tok-t">Email</span>"#));
        assert!(html.contains(r#"<span class="tok-c">// checked</span>"#));
        assert!(html.contains(r#"<span class="tok-o">?</span>"#));
    }
}
