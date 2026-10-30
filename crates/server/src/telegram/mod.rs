//! Telegram (docs/CONTRACTS.md §8.2, §19), configured with `MINTAKA_TELEGRAM_BOT_TOKEN` and
//! `MINTAKA_TELEGRAM_CHAT_ID`: the team chat's notification of every new brief, and the bot a client
//! can continue in from the form's last screen. The bot answers at once, in the brief's language,
//! copies what the client writes into the team chat and the team's replies back. It long-polls
//! (`getUpdates`), so nothing has to reach this server from outside; its memory is
//! `{data-dir}/telegram.json` (`chats`), its links' key `{data-dir}/telegram.key` (`link`).

mod api;
mod chats;
mod link;
mod relay;

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, OnceLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::ensure;
use proof::BOT_MESSAGES;
use tokio::sync::watch;

pub(crate) use chats::Chats;

use crate::config::TelegramSettings;
use api::{Api, ApiError, User};
use link::Links;

/// How long one `getUpdates` waits for a message, in seconds.
const POLL_SECS: u64 = 50;
/// How long startup waits for `getMe` before serving without links (the bot keeps asking).
const HELLO_TIMEOUT: Duration = Duration::from_secs(5);
/// Links are offered only while Telegram answered the bot this recently, in seconds: the form
/// promises that the bot answers right away.
const ALIVE_SECS: u64 = 2 * POLL_SECS + 20;
/// The longest wait between attempts after errors.
const MAX_BACKOFF: Duration = Duration::from_secs(300);

fn unix_now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs())
}

pub(crate) struct Telegram {
    api: Api,
    /// The team chat: notifications go there, and so do the clients' messages.
    team: i64,
    links: Links,
    /// `lang -> (message id -> text)` from `manifest.bot`, and the language to fall back to.
    strings: BTreeMap<String, BTreeMap<String, String>>,
    default_lang: String,
    /// The bot's username, from `getMe`: links are `https://t.me/{username}?start=…`.
    username: OnceLock<String>,
    /// Unix seconds of Telegram's last answer to the bot (`getMe`, `getUpdates`).
    alive: AtomicU64,
}

impl Telegram {
    /// Loads the links' key and the bot's memory, and checks that the manifest has every message
    /// the bot sends.
    pub fn new(
        settings: &TelegramSettings,
        data_dir: &Path,
        strings: &BTreeMap<String, BTreeMap<String, String>>,
        default_lang: &str,
    ) -> anyhow::Result<(Telegram, Chats)> {
        let defaults = strings.get(default_lang);
        for id in BOT_MESSAGES {
            ensure!(
                defaults.is_some_and(|d| d.contains_key(id)),
                "the Telegram bot is configured, but dist/manifest.json has no `bot` message {id:?} for {default_lang:?}: rebuild dist/"
            );
        }
        let telegram = Telegram {
            api: Api::new(settings)?,
            team: settings.chat_id,
            links: Links::new(crate::secret::load_or_create(data_dir, "telegram.key")?),
            strings: strings.clone(),
            default_lang: default_lang.to_owned(),
            username: OnceLock::new(),
            alive: AtomicU64::new(0),
        };
        Ok((telegram, Chats::open(data_dir)?))
    }

    /// Asks Telegram who the bot is, once, briefly: links can be offered from the first request.
    pub async fn hello(&self) {
        match tokio::time::timeout(HELLO_TIMEOUT, self.api.get_me()).await {
            Ok(Ok(me)) => {
                self.met(me);
            }
            Ok(Err(e)) => tracing::warn!(target: "telegram", "getMe failed ({e}): no Telegram links on the form until the bot reaches Telegram"),
            Err(_) => tracing::warn!(target: "telegram", "no answer to getMe within {HELLO_TIMEOUT:?}: no Telegram links on the form until the bot reaches Telegram"),
        }
    }

    /// Remembers the bot's username; `false` if Telegram sent none.
    fn met(&self, me: User) -> bool {
        let Some(username) = me.username.filter(|u| (5..=32).contains(&u.len()) && u.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')) else {
            tracing::error!(target: "telegram", "getMe returned no usable username: the bot can't offer links");
            return false;
        };
        tracing::info!(target: "telegram", "bot @{username} answers clients and relays them to the team chat");
        let _ = self.username.set(username);
        self.alive.store(unix_now(), Ordering::Relaxed);
        true
    }

    /// The bot's username, while Telegram is answering it.
    fn ready(&self) -> Option<&str> {
        let alive = unix_now().saturating_sub(self.alive.load(Ordering::Relaxed)) <= ALIVE_SECS;
        self.username.get().map(String::as_str).filter(|_| alive)
    }

