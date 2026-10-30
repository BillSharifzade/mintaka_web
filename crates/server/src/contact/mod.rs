//! The contact form back end (docs/CONTRACTS.md §8): typed fields, rate limits, encrypted
//! storage, the optional notification and Telegram bot link (§19). The HTTP side is
//! `routes::contact`.

mod fields;
mod limit;
mod notify;
mod store;

use std::path::Path;
use std::sync::Arc;

pub(crate) use fields::Brief;
pub(crate) use limit::{ClientNet, RateLimiter};
pub(crate) use notify::notification_text;
pub(crate) use store::{Store, new_id};

use crate::telegram::Telegram;

/// Everything `POST /api/contact` needs besides the proof of work.
pub(crate) struct Contact {
    /// Shared with the blocking task that encrypts and writes each brief.
    pub store: Arc<Store>,
    pub limiter: RateLimiter,
    /// The team chat's notifications and the client bot, when Telegram is configured.
    pub telegram: Option<Arc<Telegram>>,
}

impl Contact {
    pub fn new(data_dir: &Path, recipient: Option<&str>, telegram: Option<Arc<Telegram>>) -> anyhow::Result<Contact> {
        Ok(Contact { store: Arc::new(Store::open(data_dir, recipient)?), limiter: RateLimiter::default(), telegram })
    }
}
