//! Optional Telegram notification for new briefs: "New brief · kinds · timeline · id" — never the
//! message, name or contact details. HTTPS via rustls with the same aws-lc-rs provider as the
//! server and compiled-in Mozilla roots (no OpenSSL, no system trust store needed).

use std::sync::Arc;
use std::time::Duration;

use anyhow::Context;
use proof::{Kind, Timeline};

use crate::config::TelegramSettings;

/// Sends notifications in the background.
pub(crate) struct Notifier {
    client: reqwest::Client,
    /// Contains the bot token: never log it (reqwest errors are logged `without_url`).
    endpoint: String,
    chat_id: String,
}

impl Notifier {
    pub fn new(settings: &TelegramSettings) -> anyhow::Result<Notifier> {
        let mut roots = rustls::RootCertStore::empty();
        roots.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
        let mut tls = rustls::ClientConfig::builder_with_provider(Arc::new(crate::tls::provider()))
            .with_safe_default_protocol_versions()
            .context("TLS client versions")?
            .with_root_certificates(roots)
            .with_no_client_auth();
        tls.alpn_protocols = vec![b"http/1.1".to_vec()];
        let client = reqwest::Client::builder()
            .tls_backend_preconfigured(tls)
            .https_only(true)
            .redirect(reqwest::redirect::Policy::none())
            .timeout(Duration::from_secs(10))
            .build()
            .context("building the Telegram HTTPS client")?;
        Ok(Notifier {
            client,
            endpoint: format!("https://api.telegram.org/bot{}/sendMessage", settings.bot_token),
            chat_id: settings.chat_id.clone(),
        })
    }

    /// Fire and forget; failures are logged without the URL (which holds the token).
    pub fn brief_received(&self, id: &str, kinds: &[Kind], timeline: Option<Timeline>) {
        let request = self
            .client
            .post(&self.endpoint)
            .header(http::header::CONTENT_TYPE, "application/json")
            .body(
                serde_json::json!({
                    "chat_id": self.chat_id,
                    "text": notification_text(id, kinds, timeline),
                    "disable_web_page_preview": true,
                })
                .to_string(),
            );
        tokio::spawn(async move {
            match request.send().await {
                Ok(res) if res.status().is_success() => tracing::info!("telegram notification sent"),
                Ok(res) => tracing::warn!(status = res.status().as_u16(), "telegram refused the notification"),
                Err(e) => tracing::warn!("telegram notification failed: {}", e.without_url()),
            }
        });
    }
}

/// "New brief · ai, performance · asap · 1793102400123-a1b2c3d4e5f6a7b8".
pub(crate) fn notification_text(id: &str, kinds: &[Kind], timeline: Option<Timeline>) -> String {
    let kinds = if kinds.is_empty() {
        "—".to_owned()
    } else {
        kinds.iter().map(|k| k.as_str()).collect::<Vec<_>>().join(", ")
    };
    let timeline = timeline.map_or("—", Timeline::as_str);
    format!("New brief · {kinds} · {timeline} · {id}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_has_no_body() {
        assert_eq!(
            notification_text("1-ab", &[Kind::Ai, Kind::Design], Some(Timeline::Asap)),
            "New brief · ai, design · asap · 1-ab"
        );
        assert_eq!(notification_text("1-ab", &[], None), "New brief · — · — · 1-ab");
    }

    #[test]
    fn client_builds_with_rustls() {
        let n = Notifier::new(&TelegramSettings { bot_token: "1:x".into(), chat_id: "2".into() });
        assert!(n.is_ok(), "{:?}", n.err());
    }
}