    /// `https://t.me/<bot>?start=<code>` for brief `id`, written in `lang`.
    pub fn link(&self, id: &str, lang: &str) -> Option<String> {
        let bot = self.ready()?;
        Some(format!("https://t.me/{bot}?start={}", self.links.code(id, lang)?))
    }

    /// A link of the same shape for a honeypot submission, which the bot ignores.
    pub fn decoy_link(&self, lang: &str) -> Option<String> {
        let bot = self.ready()?;
        Some(format!("https://t.me/{bot}?start={}", self.links.decoy(lang, unix_now() * 1000)?))
    }

    /// One of the bot's messages in `lang` (else the default language), with the brief's number.
    fn text(&self, lang: &str, id: &str, brief: &str) -> String {
        let table = self.strings.get(lang).filter(|t| t.contains_key(id)).or_else(|| self.strings.get(&self.default_lang));
        table.and_then(|t| t.get(id)).map_or_else(String::new, |text| text.replace("{brief}", brief))
    }

    /// Posts `text` to the team chat in the background; failures are logged, never the text.
    pub fn notify(self: &Arc<Self>, text: String) {
        let telegram = Arc::clone(self);
        tokio::spawn(async move {
            match telegram.api.send_message(telegram.team, &text, None).await {
                Ok(_) => tracing::info!(target: "telegram", "notification sent"),
                Err(e) => tracing::warn!(target: "telegram", "the notification didn't reach the team chat: {e}"),
            }
        });
    }

    /// The bot: polls Telegram until the server shuts down, then writes its memory.
    pub async fn run(self: Arc<Self>, chats: Chats, mut shutdown: watch::Receiver<bool>) {
        let mut bot = relay::Bot::new(&self, chats);
        let mut failures = 0u32;
        loop {
            let attempt = if self.username.get().is_none() {
                tokio::select! {
                    result = self.api.get_me() => result.map(|me| self.met(me)).map(|_| Vec::new()),
                    () = stopped(&mut shutdown) => break,
                }
            } else {
                tokio::select! {
                    result = self.api.get_updates(bot.chats.offset(), POLL_SECS) => result,
                    () = stopped(&mut shutdown) => break,
                }
            };
            match attempt {
                Ok(updates) => {
                    failures = 0;
                    if self.username.get().is_none() {
                        break; // getMe answered, without a username: nothing to offer
                    }
                    self.alive.store(unix_now(), Ordering::Relaxed);
                    let now = unix_now();
                    for update in updates {
                        bot.chats.set_offset(update.update_id + 1);
                        bot.handle(update, now).await;
                    }
                    bot.save(now).await;
                }
                Err(ApiError::Refused { code: code @ (401 | 404), .. }) => {
                    tracing::error!(target: "telegram", "Telegram refused the bot token ({code}): the bot stops; check MINTAKA_TELEGRAM_BOT_TOKEN");
                    break;
                }
                Err(e) => {
                    failures = failures.saturating_add(1);
                    let wait = backoff(failures, &e);
                    let hint = match &e {
                        ApiError::Refused { code: 409, .. } => " (a webhook is set, or another server polls with this token)",
                        _ => "",
                    };
                    tracing::warn!(target: "telegram", "polling Telegram failed: {e}{hint}; next try in {} s", wait.as_secs());
                    tokio::select! {
                        () = tokio::time::sleep(wait) => {}
                        () = stopped(&mut shutdown) => break,
                    }
                }
            }
        }
        bot.save(unix_now()).await;
    }
}

/// 1 s, doubling, at most [`MAX_BACKOFF`]; or as long as a `429` asks.
fn backoff(failures: u32, e: &ApiError) -> Duration {
    if let ApiError::Refused { retry_after: Some(secs), .. } = e {
        return Duration::from_secs(*secs).min(MAX_BACKOFF);
    }
    Duration::from_secs(1u64 << failures.saturating_sub(1).min(9)).min(MAX_BACKOFF)
}

async fn stopped(shutdown: &mut watch::Receiver<bool>) {
    let _ = shutdown.wait_for(|stopping| *stopping).await;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backoff_doubles_to_five_minutes_and_honours_retry_after() {
        let net = ApiError::Transport("down".into());
        let waits: Vec<u64> = (1..=11).map(|n| backoff(n, &net).as_secs()).collect();
        assert_eq!(waits, [1, 2, 4, 8, 16, 32, 64, 128, 256, 300, 300]);
        let slow = ApiError::Refused { code: 429, description: "Too Many Requests".into(), retry_after: Some(7) };
        assert_eq!(backoff(1, &slow).as_secs(), 7);
    }
}
