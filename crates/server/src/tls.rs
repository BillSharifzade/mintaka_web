//! rustls server configuration (aws-lc-rs, post-quantum hybrid key exchange) and the TLS facts
//! that `/_proof` and `Server-Timing` report for each connection.
//!
//! rustls 0.23 is used with its default features, which include `aws_lc_rs` and
//! `prefer-post-quantum`. The server offers exactly X25519MLKEM768, X25519 and secp256r1; note
//! that a rustls *server* picks among the groups both sides support in the **client's** order of
//! preference (current browsers and OpenSSL 3.5+ put X25519MLKEM768 first).

use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, anyhow};
use rustls::crypto::CryptoProvider;
use rustls::crypto::aws_lc_rs::{self, kx_group};
use rustls::pki_types::pem::PemObject;
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};
use rustls::server::{ResolvesServerCert, ServerConnection};
use rustls::sign::{CertifiedKey, SingleCertAndKey};
use rustls::{CipherSuite, NamedGroup, ProtocolVersion, ServerConfig, SupportedProtocolVersion};
use sha2::{Digest, Sha256};

use crate::acme::{self, Acme};
use crate::config::{CertSource, TlsSettings};

/// The aws-lc-rs provider restricted to the key-exchange groups the server offers:
/// X25519MLKEM768 (post-quantum hybrid), X25519, secp256r1.
pub fn provider() -> CryptoProvider {
    let mut provider = aws_lc_rs::default_provider();
    provider.kx_groups = vec![kx_group::X25519MLKEM768, kx_group::X25519, kx_group::SECP256R1];
    provider
}

/// A certificate generated at startup by `--tls-self-signed`.
#[derive(Debug, Clone)]
pub struct SelfSigned {
    /// DER of the certificate (clients in tests pin it).
    pub cert: CertificateDer<'static>,
    /// SHA-256 of the DER, upper-case hex with colons (as browsers show it).
    pub fingerprint: String,
}

/// The rustls configurations of the TLS listener's address, all sharing one certificate
/// resolver, so a renewed ACME certificate reaches TCP and QUIC at the same moment.
pub(crate) struct TlsSetup {
    /// The TCP listener.
    pub tcp: TcpTls,
    /// QUIC (HTTP/3), when enabled.
    pub quic: Option<Arc<ServerConfig>>,
    pub self_signed: Option<SelfSigned>,
    /// The ACME client whose certificates the resolver serves.
    pub acme: Option<Arc<Acme>>,
}

/// What the TCP listener chooses from after reading the ClientHello.
#[derive(Clone)]
pub(crate) struct TcpTls {
    /// Every ordinary connection: ALPN `h2` then `http/1.1`.
    pub config: Arc<ServerConfig>,
    /// TLS-ALPN-01 validations (RFC 8737): ALPN `acme-tls/1`, the challenge certificates. ACME only.
    pub challenge: Option<Arc<ServerConfig>>,
}

static TLS13_ONLY: &[&SupportedProtocolVersion] = &[&rustls::version::TLS13];
static TLS13_AND_12: &[&SupportedProtocolVersion] = &[&rustls::version::TLS13, &rustls::version::TLS12];

/// Builds every configuration the settings need: TLS 1.3 only (plus 1.2 on TCP if
/// `allow_tls12`), no client auth, no 0-RTT. QUIC gets ALPN `h3` and TLS 1.3 only, which QUIC
/// requires (RFC 9001 §4.2).
pub(crate) fn setup(settings: &TlsSettings) -> anyhow::Result<TlsSetup> {
    let (certs, self_signed, acme): (Arc<dyn ResolvesServerCert>, _, _) = match &settings.source {
        CertSource::Pem { cert, key } => {
            let certs = CertificateDer::pem_file_iter(cert)
                .with_context(|| format!("reading certificate chain {}", cert.display()))?
                .collect::<Result<Vec<_>, _>>()
                .with_context(|| format!("parsing certificate chain {}", cert.display()))?;
            if certs.is_empty() {
                anyhow::bail!("{} contains no PEM certificate", cert.display());
            }
            let key = PrivateKeyDer::from_pem_file(key)
                .with_context(|| format!("reading private key {}", key.display()))?;
            (Arc::new(SingleCertAndKey::from(certified(certs, key)?)), None, None)
        }
        CertSource::SelfSigned => {
            let names = vec!["localhost".to_owned(), "127.0.0.1".to_owned(), "::1".to_owned()];
            let generated = rcgen::generate_simple_self_signed(names)
                .context("generating a self-signed certificate")?;
            let cert = generated.cert.der().clone();
            let key = PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(generated.signing_key.serialize_der()));
            let fingerprint = fingerprint(&cert);
            let resolver = Arc::new(SingleCertAndKey::from(certified(vec![cert.clone()], key)?));
            (resolver, Some(SelfSigned { cert, fingerprint }), None)
        }
        CertSource::Acme(settings) => {
            let acme = Arc::new(Acme::load(settings.clone())?);
            (acme.resolver(), None, Some(acme))
        }
    };

    let versions = if settings.allow_tls12 { TLS13_AND_12 } else { TLS13_ONLY };
    let tcp = TcpTls {
        config: server_config(versions, Arc::clone(&certs), &[b"h2", b"http/1.1"])?,
        challenge: match &acme {
            Some(acme) => Some(server_config(versions, acme.challenge_resolver(), &[acme::ALPN])?),
            None => None,
        },
    };
    let quic = match settings.http3 {
        Some(_) => Some(server_config(TLS13_ONLY, certs, &[b"h3"])?),
        None => None,
    };
    Ok(TlsSetup { tcp, quic, self_signed, acme })
}

