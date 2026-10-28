//! The small pages: 404, the no-JavaScript contact result, privacy.

use super::Ctx;
use maud::{Markup, PreEscaped, html};

/// The server replaces this marker with a link to the closest real page (CONTRACTS §4.1).
pub const SUGGEST_MARKER: &str = "<!--mintaka:suggest-->";

pub fn not_found(ctx: &Ctx) -> Markup {
    let s = ctx.s;
    html! {
        section class="sec sec--small lost" aria-labelledby="lost-title" {
            div class="wrap" {
                div class="lost__sky" aria-hidden="true" { (super::svg::orion_figure(220.0)) }
                p class="eyebrow mono" { "404" }
                h1 class="sec__title" id="lost-title" { (s.t("nf-title")) }
                p class="lead" { (s.t("nf-body")) }
                p class="lost__suggest" { (s.t("nf-suggest")) " " (PreEscaped(SUGGEST_MARKER)) }
                a class="btn btn--primary" href=(ctx.home()) { (s.t("nf-home")) }
            }
        }
    }
}

/// Every error code the server can put in the fragment (`/sent/#e-email-invalid`).
pub const ERROR_CODES: [&str; 13] = [
    "name-required", "name-too-long", "email-invalid", "telegram-invalid", "contact-required", "message-too-long",
    "company-too-long", "choice-invalid", "pow-invalid", "pow-expired", "rate-limited", "too-large", "server-error",
];

/// Result of a contact form posted without JavaScript. The server redirects to `/sent/`
/// (success) or `/sent/#e-{code}`; CSS shows the matching message with `:target`.
pub fn sent(ctx: &Ctx) -> Markup {
    let s = ctx.s;
    let within = ctx.cfg.contact.reply_within.as_str();
    html! {
        section class="sec sec--small sent" aria-labelledby="sent-title" {
            div class="wrap" {
                div class="sent__ok" {
                    h1 class="sec__title" id="sent-title" { (s.t("sent-title")) }
                    p class="lead" { (s.ta("sent-body", &[("within", within.into())])) }
                }
                div class="sent__errors" {
                    p class="sent__error-title" { (s.t("sent-error-title")) }
                    @for code in ERROR_CODES {
                        p class="sent__error" id=(format!("e-{code}")) tabindex="-1" { (s.t(&format!("e-{code}"))) }
                    }
                }
                a class="btn btn--ghost" href=(format!("{}#contact", ctx.home())) { (s.t("sent-back")) }
            }
        }
    }
}

pub fn privacy(ctx: &Ctx) -> Markup {
    let s = ctx.s;
    let parts = ["cookies", "logs", "contact", "physics", "experiments", "storage", "data"];
    html! {
        article class="sec sec--small prose" aria-labelledby="privacy-title" {
            div class="wrap" {
                h1 class="sec__title" id="privacy-title" { (s.t("pv-title")) }
                p class="lead" { (s.t("pv-lead")) }
                @for p in parts {
                    h2 { (s.t(&format!("pv-{p}-title"))) }
                    p { (s.t(&format!("pv-{p}"))) }
                }
                p { (s.ta("pv-contact-us", &[("email", ctx.cfg.contact.email.as_str().into())])) }
            }
        }
    }
}
