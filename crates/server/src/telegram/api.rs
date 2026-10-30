//! A small client for the Telegram Bot API (<https://core.telegram.org/bots/api>): the four methods
//! the server uses, JSON in and out. HTTPS through rustls with the server's own aws-lc-rs provider
//! and compiled-in Mozilla roots (no OpenSSL, no system trust store). The bot token is part of
//! every URL, so URLs are never logged (reqwest errors are reported `without_url`).
//!
//! The wire types keep only the fields the bot reads, and none of them implements `Debug`: a
//! message's text must never end up in a log line by accident.

use std::fmt;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Context;
use serde::Deserialize;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};

use crate::config::TelegramSettings;

/// Every call but the long poll gives up after this long.
const CALL_TIMEOUT: Duration = Duration::from_secs(10);

pub(crate) struct Api {
    client: reqwest::Client,
    /// `{base}/bot{token}/`: holds the token.
    prefix: String,
}

/// Why a call failed.
pub(crate) enum ApiError {
    /// No answer: the network, TLS, a timeout.
    Transport(String),
    /// Telegram said no: its error code (an HTTP status), its description and, for `429`, how many
    /// seconds to wait.
    Refused { code: u16, description: String, retry_after: Option<u64> },
    /// An answer that isn't the documented shape.
    Malformed,
}

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ApiError::Transport(e) => write!(f, "no answer from Telegram: {e}"),
            ApiError::Refused { code, description, .. } => write!(f, "Telegram refused it: {code} {description}"),
            ApiError::Malformed => f.write_str("Telegram's answer wasn't the documented shape"),
        }
    }
}

#[derive(Deserialize)]
struct Envelope {
    ok: bool,
    result: Option<Value>,
    error_code: Option<u16>,
    description: Option<String>,
    parameters: Option<Parameters>,
}

#[derive(Deserialize)]
struct Parameters {
    retry_after: Option<u64>,
}

/// A user or bot (`getMe`, `message.from`).
#[derive(Deserialize)]
pub(crate) struct User {
    #[serde(default)]
    pub is_bot: bool,
    pub username: Option<String>,
}

/// One update from `getUpdates`; only messages are asked for.
#[derive(Deserialize)]
pub(crate) struct Update {
    pub update_id: i64,
    pub message: Option<Message>,
}

#[derive(Deserialize)]
pub(crate) struct Message {
    pub message_id: i64,
    pub chat: Chat,
    pub from: Option<User>,
    pub text: Option<String>,
    pub reply_to_message: Option<Box<Message>>,
}

#[derive(Deserialize)]
pub(crate) struct Chat {
    pub id: i64,
    /// `private`, `group`, `supergroup` or `channel`.
    #[serde(rename = "type")]
    pub kind: String,
}

#[derive(Deserialize)]
struct MessageId {
    message_id: i64,
}

impl Api {
    pub fn new(settings: &TelegramSettings) -> anyhow::Result<Api> {
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
            // Plain HTTP only to a test double on this machine (config checks MINTAKA_TELEGRAM_API_BASE).
            .https_only(settings.api_base.starts_with("https://"))
            .redirect(reqwest::redirect::Policy::none())
            .connect_timeout(CALL_TIMEOUT)
            .build()
            .context("building the Telegram HTTPS client")?;
        Ok(Api { client, prefix: format!("{}/bot{}/", settings.api_base, settings.bot_token) })
    }

    async fn call<T: DeserializeOwned>(&self, method: &str, body: Value, timeout: Duration) -> Result<T, ApiError> {
        let response = self
            .client
            .post(format!("{}{method}", self.prefix))
            .header(http::header::CONTENT_TYPE, "application/json")
            .body(body.to_string())
            .timeout(timeout)
            .send()
            .await
            .map_err(|e| ApiError::Transport(e.without_url().to_string()))?;
        let bytes = response.bytes().await.map_err(|e| ApiError::Transport(e.without_url().to_string()))?;
        let envelope: Envelope = serde_json::from_slice(&bytes).map_err(|_| ApiError::Malformed)?;
        match (envelope.ok, envelope.result) {
            (true, Some(result)) => serde_json::from_value(result).map_err(|_| ApiError::Malformed),
            (true, None) => Err(ApiError::Malformed),
            (false, _) => Err(ApiError::Refused {
                code: envelope.error_code.unwrap_or(0),
                description: envelope.description.unwrap_or_default(),
                retry_after: envelope.parameters.and_then(|p| p.retry_after),
            }),
        }
    }

    /// The bot itself.
    pub async fn get_me(&self) -> Result<User, ApiError> {
        self.call("getMe", json!({}), CALL_TIMEOUT).await
    }

    /// New messages from `offset` on (which also confirms every update before it), waiting up to
    /// `wait` seconds for one to arrive.
    pub async fn get_updates(&self, offset: i64, wait: u64) -> Result<Vec<Update>, ApiError> {
        let body = json!({ "offset": offset, "timeout": wait, "allowed_updates": ["message"] });
        self.call("getUpdates", body, Duration::from_secs(wait) + CALL_TIMEOUT).await
    }

    /// Sends `text` to `chat`, optionally as a reply; returns the new message's number.
    pub async fn send_message(&self, chat: i64, text: &str, reply_to: Option<i64>) -> Result<i64, ApiError> {
        let mut body = json!({ "chat_id": chat, "text": text, "link_preview_options": { "is_disabled": true } });
        if let Some(id) = reply_to {
            body["reply_parameters"] = json!({ "message_id": id, "allow_sending_without_reply": true });
        }
        let sent: MessageId = self.call("sendMessage", body, CALL_TIMEOUT).await?;
        Ok(sent.message_id)
    }

    /// Copies message `message_id` of chat `from` into chat `to`, as the bot's own message (no
    /// "forwarded from"), optionally as a reply; returns the copy's number.
    pub async fn copy_message(&self, to: i64, from: i64, message_id: i64, reply_to: Option<i64>) -> Result<i64, ApiError> {
        let mut body = json!({ "chat_id": to, "from_chat_id": from, "message_id": message_id });
        if let Some(id) = reply_to {
            body["reply_parameters"] = json!({ "message_id": id, "allow_sending_without_reply": true });
        }
        let copied: MessageId = self.call("copyMessage", body, CALL_TIMEOUT).await?;
        Ok(copied.message_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn client_builds_with_rustls() {
        let settings = TelegramSettings { bot_token: "1:x".into(), chat_id: 2, api_base: "https://api.telegram.org".into() };
        let api = Api::new(&settings);
        assert!(api.is_ok(), "{:?}", api.err());
        assert_eq!(api.unwrap().prefix, "https://api.telegram.org/bot1:x/");
    }

    #[test]
    fn errors_name_no_url() {
        let e = ApiError::Refused { code: 403, description: "Forbidden: bot was blocked by the user".into(), retry_after: None };
        assert_eq!(e.to_string(), "Telegram refused it: 403 Forbidden: bot was blocked by the user");
    }
}
