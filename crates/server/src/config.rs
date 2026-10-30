//! Command-line interface and the validated configuration the server runs with.

use std::fmt;
use std::net::{IpAddr, SocketAddr};
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

    /// A reverse proxy in front of this server that ends the visitors' connections, as an address
    /// or CIDR (repeatable). Connections from there are proxied: the visitor's address comes from
    /// --client-ip-header (for rate limits only, never stored), /_proof reports the edge instead
    /// of TLS it cannot see, and the kernel RTT of the proxy hop is not passed off as the visitor's.
    #[arg(long, value_name = "CIDR", value_parser = parse_cidr)]
    pub trusted_proxy: Vec<Cidr>,

    /// Request header in which the trusted proxy names the visitor's address; the last address in
    /// it counts (the one the proxy itself saw).
    #[arg(long, value_name = "NAME", default_value = "x-forwarded-for")]
    pub client_ip_header: String,
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
    /// The reverse proxy in front of this server, if any.
    pub proxy: Option<ProxySettings>,
    /// Optional new-brief notifications.
    pub telegram: Option<TelegramSettings>,
    /// Concurrent `/_telemetry` streams per process; more get `503`.
    pub max_telemetry_streams: usize,
    /// How long shutdown waits for in-flight requests.
    pub shutdown_grace: Duration,
}

/// A reverse proxy that ends visitors' connections before they reach this server.
#[derive(Debug, Clone)]
pub struct ProxySettings {
    /// Where its connections come from.
    pub from: Vec<Cidr>,
    /// The header it puts the visitor's address in.
    pub client_ip_header: http::HeaderName,
}

impl ProxySettings {
    /// Whether a connection from `peer` comes through the proxy.
    pub fn trusts(&self, peer: IpAddr) -> bool {
        self.from.iter().any(|c| c.contains(peer))
    }
}

/// An IP network: `172.20.0.0/16`, `2001:db8::/32`, or one address.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cidr {
    addr: IpAddr,
    prefix: u8,
}

impl Cidr {
    /// Whether `ip` is in the network (IPv4-mapped IPv6 counts as IPv4).
    pub fn contains(&self, ip: IpAddr) -> bool {
        match (self.addr, ip.to_canonical()) {
            (IpAddr::V4(net), IpAddr::V4(ip)) => masked(u32::from(net).into(), self.prefix, 32) == masked(u32::from(ip).into(), self.prefix, 32),
            (IpAddr::V6(net), IpAddr::V6(ip)) => masked(u128::from(net), self.prefix, 128) == masked(u128::from(ip), self.prefix, 128),
            _ => false,
        }
    }
}

impl fmt::Display for Cidr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}/{}", self.addr, self.prefix)
    }
}

/// The top `prefix` of `bits` bits.
fn masked(addr: u128, prefix: u8, bits: u8) -> u128 {
    if prefix == 0 { 0 } else { addr >> (bits - prefix) }
}

