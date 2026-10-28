//! Performance budgets (spec §10.1). Development builds report; release builds fail.

use std::fmt::Write as _;

pub struct Line {
    pub what: String,
    pub bytes: u64,
    pub limit: Option<u64>,
}

#[derive(Default)]
pub struct Budget {
    pub lines: Vec<Line>,
}

impl Budget {
    pub fn add(&mut self, what: impl Into<String>, bytes: u64, limit: Option<u64>) {
        self.lines.push(Line { what: what.into(), bytes, limit });
    }

    pub fn over(&self) -> Vec<&Line> {
        self.lines.iter().filter(|l| l.limit.is_some_and(|max| l.bytes > max)).collect()
    }

    pub fn table(&self) -> String {
        let mut out = String::new();
        let w = self.lines.iter().map(|l| l.what.len()).max().unwrap_or(10);
        for l in &self.lines {
            let status = match l.limit {
                Some(max) if l.bytes > max => format!("OVER  (≤ {})", kb(max)),
                Some(max) => format!("ok    (≤ {})", kb(max)),
                None => String::new(),
            };
            let _ = writeln!(out, "  {:<w$}  {:>9}  {status}", l.what, kb(l.bytes));
        }
        out
    }
}

pub fn kb(bytes: u64) -> String {
    if bytes < 1000 { format!("{bytes} B") } else { format!("{:.1} kB", bytes as f64 / 1000.0) }
}
