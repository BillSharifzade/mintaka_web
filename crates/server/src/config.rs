//! Command-line interface and the validated configuration the server runs with.

use std::fmt;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::time::Duration;

use anyhow::{Context, bail};
use clap::Parser;
use proof::Location;

/// Default TLS listener when a certificate source is given without `--tls-listen`.
const DEFAULT_TLS_LISTEN: &str = "127.0.0.1:8443";

/// `mintaka-server` command line.
#[derive(Debug, Parser)]
#[command(name = "mintaka-server", version, about = "The mintaka.tj web server")]
pub struct Cli {
    /// Site generator output: `manifest.json` and every file it lists.
    #[arg(long, default_value = "dist")]
    pub dist: PathBuf,

    /// Plain-HTTP listener (development and local health checks).
    #[arg(long, default_value = "127.0.0.1:8080")]
    pub listen: SocketAddr,

    /// TLS listener. Defaults to 127.0.0.1:8443 when a certificate source is given.
    #[arg(long, value_name = "ADDR")]
    pub tls_listen: Option<SocketAddr>,

    /// PEM certificate chain, leaf first.
    #[arg(long, value_name = "PEM", requires = "tls_key")]
    pub tls_cert: Option<PathBuf>,

    /// PEM private key (PKCS#8, PKCS#1 or SEC1).
    #[arg(long, value_name = "PEM", requires = "tls_cert")]
    pub tls_key: Option<PathBuf>,

    /// Generate a self-signed certificate for localhost, 127.0.0.1 and ::1 at startup.
    #[arg(long, conflicts_with_all = ["tls_cert", "tls_key"])]
    pub tls_self_signed: bool,

    /// Also accept TLS 1.2 (default: TLS 1.3 only).
    #[arg(long)]
    pub tls12: bool,

    /// Get and renew the certificate from an ACME CA (Let's Encrypt) for this name; repeat for
    /// more names. Validated with TLS-ALPN-01 on the TLS listener, which must be reachable on
    /// port 443. The account key and certificates are cached in {data-dir}/acme.
    #[arg(long, value_name = "DOMAIN", conflicts_with_all = ["tls_cert", "tls_key", "tls_self_signed"])]
    pub acme_domain: Vec<String>,

    /// ACME account contact, e.g. mailto:ops@mintaka.tj.
    #[arg(long, value_name = "MAILTO")]
    pub acme_contact: Option<String>,

    /// ACME directory: production or staging (Let's Encrypt), or a directory URL [default: production].
    #[arg(long, value_name = "DIRECTORY")]
    pub acme_directory: Option<String>,

    /// PEM CA certificate to trust for the ACME directory's HTTPS instead of the system roots
    /// (Pebble or a private ACME CA).
    #[arg(long, value_name = "PEM")]
    pub acme_ca_root: Option<PathBuf>,

    /// Also serve HTTP/3 (QUIC) on UDP at the TLS listener's address, announced with Alt-Svc.
    #[arg(long)]
    pub http3: bool,

    /// UDP port announced in Alt-Svc [default: the port HTTP/3 listens on] — for containers or
    /// NAT that map the public port 443 onto another one.
    #[arg(long, value_name = "PORT", requires = "http3")]
    pub http3_advertise_port: Option<u16>,

    /// Answer plain HTTP on this address with a 301 to https:// on the same host.
    #[arg(long, value_name = "ADDR")]
    pub redirect_http: Option<SocketAddr>,

    /// Deployment name reported by /_proof and /_telemetry.
    #[arg(long, default_value = "dev")]
    pub region: String,

    /// Where the machine is, reported by /_proof: "City,CC,lat,lon".
    #[arg(long, value_name = "CITY,CC,LAT,LON", value_parser = parse_location)]
    pub location: Option<Location>,

    /// Writable directory for encrypted contact briefs (and the development identity).
    #[arg(long, default_value = "data")]
    pub data_dir: PathBuf,

    /// age X25519 recipient (age1…) that contact briefs are encrypted to.
    #[arg(long, env = "MINTAKA_CONTACT_RECIPIENT", value_name = "AGE1…")]
    pub contact_recipient: Option<String>,

    /// Leading zero bits required by the contact form's proof of work.
    #[arg(long, default_value_t = 18, value_parser = clap::value_parser!(u8).range(1..=32))]
    pub pow_difficulty: u8,
}