/// A certificate chain with its key, checked to belong together.
pub(crate) fn certified(certs: Vec<CertificateDer<'static>>, key: PrivateKeyDer<'static>) -> anyhow::Result<CertifiedKey> {
    CertifiedKey::from_der(certs, key, &provider()).map_err(|e| anyhow!("certificate and key do not form a usable pair: {e}"))
}

fn server_config(
    versions: &[&'static SupportedProtocolVersion],
    certs: Arc<dyn ResolvesServerCert>,
    alpn: &[&[u8]],
) -> anyhow::Result<Arc<ServerConfig>> {
    let mut config = ServerConfig::builder_with_provider(Arc::new(provider()))
        .with_protocol_versions(versions)
        .context("TLS protocol versions")?
        .with_no_client_auth()
        .with_cert_resolver(certs);
    config.alpn_protocols = alpn.iter().map(|p| p.to_vec()).collect();
    // No 0-RTT: replayable early data is not worth one round trip. (QUIC accepts 0 or u32::MAX.)
    config.max_early_data_size = 0;
    Ok(Arc::new(config))
}

/// SHA-256 fingerprint, `AB:CD:…`.
pub(crate) fn fingerprint(der: &[u8]) -> String {
    let digest = Sha256::digest(der);
    let mut out = String::with_capacity(digest.len() * 3);
    for (i, byte) in digest.iter().enumerate() {
        if i > 0 {
            out.push(':');
        }
        out.push_str(&format!("{byte:02X}"));
    }
    out
}

/// What was negotiated on one TLS connection.
#[derive(Debug, Clone)]
pub(crate) struct TlsFacts {
    /// `TLSv1.3` / `TLSv1.2`.
    pub version: &'static str,
    /// e.g. `TLS13_AES_128_GCM_SHA256`; `None` if the TLS stack did not report it.
    pub cipher: Option<&'static str>,
    /// e.g. `X25519MLKEM768`; `None` if the TLS stack did not report it.
    pub group: Option<&'static str>,
    pub post_quantum: bool,
    /// `h2` / `http/1.1` / `h3`, or `None` if the client offered no ALPN.
    pub alpn: Option<&'static str>,
    /// Complete ClientHello → client Finished, measured on the server.
    pub handshake: Option<Duration>,
}

impl TlsFacts {
    /// Reads the facts of a completed TCP handshake from rustls.
    pub fn of(conn: &ServerConnection, handshake: Duration) -> Self {
        let group = conn.negotiated_key_exchange_group().map(|g| g.name());
        TlsFacts {
            version: match conn.protocol_version() {
                Some(ProtocolVersion::TLSv1_3) => "TLSv1.3",
                Some(ProtocolVersion::TLSv1_2) => "TLSv1.2",
                _ => "unknown",
            },
            cipher: conn.negotiated_cipher_suite().and_then(|s| s.suite().as_str()),
            group: group.and_then(|g| g.as_str()),
            post_quantum: group.is_some_and(is_post_quantum),
            alpn: alpn_name(conn.alpn_protocol()),
            handshake: Some(handshake),
        }
    }

    /// A QUIC connection. quinn 0.11 reports only the ALPN protocol and SNI, so the suite and the
    /// group come from our own ServerHello, read back by `quic::handshake` (`None` if it could
    /// not be read). The version is TLS 1.3 because the QUIC configuration offers nothing else,
    /// as QUIC requires (RFC 9001 §4.2).
    pub fn quic(cipher: Option<CipherSuite>, group: Option<NamedGroup>, alpn: Option<&[u8]>, handshake: Option<Duration>) -> Self {
        TlsFacts {
            version: "TLSv1.3",
            cipher: cipher.and_then(|c| c.as_str()),
            group: group.and_then(|g| g.as_str()),
            post_quantum: group.is_some_and(is_post_quantum),
            alpn: alpn_name(alpn),
            handshake,
        }
    }

    /// For curl mode: `TLS 1.3 · X25519MLKEM768 (post-quantum hybrid)`.
    pub fn describe(&self) -> String {
        let version = match self.version {
            "TLSv1.3" => "TLS 1.3",
            "TLSv1.2" => "TLS 1.2",
            other => other,
        };
        let Some(group) = self.group else { return version.to_owned() };
        let note = if !self.post_quantum {
            ""
        } else if group.starts_with("MLKEM") {
            " (post-quantum)"
        } else {
            " (post-quantum hybrid)"
        };
        format!("{version} · {group}{note}")
    }
}

/// The negotiated ALPN protocol as a name. The configurations offer only these (the
/// TLS-ALPN-01 one, `acme-tls/1`, never reaches HTTP), so rustls cannot negotiate anything else.
fn alpn_name(protocol: Option<&[u8]>) -> Option<&'static str> {
    match protocol {
        Some(b"h2") => Some("h2"),
        Some(b"http/1.1") => Some("http/1.1"),
        Some(b"h3") => Some("h3"),
        _ => None,
    }
}

