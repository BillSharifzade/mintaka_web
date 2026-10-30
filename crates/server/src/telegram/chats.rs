//! The bot's memory (docs/CONTRACTS.md §19): which Telegram chat belongs to which brief, so a reply
//! from the team finds its way back hours or days later, across restarts. One JSON file,
//! `{data-dir}/telegram.json`, mode 0600, replaced atomically (the same write as the ACME cache).
//! A link is forgotten 30 days after its last message either way, or at once on `/stop`.
//!
//! What it holds: chat ids, brief ids and languages, message numbers in the team chat, times, and
//! the polling offset. Never a message, a name or a username.

use std::fs::{self, Permissions};
use std::io;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use anyhow::Context;
use serde::{Deserialize, Serialize};

/// A link is kept this long after its last message, in seconds.
pub(crate) const KEEP_SECS: u64 = 30 * 24 * 3600;
/// Most links at once; beyond it, no new chat is connected (and the log says so).
pub(crate) const MAX_LINKS: usize = 10_000;
/// Team-chat messages remembered per link, newest kept.
const MAX_TEAM: usize = 200;
const VERSION: u32 = 1;

#[derive(Serialize, Deserialize)]
struct Stored {
    version: u32,
    /// The next `getUpdates` offset: everything before it has been handled.
    offset: i64,
    links: Vec<Link>,
}

/// One chat connected to one brief.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct Link {
    pub chat: i64,
    pub brief: String,
    /// The brief's language: the bot answers in it.
    pub lang: String,
    /// Unix seconds: when `/start` connected it, and the last message in either direction.
    pub linked: u64,
    pub seen: u64,
    /// The team chat's message announcing it; the copies of the client's messages reply to it.
    pub header: Option<i64>,
    /// The team chat's messages that belong to it (copies, and the team's replies), newest last.
    pub team: Vec<i64>,
}

pub(crate) struct Chats {
    path: PathBuf,
    offset: i64,
    links: Vec<Link>,
    /// Changed since the last save.
    dirty: bool,
}

impl Chats {
    /// Loads `{data_dir}/telegram.json`, or starts empty. A file others could read is set back to
    /// 0600; one that can't be read as this shape stops the server (move it away to start over).
    pub fn open(data_dir: &Path) -> anyhow::Result<Chats> {
        let path = data_dir.join("telegram.json");
        let stored = match fs::metadata(&path) {
            Ok(meta) => {
                if meta.permissions().mode() & 0o077 != 0 {
                    tracing::warn!(target: "telegram", "{} was readable by others; setting it to 0600", path.display());
                    fs::set_permissions(&path, Permissions::from_mode(0o600))?;
                }
                let bytes = fs::read(&path).with_context(|| format!("reading {}", path.display()))?;
                let stored: Stored = serde_json::from_slice(&bytes)
                    .with_context(|| format!("{} is not the bot's memory (move it away to start over)", path.display()))?;
                anyhow::ensure!(stored.version == VERSION, "{}: version {} is not {VERSION}", path.display(), stored.version);
                stored
            }
            Err(e) if e.kind() == io::ErrorKind::NotFound => Stored { version: VERSION, offset: 0, links: Vec::new() },
            Err(e) => return Err(e).with_context(|| format!("reading {}", path.display())),
        };
        Ok(Chats { path, offset: stored.offset, links: stored.links, dirty: false })
    }

    pub fn offset(&self) -> i64 {
        self.offset
    }

    pub fn set_offset(&mut self, offset: i64) {
        if offset > self.offset {
            self.offset = offset;
            self.dirty = true;
        }
    }

    pub fn by_chat(&self, chat: i64) -> Option<&Link> {
        self.links.iter().find(|l| l.chat == chat)
    }

    pub fn by_brief(&self, brief: &str) -> Option<&Link> {
        self.links.iter().find(|l| l.brief == brief)
    }

    /// The link a team-chat message belongs to: its header, a copy, or a reply sent on.
    pub fn by_team_message(&self, message: i64) -> Option<&Link> {
        self.links.iter().find(|l| l.header == Some(message) || l.team.contains(&message))
    }

    /// Connects a chat, replacing whatever it was connected to; `false` when full.
    pub fn connect(&mut self, link: Link) -> bool {
        self.links.retain(|l| l.chat != link.chat);
        if self.links.len() >= MAX_LINKS {
            return false;
        }
        self.links.push(link);
        self.dirty = true;
        true
    }

    /// Changes a chat's link, if it has one.
    pub fn update(&mut self, chat: i64, change: impl FnOnce(&mut Link)) {
        if let Some(link) = self.links.iter_mut().find(|l| l.chat == chat) {
            change(link);
            if link.team.len() > MAX_TEAM {
                let extra = link.team.len() - MAX_TEAM;
                link.team.drain(..extra);
            }
            self.dirty = true;
        }
    }

