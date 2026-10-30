//! `content/site.toml`: site-wide facts. Every value in it is published.

use anyhow::{Context, Result, bail};
use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Clone, Deserialize)]
pub struct SiteConfig {
    /// Canonical origin without a trailing slash, e.g. `https://mintaka.tj`.
    pub origin: String,
    /// URL prefix the site is published under — `/mintaka` for `https://host/mintaka/` — or empty
    /// (the default) at the root of `origin`. `MINTAKA_BASE` overrides it for one build:
    /// `MINTAKA_BASE= cargo xtask build` builds for the root, e.g. a local preview.
    #[serde(default)]
    pub base: String,
    pub default_lang: String,
    /// Languages to build. A language is enabled only when its strings are complete.
    pub langs: Vec<String>,
    pub company: Company,
    pub hosting: Hosting,
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
    /// The A/B test this build runs, if any (spec §12: one at a time).
    #[serde(default)]
    pub experiment: Option<Experiment>,
}

/// `[experiment]`: an A/B test of one page string on the home page of every language (spec §12,
/// docs/CONTRACTS.md §17). The planning assumptions are published together with the sample size
/// they imply. The build computes that number again and refuses a `sample_size` that disagrees,
/// so the stopping point stays the one written down before the test began.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Experiment {
    pub id: String,
    /// The message that varies (`content/{lang}/page.ftl`); the control shows it as it is.
    pub control: String,
    /// The messages the other variants show in its place.
    pub variants: Vec<String>,
    /// The primary metric: an event key of `proof::rum::EVENTS`.
    pub metric: String,
    /// Event keys counted per variant for context, never tested.
    #[serde(default)]
    pub secondary: Vec<String>,
    pub baseline: f64,
    /// Relative: 0.3 means the baseline plus 30 % of it.
    pub mde: f64,
    pub alpha: f64,
    pub power: f64,
    /// Page views per variant (see [`proof::Plan`]).
    pub sample_size: u64,
}

impl Experiment {
    /// Every variant's message id, control first.
    pub fn ids(&self) -> impl Iterator<Item = &str> {
        std::iter::once(self.control.as_str()).chain(self.variants.iter().map(String::as_str))
    }

    /// The events that count as its conversions: the metric, then the secondary ones.
    pub fn metrics(&self) -> impl Iterator<Item = &str> {
        std::iter::once(self.metric.as_str()).chain(self.secondary.iter().map(String::as_str))
    }

    pub fn plan(&self) -> proof::Plan {
        proof::Plan { baseline: self.baseline, mde: self.mde, alpha: self.alpha, power: self.power, sample_size: self.sample_size }
    }

    /// `manifest.experiment`.
    pub fn manifest(&self) -> proof::Experiment {
        proof::Experiment {
            id: self.id.clone(),
            variants: self.ids().map(str::to_owned).collect(),
            metric: self.metric.clone(),
            secondary: self.secondary.clone(),
            plan: self.plan(),
        }
    }

