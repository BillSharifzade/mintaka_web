//! The team chat's notification of a new brief: "New brief · kinds · timeline · id" — never the
//! message, name or contact details. Sent through `telegram::Telegram::notify`.

use proof::{Kind, Timeline};

/// "New brief · ai, performance · asap · 1793102400123-a1b2c3d4e5f6a7b8", or "New application
/// (console challenge solved) · …" for a brief sent with a valid challenge pass.
pub(crate) fn notification_text(id: &str, kinds: &[Kind], timeline: Option<Timeline>, ctf_solved: bool) -> String {
    let kinds = if kinds.is_empty() {
        "—".to_owned()
    } else {
        kinds.iter().map(|k| k.as_str()).collect::<Vec<_>>().join(", ")
    };
    let timeline = timeline.map_or("—", Timeline::as_str);
    let what = if ctf_solved { "New application (console challenge solved)" } else { "New brief" };
    format!("{what} · {kinds} · {timeline} · {id}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_has_no_body() {
        assert_eq!(
            notification_text("1-ab", &[Kind::Ai, Kind::Design], Some(Timeline::Asap), false),
            "New brief · ai, design · asap · 1-ab"
        );
        assert_eq!(notification_text("1-ab", &[], None, false), "New brief · — · — · 1-ab");
        assert_eq!(notification_text("1-ab", &[], None, true), "New application (console challenge solved) · — · — · 1-ab");
    }


}