    /// Forgets a chat's link (`/stop`).
    pub fn forget(&mut self, chat: i64) -> Option<Link> {
        let at = self.links.iter().position(|l| l.chat == chat)?;
        self.dirty = true;
        Some(self.links.remove(at))
    }

    /// Forgets every link quiet for longer than [`KEEP_SECS`]; returns how many.
    pub fn prune(&mut self, now: u64) -> usize {
        let before = self.links.len();
        self.links.retain(|l| now.saturating_sub(l.seen) <= KEEP_SECS);
        let gone = before - self.links.len();
        self.dirty |= gone > 0;
        gone
    }

    /// What to write, if anything changed since the last call: the file's path and bytes.
    pub fn changes(&mut self) -> Option<(PathBuf, Vec<u8>)> {
        if !self.dirty {
            return None;
        }
        let stored = Stored { version: VERSION, offset: self.offset, links: self.links.clone() };
        let bytes = serde_json::to_vec(&stored).ok()?;
        self.dirty = false;
        Some((self.path.clone(), bytes))
    }
}

/// Writes the bot's memory: a 0600 temporary file, fsynced, renamed over the old one.
pub(crate) fn write(path: &Path, bytes: &[u8]) -> io::Result<()> {
    crate::acme::cache::write(path, bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("../../target/tmp/mintaka-chats-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn link(chat: i64, brief: &str, seen: u64) -> Link {
        Link { chat, brief: brief.into(), lang: "en".into(), linked: seen, seen, header: Some(chat * 10), team: vec![] }
    }

    fn save(chats: &mut Chats) {
        let (path, bytes) = chats.changes().expect("something to save");
        write(&path, &bytes).unwrap();
    }

    #[test]
    fn links_survive_a_restart_in_a_private_file() {
        let dir = temp_dir("restart");
        let mut chats = Chats::open(&dir).unwrap();
        assert_eq!((chats.offset(), chats.changes().is_none()), (0, true), "empty, nothing to write");
        assert!(chats.connect(link(1, "b-1", 100)));
        chats.update(1, |l| l.team.push(42));
        chats.set_offset(9);
        save(&mut chats);
        assert!(chats.changes().is_none(), "saved");
        let path = dir.join("telegram.json");
        assert_eq!(fs::metadata(&path).unwrap().permissions().mode() & 0o777, 0o600);

        let again = Chats::open(&dir).unwrap();
        assert_eq!(again.offset(), 9);
        assert_eq!(again.by_chat(1).unwrap().brief, "b-1");
        assert_eq!(again.by_team_message(42).unwrap().chat, 1, "a copy");
        assert_eq!(again.by_team_message(10).unwrap().chat, 1, "the header");
        assert!(again.by_team_message(43).is_none());
        let text = fs::read_to_string(&path).unwrap();
        assert!(!text.contains("text") && !text.contains("name"), "{text}");
    }

    #[test]
    fn one_brief_per_chat_forgotten_on_stop_or_after_30_days() {
        let dir = temp_dir("forget");
        let mut chats = Chats::open(&dir).unwrap();
        chats.connect(link(1, "b-1", 0));
        chats.connect(link(1, "b-2", 0));
        assert_eq!(chats.by_chat(1).unwrap().brief, "b-2", "a new /start replaces the chat's link");
        assert!(chats.by_brief("b-1").is_none());
        chats.connect(link(2, "b-3", 0));
        assert_eq!(chats.forget(2).unwrap().brief, "b-3");
        assert!(chats.forget(2).is_none());
        chats.update(1, |l| l.seen = 1000);
        assert_eq!(chats.prune(1000 + KEEP_SECS), 0, "exactly 30 days: kept");
        assert_eq!(chats.prune(1001 + KEEP_SECS), 1);
        assert!(chats.by_chat(1).is_none());
        chats.set_offset(5);
        chats.set_offset(3);
        assert_eq!(chats.offset(), 5, "the offset only moves forward");
    }

    #[test]
    fn team_messages_are_bounded_and_the_file_must_be_ours() {
        let dir = temp_dir("bounds");
        let mut chats = Chats::open(&dir).unwrap();
        chats.connect(link(1, "b-1", 0));
        chats.update(1, |l| l.team.extend(0..250));
        let team = &chats.by_chat(1).unwrap().team;
        assert_eq!((team.len(), team[0]), (MAX_TEAM, 50), "the newest kept");
        fs::write(dir.join("telegram.json"), b"{\"not\":\"ours\"}").unwrap();
        let refused = Chats::open(&dir).err().expect("not the bot's memory");
        assert!(refused.to_string().contains("move it away"), "{refused}");
        fs::write(dir.join("telegram.json"), b"{\"version\":1,\"offset\":0,\"links\":[]}").unwrap();
        fs::set_permissions(dir.join("telegram.json"), Permissions::from_mode(0o644)).unwrap();
        Chats::open(&dir).unwrap();
        assert_eq!(fs::metadata(dir.join("telegram.json")).unwrap().permissions().mode() & 0o777, 0o600, "tightened");
    }
}
