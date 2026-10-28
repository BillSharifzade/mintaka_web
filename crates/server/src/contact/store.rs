//! Encrypted storage of accepted briefs: one `age` file per brief in `{data-dir}/contact/`,
//! mode 0600, encrypted to an X25519 recipient. The server holds only the public key.

use std::fs::{DirBuilder, File, OpenOptions};
use std::io::{self, Write};
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt};
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::time::{SystemTime, UNIX_EPOCH};

use age::secrecy::ExposeSecret;
use age::x25519;
use anyhow::{Context, anyhow};
use proof::StoredBrief;
use rand::RngCore;

/// Where briefs go and who can read them.
pub(crate) struct Store {
    dir: PathBuf,
    recipient: x25519::Recipient,
}

impl Store {
    /// Creates `{data_dir}/contact/` (mode 0700). Without a configured recipient, loads or
    /// creates the development identity `{data_dir}/dev-identity.txt` and warns loudly.
    pub fn open(data_dir: &Path, recipient: Option<&str>) -> anyhow::Result<Store> {
        let dir = data_dir.join("contact");
        DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(&dir)
            .with_context(|| format!("cannot create {} (set --data-dir to a writable directory)", dir.display()))?;
        let recipient = match recipient {
            Some(r) => x25519::Recipient::from_str(r.trim())
                .map_err(|e| anyhow!("--contact-recipient is not an age X25519 recipient (age1…): {e}"))?,
            None => {
                let path = data_dir.join("dev-identity.txt");
                let recipient = dev_identity(&path)?;
                tracing::warn!(
                    "!!! no --contact-recipient: contact briefs are encrypted to a DEVELOPMENT identity \
                     whose secret key is {} on this machine. Never run production like this: \
                     configure the team's age1… public key instead. !!!",
                    path.display()
                );
                recipient
            }
        };
        Ok(Store { dir, recipient })
    }

    /// The public key briefs are encrypted to.
    pub fn recipient(&self) -> String {
        self.recipient.to_string()
    }

    /// Encrypts and writes one brief (blocking: call from `spawn_blocking`). The file is fsynced,
    /// and so is the directory, before this returns.
    pub fn save(&self, brief: &StoredBrief) -> io::Result<PathBuf> {
        let plaintext = serde_json::to_vec(brief)?;
        let ciphertext = age::encrypt(&self.recipient, &plaintext).map_err(io::Error::other)?;
        let path = self.dir.join(format!("{}.json.age", brief.id));
        let mut file = OpenOptions::new().write(true).create_new(true).mode(0o600).open(&path)?;
        file.write_all(&ciphertext)?;
        file.sync_all()?;
        File::open(&self.dir)?.sync_all()?;
        Ok(path)
    }
}

/// A new brief id: `{unix-ms}-{16 hex digits}`.
pub(crate) fn new_id() -> (String, u64) {
    let ms = SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_millis() as u64);
    let mut random = [0u8; 8];
    rand::thread_rng().fill_bytes(&mut random);
    let hex: String = random.iter().map(|b| format!("{b:02x}")).collect();
    (format!("{ms}-{hex}"), ms)
}

/// Loads the development identity, creating it (mode 0600) on first use.
fn dev_identity(path: &Path) -> anyhow::Result<x25519::Recipient> {
    match std::fs::read_to_string(path) {
        Ok(text) => {
            let line = text
                .lines()
                .map(str::trim)
                .find(|l| l.starts_with("AGE-SECRET-KEY-"))
                .ok_or_else(|| anyhow!("{} has no AGE-SECRET-KEY- line", path.display()))?;
            let identity = x25519::Identity::from_str(line)
                .map_err(|e| anyhow!("{}: not an age identity: {e}", path.display()))?;
            Ok(identity.to_public())
        }
        Err(e) if e.kind() == io::ErrorKind::NotFound => {
            let identity = x25519::Identity::generate();
            let recipient = identity.to_public();
            let created = SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs());
            let text = format!(
                "# mintaka-server DEVELOPMENT identity, created at unix time {created}.\n\
                 # Decrypt with: age -d -i {} {{file}}.json.age — never use this key in production.\n\
                 # public key: {recipient}\n{}\n",
                path.display(),
                identity.to_string().expose_secret()
            );
            let mut file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .mode(0o600)
                .open(path)
                .with_context(|| format!("cannot create {}", path.display()))?;
            file.write_all(text.as_bytes())?;
            file.sync_all()?;
            Ok(recipient)
        }
        Err(e) => Err(e).with_context(|| format!("cannot read {}", path.display())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    fn temp_dir(name: &str) -> PathBuf {
        let base = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/tmp");
        let dir = base.join(format!("mintaka-store-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn writes_decryptable_0600_files_with_a_dev_identity() {
        let dir = temp_dir("dev");
        let store = Store::open(&dir, None).unwrap();
        let again = Store::open(&dir, None).unwrap();
        assert_eq!(store.recipient(), again.recipient(), "dev identity is reused");

        let (id, ms) = new_id();
        let brief = StoredBrief {
            id,
            received_at_unix_ms: ms,
            lang: "en".into(),
            kinds: vec![proof::Kind::Ai],
            message: Some("hello".into()),
            timeline: None,
            budget: None,
            name: "Bill".into(),
            email: Some("bill@mintaka.tj".into()),
            telegram: None,
            company: None,
            proof_of_work: true,
        };
        let path = store.save(&brief).unwrap();
        assert_eq!(std::fs::metadata(&path).unwrap().permissions().mode() & 0o777, 0o600);
        assert_eq!(std::fs::metadata(dir.join("dev-identity.txt")).unwrap().permissions().mode() & 0o777, 0o600);

        let identity_text = std::fs::read_to_string(dir.join("dev-identity.txt")).unwrap();
        let key = identity_text.lines().find(|l| l.starts_with("AGE-SECRET-KEY-")).unwrap();
        let identity = x25519::Identity::from_str(key).unwrap();
        let plaintext = age::decrypt(&identity, &std::fs::read(&path).unwrap()).unwrap();
        assert_eq!(serde_json::from_slice::<StoredBrief>(&plaintext).unwrap(), brief);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn rejects_a_bad_recipient() {
        let dir = temp_dir("bad");
        assert!(Store::open(&dir, Some("age1notakey")).is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
