//! `content/site.toml`: site-wide facts. Every value in it is published.

use anyhow::{Context, Result, bail};
use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Clone, Deserialize)]
pub struct SiteConfig {
    /// Canonical origin without a trailing slash, e.g. `https://mintaka.tj`.
    pub origin: String,
    pub default_lang: String,
    /// Languages to build. A language is enabled only when its strings are complete.
    pub langs: Vec<String>,
    pub company: Company,
    pub contact: Contact,
    /// Statements that must be true on launch day (see [`Claim`]).
    #[serde(default)]
    pub claims: Vec<Claim>,
    /// The S12 team grid, in display order.
    #[serde(default)]
    pub team: Vec<Member>,
    /// Set to true only when the domain and every contact detail above are final.
    #[serde(default)]
    pub launch_ready: bool,
    /// Where the Lab's code links point (spec §S11: a reproducible repository for every post).
    pub lab: Lab,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Lab {
    /// The public repository, e.g. `https://github.com/BillSharifzade/mintaka_web`.
    pub repo: String,
    /// The branch the site is built from: code links point at it.
    pub branch: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Company {
    pub name: String,
    pub city: String,
    pub country: String,
    pub lat: f64,
    pub lon: f64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Contact {
    pub email: String,
    pub telegram: String,
    #[serde(default)]
    pub phone: String,
    pub careers_email: String,
    pub security_email: String,
    /// Shown on the success screen and in the copy: how fast we reply.
    pub reply_within: String,
}

/// A claim about Mintaka's own practice ("hardware security keys on every account"). Its text
/// lives in the strings (`content/{lang}/page.ftl`, id = `key`). Unverified claims are shown in
/// development builds with a visible UNVERIFIED mark and are dropped from release builds — a
/// security company caught overstating its own security loses everything (spec §S5).
#[derive(Debug, Clone, Deserialize)]
pub struct Claim {
    pub group: String,
    pub key: String,
    #[serde(default)]
    pub verified: bool,
}

/// A person in the S12 team grid. Name, role and the one line are strings (`team-<id>-name`,
/// `team-<id>-role`, `team-<id>-line`) because they are written per language; the portrait is
/// `assets/team/<id>.avif` with a `.webp` fallback.
#[derive(Debug, Clone, Deserialize)]
pub struct Member {
    pub id: String,
    /// GitHub user name.
    pub github: String,
    /// Personal site (https), if any.
    #[serde(default)]
    pub site: String,
}

impl SiteConfig {
    pub fn load(root: &Path) -> Result<Self> {
        let path = root.join("content/site.toml");
        let text = std::fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
        let cfg: SiteConfig = toml::from_str(&text).with_context(|| format!("parsing {}", path.display()))?;
        if cfg.origin.ends_with('/') {
            bail!("site.toml: origin must not end with '/'");
        }
        if !cfg.lab.repo.starts_with("https://") || cfg.lab.branch.is_empty() {
            bail!("site.toml: [lab] needs an https:// repo and a branch");
        }
        if !cfg.langs.contains(&cfg.default_lang) {
            bail!("site.toml: default_lang {:?} is not in langs", cfg.default_lang);
        }
        for m in &cfg.team {
            // The id names files and string ids; the GitHub name goes into a URL path.
            if m.id.is_empty() || !m.id.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-') {
                bail!("site.toml: team id {:?} must be lower-case letters, digits and '-'", m.id);
            }
            if m.github.is_empty() || !m.github.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-') {
                bail!("site.toml: team {:?} has an invalid GitHub name {:?}", m.id, m.github);
            }
            if !m.site.is_empty() && !m.site.starts_with("https://") {
                bail!("site.toml: team {:?} site must be an https:// URL", m.id);
            }
        }
        Ok(cfg)
    }

    /// Claims of a group that may be shown in this build.
    pub fn claims<'a>(&'a self, group: &'a str, release: bool) -> impl Iterator<Item = &'a Claim> + 'a {
        self.claims.iter().filter(move |c| c.group == group && (c.verified || !release))
    }
}
