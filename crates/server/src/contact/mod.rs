//! The contact form back end (docs/CONTRACTS.md §8): typed fields, rate limits, encrypted
//! storage, optional notification. The HTTP side is `routes::contact`.

mod fields;
mod limit;
mod notify;
mod store;

use std::path::Path;
use std::sync::Arc;

pub(crate) use fields::Brief;
pub(crate) use limit::{ClientNet, RateLimiter};
pub(crate) use notify::Notifier;
pub(crate) use store::{Store, new_id};

use crate::config::TelegramSettings;

/// Everything `POST /api/contact` needs besides the proof of work.
pub(crate) struct Contact {
    /// Shared with the blocking task that encrypts and writes each brief.
    pub store: Arc<Store>,
    pub limiter: RateLimiter,
    pub notifier: Option<Notifier>,
}

impl Contact {
    pub fn new(data_dir: &Path, recipient: Option<&str>, telegram: Option<&TelegramSettings>) -> anyhow::Result<Contact> {
        Ok(Contact {
            store: Arc::new(Store::open(data_dir, recipient)?),
            limiter: RateLimiter::default(),
            notifier: telegram.map(Notifier::new).transpose()?,
        })
    }
}