    fn check(&self) -> Result<()> {
        // Ids name files (`en/index.{id}.html`) and appear in beacons.
        let id_like = |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-');
        if !id_like(&self.id) {
            bail!("site.toml: [experiment] id {:?} must be lower-case letters, digits and '-'", self.id);
        }
        let ids: Vec<&str> = self.ids().collect();
        if self.variants.is_empty() || ids.iter().any(|v| !id_like(v)) || (1..ids.len()).any(|i| ids[..i].contains(&ids[i])) {
            bail!("site.toml: [experiment] needs a control and at least one other variant, distinct message ids of lower-case letters, digits and '-'");
        }
        let metrics: Vec<&str> = self.metrics().collect();
        if let Some(m) = metrics.iter().find(|m| !proof::rum::EVENTS.contains(m)) {
            bail!("site.toml: [experiment] metric {m:?} is not an event key (crates/proof/src/rum.rs EVENTS)");
        }
        if (1..metrics.len()).any(|i| metrics[..i].contains(&metrics[i])) {
            bail!("site.toml: [experiment] lists a metric twice");
        }
        let rate = |x: f64| x > 0.0 && x < 1.0;
        if !rate(self.baseline) || !rate(self.alpha) || !rate(self.power) || self.mde.is_nan() || self.mde <= 0.0 || !rate(self.baseline * (1.0 + self.mde)) {
            bail!("site.toml: [experiment] baseline, alpha and power lie strictly between 0 and 1, and mde > 0 keeps baseline · (1 + mde) below 1");
        }
        let required = self.plan().required(ids.len());
        if self.sample_size != required {
            bail!(
                "site.toml: [experiment] sample_size = {} but baseline {}, mde {}, alpha {} and power {} need {required} page views per variant for {} variants — set sample_size = {required}, or keep the plan the test started with",
                self.sample_size, self.baseline, self.mde, self.alpha, self.power, ids.len()
            );
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct Lab {
    /// The public repository, e.g. `https://github.com/BillSharifzade/mintaka_web`.
    pub repo: String,
    /// The branch the site is built from: code links point at it.
    pub branch: String,
}

/// Where the site is served from (the privacy page says so).
#[derive(Debug, Clone, Deserialize)]
pub struct Hosting {
    /// The company running the machine, e.g. `Koinot Innovations and Technologies`.
    pub provider: String,
    pub city: String,
    pub country: String,
    /// A proxy of the provider ends visitors' HTTPS before our server (and sees their address).
    #[serde(default)]
    pub edge: bool,
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
    /// Each detail is optional (empty = not published), but at least one of email, Telegram and
    /// phone must be set.
    #[serde(default)]
    pub email: String,
    /// Handle without `@`.
    #[serde(default)]
    pub telegram: String,
    /// As people write it, e.g. `+992 98 544 70 72`; the `tel:` link drops the spaces.
    #[serde(default)]
    pub phone: String,
    /// Applications; without it, the team section points to the contact form.
    #[serde(default)]
    pub careers_email: String,
    /// security.txt; without it, the first of email and phone.
    #[serde(default)]
    pub security_email: String,
    /// Shown on the success screen and in the copy: how fast we reply.
    pub reply_within: String,
}

impl Contact {
    /// The ways to reach us directly, in display order, as `(href, text)`.
    pub fn direct(&self) -> Vec<(String, String)> {
        let mut out = vec![];
        if !self.email.is_empty() {
            out.push((format!("mailto:{}", self.email), self.email.clone()));
        }
        if !self.telegram.is_empty() {
            out.push((format!("https://t.me/{}", self.telegram), format!("@{}", self.telegram)));
        }
        if !self.phone.is_empty() {
            out.push((format!("tel:{}", self.phone.replace(' ', "")), self.phone.clone()));
        }
        out
    }

    /// The first of them: what the privacy page, humans.txt and curl mode name.
    pub fn primary(&self) -> (String, String) {
        self.direct().into_iter().next().unwrap_or_default()
    }

    /// The `Contact:` URI of security.txt (RFC 9116 §2.5.3).
    pub fn security_uri(&self) -> String {
        match [&self.security_email, &self.email].into_iter().find(|e| !e.is_empty()) {
            Some(email) => format!("mailto:{email}"),
            None if !self.phone.is_empty() => format!("tel:{}", self.phone.replace(' ', "")),
            None => String::new(),
        }
    }

    /// Where "write to us about joining" goes, and what it shows.
    pub fn careers(&self) -> (String, String) {
        if self.careers_email.is_empty() { self.primary() } else { (format!("mailto:{}", self.careers_email), self.careers_email.clone()) }
    }
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
        let mut cfg: SiteConfig = toml::from_str(&text).with_context(|| format!("parsing {}", path.display()))?;
        if cfg.origin.ends_with('/') {
            bail!("site.toml: origin must not end with '/'");
        }
        if let Ok(base) = std::env::var("MINTAKA_BASE") {
            cfg.base = base;
        }
        if cfg.contact.direct().is_empty() {
            bail!("site.toml: [contact] needs at least one of email, telegram and phone");
        }
        if cfg.contact.security_uri().is_empty() {
            bail!("site.toml: security.txt needs an email or a phone: set security_email, email or phone");
        }
        if !cfg.contact.phone.is_empty() && !cfg.contact.phone.trim_start_matches('+').bytes().all(|b| b.is_ascii_digit() || b == b' ') {
            bail!("site.toml: phone {:?}: digits and spaces, optionally after +", cfg.contact.phone);
        }
        if !is_base(&cfg.base) {
            bail!("site.toml: base {:?} must be empty or like /mintaka (no trailing '/')", cfg.base);
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
        if let Some(experiment) = &cfg.experiment {
            experiment.check()?;
        }
        Ok(cfg)
    }

    /// A path of this site as a link: `/en/` → `/mintaka/en/` under a URL prefix.
    pub fn url(&self, path: &str) -> String {
        format!("{}{path}", self.base)
    }

    /// The same link, absolute: `https://host/mintaka/en/`.
    pub fn abs(&self, path: &str) -> String {
        format!("{}{}{path}", self.origin, self.base)
    }

    /// Claims of a group that may be shown in this build.
    pub fn claims<'a>(&'a self, group: &'a str, release: bool) -> impl Iterator<Item = &'a Claim> + 'a {
        self.claims.iter().filter(move |c| c.group == group && (c.verified || !release))
    }
}

/// `` or `/mintaka`, `/a/b`: segments of letters, digits, `-`, `_` and `.` (the server checks the
/// same rule when it loads the manifest).
fn is_base(s: &str) -> bool {
    s.is_empty()
        || s.strip_prefix('/').is_some_and(|rest| {
            rest.split('/').all(|seg| !seg.is_empty() && seg != "." && seg != ".." && seg.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.')))
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn headline() -> Experiment {
        Experiment {
            id: "hero-headline".into(),
            control: "hero-title".into(),
            variants: vec!["hero-title-hardware".into(), "hero-title-cycle".into()],
            metric: "cta_start_project".into(),
            secondary: vec!["contact_submit".into()],
            baseline: 0.05,
            mde: 0.3,
            alpha: 0.05,
            power: 0.8,
            sample_size: 4578,
        }
    }

    #[test]
    fn the_published_sample_size_is_the_one_the_plan_implies() {
        assert!(headline().check().is_ok());
        let err = Experiment { sample_size: 4000, ..headline() }.check().unwrap_err().to_string();
        assert!(err.contains("sample_size = 4000") && err.contains("need 4578 page views per variant"), "{err}");
        // One alternative: a single comparison keeps all of alpha, so fewer page views do.
        assert!(Experiment { variants: vec!["hero-title-hardware".into()], sample_size: 3780, ..headline() }.check().is_ok());
        let manifest = headline().manifest();
        assert_eq!(manifest.variants, ["hero-title", "hero-title-hardware", "hero-title-cycle"]);
        assert_eq!(manifest.plan.sample_size, 4578);
    }

    #[test]
    fn a_malformed_test_fails_the_build() {
        for (bad, why) in [
            (Experiment { metric: "page_view".into(), ..headline() }, "not an event"),
            (Experiment { secondary: vec!["cta_start_project".into()], ..headline() }, "a metric twice"),
            (Experiment { variants: vec![], ..headline() }, "nothing to compare"),
            (Experiment { variants: vec!["hero-title".into()], ..headline() }, "the control again"),
            (Experiment { variants: vec!["hero_title_b".into()], ..headline() }, "an id that can't name a file"),
            (Experiment { id: "Hero headline".into(), ..headline() }, "the test's id"),
            (Experiment { mde: 30.0, ..headline() }, "a lift past 100 %"),
            (Experiment { power: 1.0, ..headline() }, "certainty"),
            (Experiment { alpha: 0.0, ..headline() }, "no false positives at all"),
        ] {
            assert!(bad.check().is_err(), "{why}");
        }
    }

    #[test]
    fn the_repository_plan_is_consistent() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let cfg = SiteConfig::load(&root).unwrap();
        if let Some(e) = &cfg.experiment {
            assert_eq!(e.sample_size, e.plan().required(e.variants.len() + 1));
        }
    }
}