/// True for the ML-KEM groups, pure or hybrid.
pub(crate) fn is_post_quantum(group: NamedGroup) -> bool {
    matches!(
        group,
        NamedGroup::X25519MLKEM768
            | NamedGroup::secp256r1MLKEM768
            | NamedGroup::MLKEM512
            | NamedGroup::MLKEM768
            | NamedGroup::MLKEM1024
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn offers_the_three_contract_groups_post_quantum_first() {
        let names: Vec<_> = provider().kx_groups.iter().map(|g| g.name()).collect();
        assert_eq!(names, [NamedGroup::X25519MLKEM768, NamedGroup::X25519, NamedGroup::secp256r1]);
        assert!(is_post_quantum(NamedGroup::X25519MLKEM768));
        assert!(!is_post_quantum(NamedGroup::X25519));
    }

    fn settings(http3: bool) -> TlsSettings {
        TlsSettings {
            listen: "127.0.0.1:0".parse().unwrap(),
            source: CertSource::SelfSigned,
            allow_tls12: false,
            http3: http3.then(Default::default),
        }
    }

    #[test]
    fn self_signed_config_builds() {
        let setup = setup(&settings(false)).unwrap();
        let ss = setup.self_signed.unwrap();
        assert_eq!(ss.fingerprint.len(), 32 * 3 - 1);
        assert_eq!(setup.tcp.config.alpn_protocols, [b"h2".to_vec(), b"http/1.1".to_vec()]);
        assert_eq!(setup.tcp.config.max_early_data_size, 0);
        assert!(setup.tcp.challenge.is_none() && setup.quic.is_none() && setup.acme.is_none());
    }

    #[test]
    fn quic_config_is_one_rustls_accepts_for_quic() {
        let setup = setup(&settings(true)).unwrap();
        let quic = setup.quic.unwrap();
        assert_eq!(quic.alpn_protocols, [b"h3".to_vec()]);
        // quinn unwraps this constructor for every connection; with `panic = "abort"` a config
        // it rejects would take the whole server down on the first QUIC client.
        let params = [0u8; 0].to_vec();
        rustls::quic::ServerConnection::new(quic, rustls::quic::Version::V1, params).expect("QUIC-compatible config");
    }

    #[test]
    fn describes_hybrid_groups() {
        let facts = TlsFacts {
            version: "TLSv1.3",
            cipher: Some("TLS13_AES_128_GCM_SHA256"),
            group: Some("X25519MLKEM768"),
            post_quantum: true,
            alpn: Some("h2"),
            handshake: Some(Duration::from_micros(1830)),
        };
        assert_eq!(facts.describe(), "TLS 1.3 · X25519MLKEM768 (post-quantum hybrid)");
        let classic = TlsFacts { group: Some("X25519"), post_quantum: false, ..facts.clone() };
        assert_eq!(classic.describe(), "TLS 1.3 · X25519");
        let unreported = TlsFacts { group: None, post_quantum: false, ..facts };
        assert_eq!(unreported.describe(), "TLS 1.3");
    }

    #[test]
    fn quic_facts_name_what_the_server_hello_said() {
        let f = TlsFacts::quic(
            Some(CipherSuite::TLS13_CHACHA20_POLY1305_SHA256),
            Some(NamedGroup::X25519MLKEM768),
            Some(b"h3"),
            None,
        );
        assert_eq!((f.version, f.cipher, f.group, f.alpn), ("TLSv1.3", Some("TLS13_CHACHA20_POLY1305_SHA256"), Some("X25519MLKEM768"), Some("h3")));
        assert!(f.post_quantum);
        let unknown = TlsFacts::quic(None, None, Some(b"h3"), None);
        assert_eq!((unknown.cipher, unknown.group, unknown.post_quantum), (None, None, false));
    }
}
