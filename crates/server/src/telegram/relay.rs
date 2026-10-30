//! What the bot does with each message (docs/CONTRACTS.md §19).
//!
//! - A private chat sends `/start <code>`: a valid code connects the chat to its brief and the bot
//!   answers at once, in the brief's language; the team chat gets a header for the brief.
//! - A connected chat writes: the message is copied into the team chat, as a reply to the header.
//! - Someone in the team chat replies to one of those messages: the reply is copied back to the
//!   client, as the bot's own message.
//! - `/stop` forgets the chat's link.
//!
//! Everything else is ignored: other groups and channels, other bots, forged or decoy codes, chats
//! that aren't connected (but for one pointer to the site on a bare `/start`). Message text is
//! read only to find a command, and is never logged; nor are chat ids.

use std::collections::HashMap;
use std::time::Instant;

use super::Telegram;
use super::api::{ApiError, Message, Update};
use super::chats::{Chats, Link, MAX_LINKS};
use super::link::Code;

/// Messages per hour a connected chat may send (more are dropped), and the burst.
const LINKED_PER_HOUR: f64 = 30.0;
/// The same for any other private chat: a few answers, then silence.
const OTHERS_PER_HOUR: f64 = 5.0;
/// Idle (full) buckets are dropped once this many chats have one.
const PRUNE_AT: usize = 10_000;

/// The bot's state while it runs; owned by its one task.
pub(super) struct Bot<'a> {
    tg: &'a Telegram,
    pub chats: Chats,
    limits: Limits,
}