/// Everything [`crate::Server::bind`] needs. Build it with [`Config::from_cli`], or directly
/// (tests do).
#[derive(Debug, Clone)]
pub struct Config {
    /// Directory with `manifest.json` and the files it lists.
    pub dist: PathBuf,
    /// Plain-HTTP listener; `None` disables it.
    pub listen: Option<SocketAddr>,
    /// TLS listener and certificate.
    pub tls: Option<TlsSettings>,
    /// HTTP → HTTPS redirect listener (requires `tls`).
    pub redirect_http: Option<SocketAddr>,
    /// Deployment name for `/_proof` and `/_telemetry`.
    pub region: String,
    /// Where the machine is, for `/_proof`.
    pub location: Option<Location>,
    /// Writable directory for contact briefs.
    pub data_dir: PathBuf,
    /// `age1…`; `None` means a development identity in `{data_dir}/dev-identity.txt`.
    pub contact_recipient: Option<String>,
    /// Leading zero bits the contact form's proof of work must reach.
    pub pow_difficulty: u8,
    /// Optional new-brief notifications.
    pub telegram: Option<TelegramSettings>,
    /// Concurrent `/_telemetry` streams per process; more get `503`.
    pub max_telemetry_streams: usize,
    /// How long shutdown waits for in-flight requests.
    pub shutdown_grace: Duration,
}

/// The TLS listener.
#[derive(Debug, Clone)]
pub struct TlsSettings {
    /// Address to accept TLS connections on.
    pub listen: SocketAddr,
    /// Where the certificate comes from.
    pub source: CertSource,
    /// Accept TLS 1.2 in addition to TLS 1.3.
    pub allow_tls12: bool,
    /// HTTP/3 on UDP at the same address; `None` disables it.
    pub http3: Option<Http3Settings>,
}

/// HTTP/3 over QUIC, next to the TLS listener.
#[derive(Debug, Clone, Default)]
pub struct Http3Settings {
    /// Port announced in `Alt-Svc`; `None` announces the UDP port actually bound.
    pub advertise_port: Option<u16>,
}

/// Where the certificate comes from.
#[derive(Debug, Clone)]
pub enum CertSource {
    /// PEM files on disk.
    Pem {
        /// Certificate chain, leaf first.
        cert: PathBuf,
        /// Private key (PKCS#8, PKCS#1 or SEC1).
        key: PathBuf,
    },
    /// Generated at startup for `localhost`, `127.0.0.1` and `::1` (development).
    SelfSigned,
    /// Issued and renewed by the server itself over ACME (RFC 8555), validated with
    /// TLS-ALPN-01 (RFC 8737).
    Acme(AcmeSettings),
}

/// Let's Encrypt production directory.
pub const LETS_ENCRYPT_PRODUCTION: &str = "https://acme-v02.api.letsencrypt.org/directory";
/// Let's Encrypt staging directory (untrusted certificates, generous rate limits).
pub const LETS_ENCRYPT_STAGING: &str = "https://acme-staging-v02.api.letsencrypt.org/directory";

/// The ACME certificate source.
#[derive(Debug, Clone)]
pub struct AcmeSettings {
    /// Names on the certificate: lower-case, no trailing dot, no wildcards, de-duplicated.
    pub domains: Vec<String>,
    /// `mailto:` contact for the account.
    pub contact: Option<String>,
    /// Directory URL of the CA.
    pub directory: String,
    /// CA certificate to trust for the directory's HTTPS instead of the system roots.
    pub ca_root: Option<PathBuf>,
    /// Account key and certificates, mode 0600 in a 0700 directory (`{data_dir}/acme`).
    pub cache: PathBuf,
}

impl AcmeSettings {
    /// Where the account (its id and private key) for this directory is cached.
    pub fn account_path(&self) -> PathBuf {
        crate::acme::cache::account_path(&self.cache, &self.directory)
    }

    /// Where the certificate chain and key for this directory and these names are cached.
    pub fn certificate_path(&self) -> PathBuf {
        crate::acme::cache::certificate_path(&self.cache, &self.directory, &self.domains)
    }
}

/// Telegram bot notification target (`MINTAKA_TELEGRAM_BOT_TOKEN`, `MINTAKA_TELEGRAM_CHAT_ID`).
#[derive(Clone)]
pub struct TelegramSettings {
    /// Bot API token (secret: never logged or printed).
    pub bot_token: String,
    /// Chat (or channel) the bot posts to.
    pub chat_id: String,
}

impl fmt::Debug for TelegramSettings {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TelegramSettings")
            .field("bot_token", &"<redacted>")
            .field("chat_id", &self.chat_id)
            .finish()
    }
}

