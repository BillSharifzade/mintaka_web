//! The ACME cache, `{data_dir}/acme/` (mode 0700): one account per CA directory, one certificate
//! per CA directory and set of names. Every file is mode 0600 and replaced atomically, so a crash
//! mid-write leaves the previous file intact.
//!
//! File names carry a short hash of what they belong to, so switching from staging to
//! production, or changing `--acme-domain`, never serves a certificate made for something else.

use std::fs::{self, DirBuilder, File, OpenOptions, Permissions};
use std::io::{self, Write};
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

/// `account-<hash of the directory URL>.json`: the account's id and private key (JSON from
/// instant-acme's `AccountCredentials`).
pub(crate) fn account_path(cache: &Path, directory: &str) -> PathBuf {
    cache.join(format!("account-{}.json", short_hash(&[directory])))
}

/// `cert-<hash of the directory URL and the sorted names>.pem`: the CA's chain, then our key.
pub(crate) fn certificate_path(cache: &Path, directory: &str, domains: &[String]) -> PathBuf {
    let mut names: Vec<&str> = domains.iter().map(String::as_str).collect();
    names.sort_unstable();
    names.insert(0, directory);
    cache.join(format!("cert-{}.pem", short_hash(&names)))
}

fn short_hash(parts: &[&str]) -> String {
    let mut hash = Sha256::new();
    for part in parts {
        hash.update(part.as_bytes());
        hash.update([0]);
    }
    hash.finalize()[..8].iter().map(|b| format!("{b:02x}")).collect()
}

/// Creates the cache directory (and its parents) and makes it 0700 even if it already existed.
pub(crate) fn create_dir(dir: &Path) -> io::Result<()> {
    DirBuilder::new().recursive(true).mode(0o700).create(dir)?;
    fs::set_permissions(dir, Permissions::from_mode(0o700))
}

/// Reads a cache file; `None` if it does not exist. A file that others could read (copied in by
/// hand, say) is tightened to 0600 first.
pub(crate) fn read(path: &Path) -> io::Result<Option<Vec<u8>>> {
    let meta = match fs::metadata(path) {
        Ok(meta) => meta,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e),
    };
    if meta.permissions().mode() & 0o077 != 0 {
        tracing::warn!(target: "acme", "{} was readable by others; setting it to 0600", path.display());
        fs::set_permissions(path, Permissions::from_mode(0o600))?;
    }
    fs::read(path).map(Some)
}

/// Writes a cache file: a 0600 temporary file in the same directory, fsynced, renamed over the
/// old one, then the directory fsynced so the rename survives a crash.
pub(crate) fn write(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let dir = path.parent().ok_or_else(|| io::Error::other("cache path has no directory"))?;
    let tmp = path.with_extension("tmp");
    match fs::remove_file(&tmp) {
        Err(e) if e.kind() != io::ErrorKind::NotFound => return Err(e),
        _ => {}
    }
    let mut file = OpenOptions::new().write(true).create_new(true).mode(0o600).open(&tmp)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    drop(file);
    fs::rename(&tmp, path)?;
    File::open(dir)?.sync_all()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../target/tmp")
            .join(format!("mintaka-acme-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        dir
    }

    fn mode(path: &Path) -> u32 {
        fs::metadata(path).unwrap().permissions().mode() & 0o777
    }

    #[test]
    fn files_are_private_and_replaced_atomically() {
        let dir = temp_dir("files").join("acme");
        create_dir(&dir).unwrap();
        assert_eq!(mode(&dir), 0o700);
        let path = dir.join("cert-x.pem");
        assert_eq!(read(&path).unwrap(), None);

        write(&path, b"first").unwrap();
        assert_eq!(mode(&path), 0o600);
        write(&path, b"second").unwrap();
        assert_eq!(read(&path).unwrap().as_deref(), Some(&b"second"[..]));
        assert_eq!(mode(&path), 0o600);
        assert!(!path.with_extension("tmp").exists(), "no temporary file left behind");

        // A leftover temporary file from a crash is replaced, not appended to.
        fs::write(path.with_extension("tmp"), b"stale").unwrap();
        write(&path, b"third").unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"third");

        // Copied in with a loose mode: tightened on read.
        fs::set_permissions(&path, Permissions::from_mode(0o644)).unwrap();
        assert!(read(&path).unwrap().is_some());
        assert_eq!(mode(&path), 0o600);

        // An existing, looser directory is tightened too.
        fs::set_permissions(&dir, Permissions::from_mode(0o755)).unwrap();
        create_dir(&dir).unwrap();
        assert_eq!(mode(&dir), 0o700);
    }

    #[test]
    fn names_depend_on_the_ca_and_the_set_of_names_only() {
        let c = Path::new("/c");
        let names = |v: &[&str]| v.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        let a = certificate_path(c, "https://ca/dir", &names(&["a.tj", "b.tj"]));
        assert_eq!(a, certificate_path(c, "https://ca/dir", &names(&["b.tj", "a.tj"])), "order does not matter");
        assert_ne!(a, certificate_path(c, "https://ca/dir", &names(&["a.tj"])));
        assert_ne!(a, certificate_path(c, "https://other/dir", &names(&["a.tj", "b.tj"])));
        assert_ne!(account_path(c, "https://ca/dir"), account_path(c, "https://other/dir"));
        assert!(a.file_name().unwrap().to_str().unwrap().starts_with("cert-"));
        assert_eq!(a.file_name().unwrap().len(), "cert-".len() + 16 + ".pem".len());
    }
}
