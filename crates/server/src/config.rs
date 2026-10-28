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
        let source = match (cli.tls_cert, cli.tls_key, cli.tls_self_signed) {
            (Some(cert), Some(key), false) => Some(CertSource::Pem { cert, key }),
            (None, None, true) => Some(CertSource::SelfSigned),
            (None, None, false) => None,
            _ => bail!("use either --tls-cert with --tls-key, or --tls-self-signed"),
        };
        let tls = match (source, cli.tls_listen) {
            (Some(source), listen) => {
                let listen = match listen {
                    Some(addr) => addr,
                    None => DEFAULT_TLS_LISTEN.parse().context("default TLS address")?,
                };
                Some(TlsSettings { listen, source, allow_tls12: cli.tls12 })
            }
            (None, Some(_)) => bail!("--tls-listen needs --tls-cert/--tls-key or --tls-self-signed"),
            (None, None) if cli.tls12 => bail!("--tls12 needs a TLS listener"),
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
    }

    #[test]
    fn telegram_token_is_redacted_in_debug() {
        let t = TelegramSettings { bot_token: "123:secret".into(), chat_id: "42".into() };
        assert!(!format!("{t:?}").contains("secret"));
    }
}