impl Config {
    /// Validates the command line and reads the Telegram settings from the environment.
    pub fn from_cli(cli: Cli) -> anyhow::Result<Config> {
        let acme = !cli.acme_domain.is_empty();
        // (clap's `requires` does not see a repeatable flag, so this is checked here.)
        if !acme && (cli.acme_contact.is_some() || cli.acme_directory.is_some() || cli.acme_ca_root.is_some()) {
            bail!("--acme-contact, --acme-directory and --acme-ca-root need --acme-domain");
        }
        let source = match (cli.tls_cert, cli.tls_key, cli.tls_self_signed, acme) {
            (Some(cert), Some(key), false, false) => Some(CertSource::Pem { cert, key }),
            (None, None, true, false) => Some(CertSource::SelfSigned),
            (None, None, false, true) => Some(CertSource::Acme(AcmeSettings {
                domains: acme_domains(&cli.acme_domain)?,
                contact: cli.acme_contact.as_deref().map(acme_contact).transpose()?,
                directory: acme_directory(cli.acme_directory.as_deref().unwrap_or("production"))?,
                ca_root: cli.acme_ca_root,
                cache: cli.data_dir.join("acme"),
            })),
            (None, None, false, false) => None,
            _ => bail!("use one certificate source: --tls-cert with --tls-key, --tls-self-signed, or --acme-domain"),
        };
        let http3 = cli.http3.then_some(Http3Settings { advertise_port: cli.http3_advertise_port });
        if cli.http3_advertise_port == Some(0) {
            bail!("--http3-advertise-port must be a port number (1-65535)");
        }
        let tls = match (source, cli.tls_listen) {
            (Some(source), listen) => {
                let listen = match listen {
                    Some(addr) => addr,
                    None => DEFAULT_TLS_LISTEN.parse().context("default TLS address")?,
                };
                Some(TlsSettings { listen, source, allow_tls12: cli.tls12, http3 })
            }
            (None, Some(_)) => bail!("--tls-listen needs a certificate source (--tls-cert/--tls-key, --tls-self-signed or --acme-domain)"),
            (None, None) if cli.tls12 => bail!("--tls12 needs a TLS listener"),
            (None, None) if http3.is_some() => bail!("--http3 needs a TLS listener (QUIC is always encrypted)"),
            (None, None) => None,
        };
        if cli.redirect_http.is_some() && tls.is_none() {
            bail!("--redirect-http needs a TLS listener to redirect to");
        }
        let contact_recipient = cli.contact_recipient.filter(|r| !r.trim().is_empty());
        Ok(Config {
            dist: cli.dist,
            listen: Some(cli.listen),
            tls,
            redirect_http: cli.redirect_http,
            region: cli.region,
            location: cli.location,
            data_dir: cli.data_dir,
            contact_recipient,
            pow_difficulty: cli.pow_difficulty,
            telegram: telegram_from_env()?,
            max_telemetry_streams: 64,
            shutdown_grace: Duration::from_secs(10),
        })
    }
}

/// Reads the Telegram bot settings. Only from the environment: a token on the command line
/// would be visible to every user of the machine in the process list.
fn telegram_from_env() -> anyhow::Result<Option<TelegramSettings>> {
    let get = |name: &str| std::env::var(name).ok().filter(|v| !v.trim().is_empty());
    match (get("MINTAKA_TELEGRAM_BOT_TOKEN"), get("MINTAKA_TELEGRAM_CHAT_ID")) {
        (Some(bot_token), Some(chat_id)) => {
            let token_ok = bot_token
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b':' | b'_' | b'-'));
            if !token_ok {
                bail!("MINTAKA_TELEGRAM_BOT_TOKEN has characters a bot token cannot contain");
            }
            Ok(Some(TelegramSettings { bot_token, chat_id: chat_id.trim().to_owned() }))
        }
        (None, None) => Ok(None),
        _ => bail!("set both MINTAKA_TELEGRAM_BOT_TOKEN and MINTAKA_TELEGRAM_CHAT_ID, or neither"),
    }
}

/// `--acme-domain` values as the CA will see them: lower-case DNS names (A-labels), no trailing
/// dot, in order, without duplicates.
fn acme_domains(raw: &[String]) -> anyhow::Result<Vec<String>> {
    let mut domains: Vec<String> = Vec::with_capacity(raw.len());
    for value in raw {
        let name = value.trim().trim_end_matches('.').to_ascii_lowercase();
        if name.starts_with("*.") {
            // RFC 8737 §3: TLS-ALPN-01 cannot prove control of a wildcard; that needs DNS-01.
            bail!("--acme-domain {value:?}: wildcard names cannot be validated with TLS-ALPN-01");
        }
        if !name.is_ascii() {
            bail!("--acme-domain {value:?}: write internationalised names in their A-label (xn--…) form");
        }
        if name.parse::<std::net::IpAddr>().is_ok() {
            bail!("--acme-domain {value:?}: an IP address is not a domain name");
        }
        let label_ok = |l: &str| {
            !l.is_empty()
                && l.len() <= 63
                && !l.starts_with('-')
                && !l.ends_with('-')
                && l.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
        };
        if name.len() > 253 || !name.contains('.') || !name.split('.').all(label_ok) {
            bail!("--acme-domain {value:?} is not a fully qualified domain name");
        }
        if !domains.contains(&name) {
            domains.push(name);
        }
    }
    Ok(domains)
}

