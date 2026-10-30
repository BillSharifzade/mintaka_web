//! Secrets the server keeps in its data directory: 32 random bytes per file, mode 0600, created on
//! first start (`ctf.key` for the console challenge, `telegram.key` for the bot's links). Deleting
//! one invalidates everything that was MAC'd with it, and nothing else.

use std::fs::{DirBuilder, OpenOptions};
use std::io::{ErrorKind, Read as _, Write as _};
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt};
use std::path::Path;

use anyhow::Context;
use rand::RngCore;

/// Loads `{data_dir}/{name}`, creating it (and `data_dir`, mode 0700) on first use.
pub(crate) fn load_or_create(data_dir: &Path, name: &str) -> anyhow::Result<[u8; 32]> {
    DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(data_dir)
        .with_context(|| format!("creating {}", data_dir.display()))?;
    let path = data_dir.join(name);
    let mut secret = [0u8; 32];
    match OpenOptions::new().read(true).open(&path) {
        Ok(mut file) => {
            let mut bytes = Vec::new();
            file.read_to_end(&mut bytes).with_context(|| format!("reading {}", path.display()))?;
            secret = bytes
                .try_into()
                .map_err(|_| anyhow::anyhow!("{} must hold exactly 32 bytes", path.display()))?;
        }
        Err(e) if e.kind() == ErrorKind::NotFound => {
            rand::rngs::OsRng.fill_bytes(&mut secret);
            let mut file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .mode(0o600)
                .open(&path)
                .with_context(|| format!("creating {}", path.display()))?;
            file.write_all(&secret).with_context(|| format!("writing {}", path.display()))?;
            file.sync_all()?;
        }
        Err(e) => return Err(e).with_context(|| format!("opening {}", path.display())),
    }
    Ok(secret)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    #[test]
    fn created_once_private_then_reused() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("../../target/tmp/mintaka-secret-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let first = load_or_create(&dir, "a.key").unwrap();
        assert_eq!(load_or_create(&dir, "a.key").unwrap(), first, "reused");
        assert_ne!(load_or_create(&dir, "b.key").unwrap(), first, "one secret per file");
        assert_eq!(std::fs::metadata(dir.join("a.key")).unwrap().permissions().mode() & 0o777, 0o600);
        std::fs::write(dir.join("short.key"), b"too short").unwrap();
        assert!(load_or_create(&dir, "short.key").is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