fn parse_cidr(s: &str) -> Result<Cidr, String> {
    let (addr, prefix) = s.split_once('/').map_or((s, None), |(a, p)| (a, Some(p)));
    let addr: IpAddr = addr.parse().map_err(|_| format!("{addr:?} is not an IP address"))?;
    let addr = addr.to_canonical();
    let max = if addr.is_ipv4() { 32 } else { 128 };
    let prefix = match prefix {
        None => max,
        Some(p) => p.parse::<u8>().ok().filter(|p| *p <= max).ok_or_else(|| format!("prefix {p:?} must be 0-{max}"))?,
    };
    Ok(Cidr { addr, prefix })
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

/// The Telegram bot (`MINTAKA_TELEGRAM_BOT_TOKEN`, `MINTAKA_TELEGRAM_CHAT_ID`): new-brief
/// notifications, and the bot clients continue in (docs/CONTRACTS.md §19).
#[derive(Clone)]
pub struct TelegramSettings {
    /// Bot API token (secret: never logged or printed).
    pub bot_token: String,
    /// The team chat's numeric id: notifications and clients' messages go there.
    pub chat_id: i64,
    /// Where the Bot API is: `https://api.telegram.org`, or a test double
    /// (`MINTAKA_TELEGRAM_API_BASE`: https://, or http:// to this machine only).
    pub api_base: String,
}

/// The Bot API's real address.
pub const TELEGRAM_API: &str = "https://api.telegram.org";

impl fmt::Debug for TelegramSettings {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TelegramSettings")
            .field("bot_token", &"<redacted>")
            .field("chat_id", &self.chat_id)
            .field("api_base", &self.api_base)
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
        let proxy = if cli.trusted_proxy.is_empty() {
            None
        } else {
            let client_ip_header = http::HeaderName::try_from(cli.client_ip_header.trim())
                .with_context(|| format!("--client-ip-header {:?} is not a header name", cli.client_ip_header))?;
            Some(ProxySettings { from: cli.trusted_proxy, client_ip_header })
        };
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
            proxy,
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
    telegram_settings(get("MINTAKA_TELEGRAM_BOT_TOKEN"), get("MINTAKA_TELEGRAM_CHAT_ID"), get("MINTAKA_TELEGRAM_API_BASE"))
}

fn telegram_settings(token: Option<String>, chat: Option<String>, api_base: Option<String>) -> anyhow::Result<Option<TelegramSettings>> {
    let (bot_token, chat_id) = match (token, chat) {
        (Some(token), Some(chat)) => (token.trim().to_owned(), chat),
        (None, None) => return Ok(None),
        _ => bail!("set both MINTAKA_TELEGRAM_BOT_TOKEN and MINTAKA_TELEGRAM_CHAT_ID, or neither"),
    };
    if !bot_token.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b':' | b'_' | b'-')) {
        bail!("MINTAKA_TELEGRAM_BOT_TOKEN has characters a bot token cannot contain");
    }
    // The bot compares chats by number; a group's id looks like -1001234567890.
    let chat_id = chat_id
        .trim()
        .parse()
        .map_err(|_| anyhow::anyhow!("MINTAKA_TELEGRAM_CHAT_ID must be the team chat's numeric id (README → Contact briefs)"))?;
    let api_base = match api_base {
        None => TELEGRAM_API.to_owned(),
        Some(base) => {
            let base = base.trim().trim_end_matches('/').to_owned();
            let uri: http::Uri = base.parse().context("MINTAKA_TELEGRAM_API_BASE is not a URL")?;
            let host = uri.host().unwrap_or("").trim_start_matches('[').trim_end_matches(']');
            let local = host == "localhost" || host.parse::<IpAddr>().is_ok_and(|ip| ip.is_loopback());
            // The token travels in the URL: plain HTTP only to this machine (a test double).
            match uri.scheme_str() {
                Some("https") if !host.is_empty() => base,
                Some("http") if local => base,
                _ => bail!("MINTAKA_TELEGRAM_API_BASE must be an https:// URL, or http:// to this machine"),
            }
        }
    };
    Ok(Some(TelegramSettings { bot_token, chat_id, api_base }))
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
    fn trusted_proxies() {
        assert!(parse(&[]).unwrap().proxy.is_none());
        let proxy = parse(&["--trusted-proxy", "172.20.0.0/16", "--trusted-proxy", "::1", "--client-ip-header", "X-Mintaka-Client-Ip"])
            .unwrap()
            .proxy
            .unwrap();
        assert_eq!(proxy.client_ip_header, "x-mintaka-client-ip");
        for (ip, trusted) in [("172.20.0.6", true), ("172.20.255.255", true), ("172.21.0.6", false), ("::1", true), ("::2", false), ("::ffff:172.20.0.6", true)] {
            assert_eq!(proxy.trusts(ip.parse().unwrap()), trusted, "{ip}");
        }
        assert!(parse_cidr("0.0.0.0/0").unwrap().contains("203.0.113.9".parse().unwrap()));
        assert!(!parse_cidr("10.0.0.0/8").unwrap().contains("::1".parse().unwrap()));
        for bad in ["172.20.0.0/33", "fe80::/129", "proxy", "1.2.3.4/x", ""] {
            assert!(parse(&["--trusted-proxy", bad]).is_err(), "{bad:?}");
        }
        assert!(parse(&["--trusted-proxy", "10.0.0.1", "--client-ip-header", "bad header"]).is_err());
    }

    #[test]
    fn telegram_token_is_redacted_in_debug() {
        let t = TelegramSettings { bot_token: "123:secret".into(), chat_id: 42, api_base: TELEGRAM_API.into() };
        assert!(!format!("{t:?}").contains("secret"));
    }

    #[test]
    fn telegram_settings_are_checked() {
        let s = |v: &str| Some(v.to_owned());
        assert!(telegram_settings(None, None, None).unwrap().is_none());
        let t = telegram_settings(s("123:abc-_x"), s(" -1001234567890 "), None).unwrap().unwrap();
        assert_eq!((t.chat_id, t.api_base.as_str()), (-1_001_234_567_890, TELEGRAM_API));
        assert!(telegram_settings(s("123:abc"), None, None).is_err(), "both or neither");
        assert!(telegram_settings(s("123:a b"), s("1"), None).is_err(), "token characters");
        assert!(telegram_settings(s("123:abc"), s("@mintaka_team"), None).is_err(), "numeric chat id");
        for (base, ok) in [
            ("https://tg-proxy.example/", true),
            ("http://127.0.0.1:9", true),
            ("http://[::1]:9", true),
            ("http://localhost:9", true),
            ("http://192.0.2.1:9", false),
            ("ftp://127.0.0.1", false),
            ("api.telegram.org", false),
        ] {
            let got = telegram_settings(s("1:x"), s("2"), s(base));
            assert_eq!(got.is_ok(), ok, "{base}");
        }
        let trimmed = telegram_settings(s("1:x"), s("2"), s("https://tg-proxy.example/")).unwrap().unwrap();
        assert_eq!(trimmed.api_base, "https://tg-proxy.example");
    }
}