/// `--acme-contact`: a single `mailto:` address (the only scheme Let's Encrypt accepts).
fn acme_contact(raw: &str) -> anyhow::Result<String> {
    let value = raw.trim();
    let address = value.strip_prefix("mailto:").unwrap_or("");
    let valid = address.split_once('@').is_some_and(|(local, host)| !local.is_empty() && host.contains('.'))
        && !address.bytes().any(|b| b.is_ascii_whitespace() || matches!(b, b',' | b'?' | b'<' | b'>'));
    if !valid {
        bail!("--acme-contact {raw:?}: expected one address as mailto:name@example.com");
    }
    Ok(value.to_owned())
}

/// `--acme-directory`: `production`, `staging` or an `https://` directory URL.
fn acme_directory(raw: &str) -> anyhow::Result<String> {
    match raw.trim() {
        "production" => Ok(LETS_ENCRYPT_PRODUCTION.to_owned()),
        "staging" => Ok(LETS_ENCRYPT_STAGING.to_owned()),
        url => {
            let uri: http::Uri = url.parse().map_err(|_| anyhow::anyhow!("--acme-directory {raw:?} is not a URL"))?;
            if uri.scheme_str() != Some("https") || uri.host().is_none() {
                bail!("--acme-directory {raw:?}: use production, staging or an https:// directory URL");
            }
            Ok(url.to_owned())
        }
    }
}