impl<'a> Bot<'a> {
    pub fn new(tg: &'a Telegram, chats: Chats) -> Bot<'a> {
        Bot { tg, chats, limits: Limits::default() }
    }

    pub async fn handle(&mut self, update: Update, now: u64) {
        let Some(msg) = update.message else { return };
        // The team chat is trusted whoever writes (an admin posting anonymously appears as a bot).
        if msg.chat.id == self.tg.team {
            return self.team_message(&msg, now).await;
        }
        if msg.chat.kind != "private" || msg.from.as_ref().is_some_and(|u| u.is_bot) {
            return;
        }
        let chat = msg.chat.id;
        let linked = self.chats.by_chat(chat).is_some();
        if !self.limits.admit(chat, linked, Instant::now()) {
            return;
        }
        match msg.text.as_deref().and_then(command) {
            Some(("start", Some(code))) => self.start(chat, code, now).await,
            Some(("start", None)) => match self.chats.by_chat(chat).cloned() {
                Some(link) => self.say(chat, &link.lang, "bot-again", &link.brief).await,
                None => self.say(chat, &self.tg.default_lang, "bot-hello", "").await,
            },
            Some(("stop", _)) => self.stop(chat).await,
            _ if linked => self.relay_to_team(&msg, now).await,
            _ => {}
        }
    }

    /// `/start <code>`.
    async fn start(&mut self, chat: i64, code: &str, now: u64) {
        let brief = match self.tg.links.parse(code, now.saturating_mul(1000)) {
            Code::Brief(brief) => brief,
            Code::Expired { lang } => return self.say(chat, &lang, "bot-expired", "").await,
            // Forged, damaged or a decoy: no answer at all.
            Code::Unknown => return,
        };
        if let Some(link) = self.chats.by_brief(&brief.id) {
            if link.chat != chat {
                return self.say(chat, &brief.lang, "bot-taken", "").await;
            }
            self.chats.update(chat, |l| l.seen = now);
            return self.say(chat, &brief.lang, "bot-again", &brief.id).await;
        }
        let link = Link { chat, brief: brief.id.clone(), lang: brief.lang.clone(), linked: now, seen: now, header: None, team: Vec::new() };
        if !self.chats.connect(link) {
            tracing::warn!(target: "telegram", "{MAX_LINKS} chats are connected already; one more could not be");
            return;
        }
        // The answer right away, then the team's header.
        self.say(chat, &brief.lang, "bot-linked", &brief.id).await;
        self.header(chat).await;
        tracing::info!(target: "telegram", brief = %brief.id, "a Telegram chat connected to a brief");
    }

    /// `/stop`.
    async fn stop(&mut self, chat: i64) {
        let Some(link) = self.chats.forget(chat) else { return };
        self.say(chat, &link.lang, "bot-stopped", &link.brief).await;
        let note = format!("Brief {}: the client disconnected this Telegram chat (/stop). Replies won't reach them any more.", link.brief);
        self.note(&note, link.header).await;
        tracing::info!(target: "telegram", brief = %link.brief, "a Telegram chat disconnected (/stop)");
    }

    /// A connected client's message → a copy in the team chat, under the brief's header.
    async fn relay_to_team(&mut self, msg: &Message, now: u64) {
        let chat = msg.chat.id;
        if self.chats.by_chat(chat).is_some_and(|l| l.header.is_none()) {
            self.header(chat).await; // it failed when the chat connected: try again first
        }
        let Some(link) = self.chats.by_chat(chat).cloned() else { return };
        match self.tg.api.copy_message(self.tg.team, chat, msg.message_id, link.header).await {
            Ok(copy) => self.chats.update(chat, |l| {
                l.team.push(copy);
                l.seen = now;
            }),
            Err(e) => tracing::warn!(target: "telegram", brief = %link.brief, "a client's message didn't reach the team chat: {e}"),
        }
    }

    /// A message in the team chat: if it replies to one of a brief's messages, a copy to its client.
    async fn team_message(&mut self, msg: &Message, now: u64) {
        let Some(to) = msg.reply_to_message.as_ref().map(|m| m.message_id) else { return };
        let Some(link) = self.chats.by_team_message(to).cloned() else { return };
        match self.tg.api.copy_message(link.chat, self.tg.team, msg.message_id, None).await {
            // The team's reply belongs to the brief too: a reply to it reaches the client as well.
            Ok(_) => self.chats.update(link.chat, |l| {
                l.team.push(msg.message_id);
                l.seen = now;
            }),
            Err(e) => {
                tracing::warn!(target: "telegram", brief = %link.brief, "a team reply didn't reach the client: {e}");
                let why = match &e {
                    ApiError::Refused { code: 403, .. } => "the client has blocked the bot or deleted the chat".to_owned(),
                    other => other.to_string(),
                };
                self.note(&format!("Not delivered to the client of brief {}: {why}.", link.brief), Some(msg.message_id)).await;
            }
        }
    }

    /// Posts the brief's header in the team chat and remembers it.
    async fn header(&mut self, chat: i64) {
        let Some(link) = self.chats.by_chat(chat).cloned() else { return };
        let text = format!(
            "Brief {} continues in Telegram. Reply to this message, or to the client's messages under it, and the bot sends your reply to the client.",
            link.brief
        );
        match self.tg.api.send_message(self.tg.team, &text, None).await {
            Ok(id) => self.chats.update(chat, |l| l.header = Some(id)),
            Err(e) => tracing::warn!(target: "telegram", brief = %link.brief, "the brief's header didn't reach the team chat: {e}"),
        }
    }

    /// One of the bot's own messages to a client, in `lang`.
    async fn say(&self, chat: i64, lang: &str, id: &str, brief: &str) {
        if let Err(e) = self.tg.api.send_message(chat, &self.tg.text(lang, id, brief), None).await {
            tracing::warn!(target: "telegram", "the bot's answer ({id}) didn't reach a client: {e}");
        }
    }

    /// A note for the team.
    async fn note(&self, text: &str, reply_to: Option<i64>) {
        if let Err(e) = self.tg.api.send_message(self.tg.team, text, reply_to).await {
            tracing::warn!(target: "telegram", "a note didn't reach the team chat: {e}");
        }
    }

    /// Forgets links quiet for 30 days and writes the memory if it changed.
    pub async fn save(&mut self, now: u64) {
        let gone = self.chats.prune(now);
        if gone > 0 {
            tracing::info!(target: "telegram", "forgot {gone} Telegram chat link(s) quiet for 30 days");
        }
        let Some((path, bytes)) = self.chats.changes() else { return };
        match tokio::task::spawn_blocking(move || super::chats::write(&path, &bytes)).await {
            Ok(Ok(())) => {}
            Ok(Err(e)) => tracing::error!(target: "telegram", "writing the bot's memory failed: {e}"),
            Err(e) => tracing::error!(target: "telegram", "writing the bot's memory panicked: {e}"),
        }
    }
}

/// `/start b1…` → `("start", Some("b1…"))`; `/stop@mintaka_bot` → `("stop", None)`.
fn command(text: &str) -> Option<(&str, Option<&str>)> {
    let rest = text.strip_prefix('/')?;
    let (head, arg) = match rest.split_once(char::is_whitespace) {
        Some((head, arg)) => (head, Some(arg.trim()).filter(|a| !a.is_empty())),
        None => (rest, None),
    };
    Some((head.split('@').next()?, arg))
}

/// A token bucket per chat: one for connected chats, a smaller one for the rest. In memory only.
#[derive(Default)]
struct Limits {
    chats: HashMap<i64, [(f64, Instant); 2]>,
}

impl Limits {
    fn admit(&mut self, chat: i64, linked: bool, now: Instant) -> bool {
        let refill = |(tokens, at): &mut (f64, Instant), capacity: f64| {
            let hours = now.saturating_duration_since(*at).as_secs_f64() / 3600.0;
            *tokens = (*tokens + hours * capacity).min(capacity);
            *at = now;
        };
        if self.chats.len() >= PRUNE_AT && !self.chats.contains_key(&chat) {
            self.chats.retain(|_, [linked, others]| {
                refill(linked, LINKED_PER_HOUR);
                refill(others, OTHERS_PER_HOUR);
                linked.0 < LINKED_PER_HOUR || others.0 < OTHERS_PER_HOUR
            });
        }
        let buckets = self.chats.entry(chat).or_insert([(LINKED_PER_HOUR, now), (OTHERS_PER_HOUR, now)]);
        let (bucket, capacity) = if linked { (&mut buckets[0], LINKED_PER_HOUR) } else { (&mut buckets[1], OTHERS_PER_HOUR) };
        refill(bucket, capacity);
        if bucket.0 >= 1.0 {
            bucket.0 -= 1.0;
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn commands() {
        assert_eq!(command("/start b1abc"), Some(("start", Some("b1abc"))));
        assert_eq!(command("/start  b1abc \n"), Some(("start", Some("b1abc"))));
        assert_eq!(command("/start"), Some(("start", None)));
        assert_eq!(command("/stop@mintaka_bot"), Some(("stop", None)));
        assert_eq!(command("/start@mintaka_bot b1abc"), Some(("start", Some("b1abc"))));
        assert_eq!(command("hello /start"), None);
        assert_eq!(command(""), None);
    }

    #[test]
    fn a_connected_chat_has_room_and_strangers_get_a_few() {
        let mut limits = Limits::default();
        let t0 = Instant::now();
        assert!((0..5).all(|_| limits.admit(1, false, t0)));
        assert!(!limits.admit(1, false, t0), "a stranger: five, then silence");
        assert!((0..30).all(|_| limits.admit(1, true, t0)), "connected: a bucket of its own");
        assert!(!limits.admit(1, true, t0));
        assert!(limits.admit(1, true, t0 + Duration::from_secs(121)), "one back every two minutes");
        assert!(limits.admit(2, false, t0), "other chats unaffected");
    }
}
