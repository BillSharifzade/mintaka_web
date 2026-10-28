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
use rustls::server::ServerConnection;
use rustls::{NamedGroup, ProtocolVersion, ServerConfig, SupportedProtocolVersion};
use sha2::{Digest, Sha256};

use crate::config::CertSource;

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

/// A ready rustls server configuration.
pub(crate) struct TlsSetup {
    pub config: Arc<ServerConfig>,
    pub self_signed: Option<SelfSigned>,
}

/// Builds the server configuration: TLS 1.3 only (plus 1.2 if `allow_tls12`), ALPN `h2` then
/// `http/1.1`, no client auth, no 0-RTT.
pub(crate) fn server_config(source: &CertSource, allow_tls12: bool) -> anyhow::Result<TlsSetup> {
    let (certs, key, self_signed) = match source {
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
            (certs, key, None)
        }
        CertSource::SelfSigned => {
            let names = vec!["localhost".to_owned(), "127.0.0.1".to_owned(), "::1".to_owned()];
            let generated = rcgen::generate_simple_self_signed(names)
                .context("generating a self-signed certificate")?;
            let cert = generated.cert.der().clone();
            let key = PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(generated.signing_key.serialize_der()));
            let fingerprint = fingerprint(&cert);
            (vec![cert.clone()], key, Some(SelfSigned { cert, fingerprint }))
        }
    };

    static TLS13_ONLY: &[&SupportedProtocolVersion] = &[&rustls::version::TLS13];
    static TLS13_AND_12: &[&SupportedProtocolVersion] = &[&rustls::version::TLS13, &rustls::version::TLS12];
    let versions = if allow_tls12 { TLS13_AND_12 } else { TLS13_ONLY };

    let mut config = ServerConfig::builder_with_provider(Arc::new(provider()))
        .with_protocol_versions(versions)
        .context("TLS protocol versions")?
        .with_no_client_auth()
        .with_single_cert(certs, key)
        .map_err(|e| anyhow!("certificate and key do not form a usable pair: {e}"))?;
    config.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec()];
    config.max_early_data_size = 0;
    Ok(TlsSetup { config: Arc::new(config), self_signed })
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
    /// e.g. `TLS13_AES_128_GCM_SHA256`.
    pub cipher: &'static str,
    /// e.g. `X25519MLKEM768`.
    pub group: &'static str,
    pub post_quantum: bool,
    /// `h2` / `http/1.1`, or `None` if the client offered no ALPN.
    pub alpn: Option<&'static str>,
    /// Complete ClientHello → client Finished, measured on the server.
    pub handshake: Duration,
}

impl TlsFacts {
    /// Reads the facts of a completed handshake.
    pub fn of(conn: &ServerConnection, handshake: Duration) -> Self {
        let group = conn.negotiated_key_exchange_group().map(|g| g.name());
        TlsFacts {
            version: match conn.protocol_version() {
                Some(ProtocolVersion::TLSv1_3) => "TLSv1.3",
                Some(ProtocolVersion::TLSv1_2) => "TLSv1.2",
                _ => "unknown",
            },
            cipher: conn
                .negotiated_cipher_suite()
                .and_then(|s| s.suite().as_str())
                .unwrap_or("unknown"),
            group: group.and_then(|g| g.as_str()).unwrap_or("unknown"),
            post_quantum: group.is_some_and(is_post_quantum),
            alpn: match conn.alpn_protocol() {
                Some(b"h2") => Some("h2"),
                Some(b"http/1.1") => Some("http/1.1"),
                // The server offers only the two above, so rustls cannot negotiate anything else.
                _ => None,
            },
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
        let note = if !self.post_quantum {
            ""
        } else if self.group.starts_with("MLKEM") {
            " (post-quantum)"
        } else {
            " (post-quantum hybrid)"
        };
        format!("{version} · {}{note}", self.group)
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

    #[test]
    fn self_signed_config_builds() {
        let setup = server_config(&CertSource::SelfSigned, false).unwrap();
        let ss = setup.self_signed.unwrap();
        assert_eq!(ss.fingerprint.len(), 32 * 3 - 1);
        assert_eq!(setup.config.alpn_protocols, [b"h2".to_vec(), b"http/1.1".to_vec()]);
    }

    #[test]
    fn describes_hybrid_groups() {
        let facts = TlsFacts {
            version: "TLSv1.3",
            cipher: "TLS13_AES_128_GCM_SHA256",
            group: "X25519MLKEM768",
            post_quantum: true,
            alpn: Some("h2"),
            handshake: Duration::from_micros(1830),
        };
        assert_eq!(facts.describe(), "TLS 1.3 · X25519MLKEM768 (post-quantum hybrid)");
        let classic = TlsFacts { group: "X25519", post_quantum: false, ..facts };
        assert_eq!(classic.describe(), "TLS 1.3 · X25519");
    }
}