/// Parses `City,CC,lat,lon`; the city may itself contain commas.
pub(crate) fn parse_location(s: &str) -> Result<Location, String> {
    let mut parts = s.rsplitn(4, ',').map(str::trim);
    let (Some(lon), Some(lat), Some(country), Some(city)) =
        (parts.next(), parts.next(), parts.next(), parts.next())
    else {
        return Err("expected \"City,CC,lat,lon\", e.g. \"Dushanbe,TJ,38.5598,68.7870\"".into());
    };
    let number = |v: &str, what: &str, limit: f64| -> Result<f64, String> {
        let n: f64 = v.parse().map_err(|_| format!("{what} {v:?} is not a number"))?;
        if n.is_finite() && n.abs() <= limit {
            Ok(n)
        } else {
            Err(format!("{what} {v} is out of range ±{limit}"))
        }
    };
    if city.is_empty() || country.is_empty() {
        return Err("city and country must not be empty".into());
    }
    Ok(Location {
        city: city.to_owned(),
        country: country.to_owned(),
        lat: number(lat, "latitude", 90.0)?,
        lon: number(lon, "longitude", 180.0)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn location_parses_the_contract_example() {
        let l = parse_location("Dushanbe,TJ,38.5598,68.7870").unwrap();
        assert_eq!((l.city.as_str(), l.country.as_str()), ("Dushanbe", "TJ"));
        assert_eq!((l.lat, l.lon), (38.5598, 68.787));
        assert_eq!(parse_location("Frankfurt, am Main,DE,50.11,8.68").unwrap().city, "Frankfurt, am Main");
        assert!(parse_location("Dushanbe,TJ,91,0").is_err());
        assert!(parse_location("Dushanbe,TJ").is_err());
        assert!(parse_location(",TJ,1,1").is_err());
    }

    #[test]
    fn tls_flags_are_checked() {
        let parse = |args: &[&str]| {
            let cli = Cli::try_parse_from(std::iter::once("mintaka-server").chain(args.iter().copied()));
            cli.map_err(|e| e.to_string()).and_then(|c| Config::from_cli(c).map_err(|e| e.to_string()))
        };
        let c = parse(&["--tls-self-signed"]).unwrap();
        assert_eq!(c.tls.unwrap().listen.to_string(), DEFAULT_TLS_LISTEN);
        assert!(parse(&["--tls-listen", "127.0.0.1:1"]).is_err());
        assert!(parse(&["--redirect-http", "127.0.0.1:1"]).is_err());
        assert!(parse(&["--tls-cert", "a.pem"]).is_err());
        assert!(parse(&["--tls-self-signed", "--tls-cert", "a", "--tls-key", "b"]).is_err());
        assert!(parse(&["--pow-difficulty", "0"]).is_err());
        let c = parse(&["--tls-cert", "a", "--tls-key", "b", "--tls12", "--tls-listen", "[::]:443"]).unwrap();
        let tls = c.tls.unwrap();
        assert!(tls.allow_tls12);
        assert!(matches!(tls.source, CertSource::Pem { .. }));
        assert!(tls.http3.is_none());
    }

    fn parse(args: &[&str]) -> Result<Config, String> {
        let cli = Cli::try_parse_from(std::iter::once("mintaka-server").chain(args.iter().copied()));
        cli.map_err(|e| e.to_string()).and_then(|c| Config::from_cli(c).map_err(|e| e.to_string()))
    }

    fn acme(c: Config) -> AcmeSettings {
        match c.tls.unwrap().source {
            CertSource::Acme(settings) => settings,
            other => panic!("expected ACME, got {other:?}"),
        }
    }

    #[test]
    fn acme_flags_are_checked() {
        let a = acme(parse(&["--acme-domain", "Mintaka.TJ.", "--acme-domain", "www.mintaka.tj", "--acme-domain", "mintaka.tj"]).unwrap());
        assert_eq!(a.domains, ["mintaka.tj", "www.mintaka.tj"], "lower-cased, trailing dot dropped, de-duplicated");
        assert_eq!(a.directory, LETS_ENCRYPT_PRODUCTION);
        assert_eq!(a.contact, None);
        assert_eq!(a.cache, PathBuf::from("data/acme"));

        let a = acme(
            parse(&["--acme-domain", "mintaka.tj", "--acme-contact", "mailto:ops@mintaka.tj", "--acme-directory", "staging", "--data-dir", "/var/lib/m"])
                .unwrap(),
        );
        assert_eq!(a.directory, LETS_ENCRYPT_STAGING);
        assert_eq!(a.contact.as_deref(), Some("mailto:ops@mintaka.tj"));
        assert_eq!(a.cache, PathBuf::from("/var/lib/m/acme"));
        let a = acme(parse(&["--acme-domain", "mintaka.tj", "--acme-directory", "https://ca.example/dir"]).unwrap());
        assert_eq!(a.directory, "https://ca.example/dir");

        // Exactly one certificate source.
        for other in [&["--tls-self-signed"][..], &["--tls-cert", "a", "--tls-key", "b"]] {
            let mut args = vec!["--acme-domain", "mintaka.tj"];
            args.extend_from_slice(other);
            assert!(parse(&args).is_err(), "{args:?}");
        }
        // The other ACME flags mean nothing without a domain.
        assert!(parse(&["--tls-self-signed", "--acme-contact", "mailto:a@b.c"]).is_err());
        assert!(parse(&["--tls-self-signed", "--acme-directory", "staging"]).is_err());

        for bad in ["*.mintaka.tj", "localhost", "127.0.0.1", "-a.tj", "a..tj", "mintaka_tj.tj", "минтака.тж", ""] {
            assert!(parse(&["--acme-domain", bad]).is_err(), "domain {bad:?}");
        }
        for bad in ["ops@mintaka.tj", "mailto:", "mailto:ops", "mailto:a@b.c,c@d.e", "mailto:a b@c.d", "https://x.tj"] {
            assert!(parse(&["--acme-domain", "mintaka.tj", "--acme-contact", bad]).is_err(), "contact {bad:?}");
        }
        for bad in ["http://ca.example/dir", "prod", "https://", "ftp://x"] {
            assert!(parse(&["--acme-domain", "mintaka.tj", "--acme-directory", bad]).is_err(), "directory {bad:?}");
        }
    }

    #[test]
    fn http3_flags_are_checked() {
        let tls = parse(&["--tls-self-signed", "--http3"]).unwrap().tls.unwrap();
        assert_eq!(tls.http3.unwrap().advertise_port, None);
        let tls = parse(&["--acme-domain", "mintaka.tj", "--http3", "--http3-advertise-port", "443"]).unwrap().tls.unwrap();
        assert_eq!(tls.http3.unwrap().advertise_port, Some(443));
        assert!(parse(&["--http3"]).is_err(), "QUIC needs TLS");
        assert!(parse(&["--tls-self-signed", "--http3-advertise-port", "443"]).is_err(), "port without --http3");
        assert!(parse(&["--tls-self-signed", "--http3", "--http3-advertise-port", "0"]).is_err());
    }

    #[test]
    fn telegram_token_is_redacted_in_debug() {
        let t = TelegramSettings { bot_token: "123:secret".into(), chat_id: "42".into() };
        assert!(!format!("{t:?}").contains("secret"));
    }
}
