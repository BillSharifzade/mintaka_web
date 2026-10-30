//! The team's side of the contact form: `cargo xtask keygen` makes the age key pair briefs are
//! encrypted to (the server only ever holds the public half, `MINTAKA_CONTACT_RECIPIENT`), and
//! `cargo xtask briefs` decrypts the `*.json.age` files the server wrote and prints them.

use std::fs::OpenOptions;
use std::io::Write as _;
use std::os::unix::fs::OpenOptionsExt as _;
use std::path::Path;
use std::str::FromStr as _;

use age::secrecy::ExposeSecret as _;
use age::x25519;
use anyhow::{Context, Result, anyhow, bail};
use proof::StoredBrief;

/// Writes a new identity to `path` (mode 0600, never over an existing file) and prints its public
/// key, the server's `MINTAKA_CONTACT_RECIPIENT`.
pub fn keygen(path: &Path) -> Result<()> {
    let identity = x25519::Identity::generate();
    let recipient = identity.to_public();
    if let Some(dir) = path.parent().filter(|d| !d.as_os_str().is_empty()) {
        std::fs::create_dir_all(dir).with_context(|| format!("creating {}", dir.display()))?;
    }
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)
        .with_context(|| format!("creating {} (it must not exist yet)", path.display()))?;
    writeln!(
        file,
        "# Mintaka contact briefs: the team's private key. Whoever holds it can read every brief.\n\
         # Keep a copy in a password manager; without it, stored briefs can't be read again.\n\
         # public key: {recipient}\n{}",
        identity.to_string().expose_secret()
    )?;
    println!("{recipient}");
    Ok(())
}

/// Decrypts every `*.json.age` in `dir` (oldest first) with the identity in `identity`.
pub fn read(identity: &Path, dir: &Path) -> Result<()> {
    let text = std::fs::read_to_string(identity).with_context(|| format!("reading {}", identity.display()))?;
    let line = text
        .lines()
        .map(str::trim)
        .find(|l| l.starts_with("AGE-SECRET-KEY-"))
        .ok_or_else(|| anyhow!("{} has no AGE-SECRET-KEY- line", identity.display()))?;
    let key = x25519::Identity::from_str(line).map_err(|e| anyhow!("{}: {e}", identity.display()))?;
    let mut files: Vec<_> = std::fs::read_dir(dir)
        .with_context(|| format!("reading {}", dir.display()))?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.to_string_lossy().ends_with(".json.age"))
        .collect();
    files.sort();
    if files.is_empty() {
        bail!("no *.json.age briefs in {}", dir.display());
    }
    for file in files {
        let plain = age::decrypt(&key, &std::fs::read(&file)?).map_err(|e| anyhow!("{}: {e}", file.display()))?;
        let b: StoredBrief = serde_json::from_slice(&plain).with_context(|| format!("{}: not a brief", file.display()))?;
        print!("{}", show(&b));
    }
    Ok(())
}

fn show(b: &StoredBrief) -> String {
    let opt = |v: &Option<String>| v.clone().unwrap_or_else(|| "—".into());
    format!(
        "── {id} · {lang}{ctf}\n  name      {name}\n  company   {company}\n  email     {email}\n  telegram  {telegram}\n  kinds     {kinds}\n  timeline  {timeline}\n  budget    {budget}\n  message   {message}\n\n",
        id = b.id,
        lang = b.lang,
        ctf = if b.ctf_solved { " · console challenge solved" } else { "" },
        name = b.name,
        company = opt(&b.company),
        email = opt(&b.email),
        telegram = b.telegram.as_deref().map_or_else(|| "—".into(), |t| format!("@{t}")),
        kinds = wire(&b.kinds),
        timeline = wire(&b.timeline),
        budget = wire(&b.budget),
        message = opt(&b.message).replace('\n', "\n            "),
    )
}

/// Choices as their wire names (`ai`, `1-3-months`), lists joined, `—` when absent.
fn wire<T: serde::Serialize>(v: &T) -> String {
    match serde_json::to_value(v) {
        Ok(serde_json::Value::String(s)) => s,
        Ok(serde_json::Value::Array(items)) if !items.is_empty() => {
            items.iter().map(|i| i.as_str().unwrap_or("?").to_owned()).collect::<Vec<_>>().join(", ")
        }
        Ok(serde_json::Value::Null | serde_json::Value::Array(_)) => "—".into(),
        Ok(other) => other.to_string(),
        Err(_) => "?".into(),
    }
}
