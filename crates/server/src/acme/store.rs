//! The certificates the TLS listener serves while ACME is on: the current one, swapped in place
//! on renewal, and — only while an order is being validated — one TLS-ALPN-01 challenge
//! certificate per name.

use std::collections::HashMap;
use std::fmt;
use std::sync::{Arc, PoisonError, RwLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::{Context, anyhow, bail, ensure};
use rcgen::{CertificateParams, CustomExtension, DistinguishedName, KeyPair};
use rustls::pki_types::pem::PemObject;
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};
use rustls::server::{ClientHello, ResolvesServerCert};
use rustls::sign::CertifiedKey;
use x509_parser::extensions::GeneralName;

use super::schedule::Validity;
use crate::tls;

/// Both kinds of certificate, behind locks that are held only to clone an `Arc`.
#[derive(Default)]
pub(crate) struct Store {
    current: RwLock<Option<Arc<Issued>>>,
    challenges: RwLock<HashMap<String, Arc<CertifiedKey>>>,
}

impl fmt::Debug for Store {
    // Never prints key material.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Store").finish_non_exhaustive()
    }
}

impl Store {
    /// The certificate ordinary handshakes get, if there is one yet.
    pub fn current(&self) -> Option<Arc<Issued>> {
        self.current.read().unwrap_or_else(PoisonError::into_inner).clone()
    }

    /// Replaces the served certificate. Handshakes already past certificate selection finish
    /// with the old one; every later one gets the new one.
    pub fn install(&self, issued: Issued) -> Arc<Issued> {
        let issued = Arc::new(issued);
        *self.current.write().unwrap_or_else(PoisonError::into_inner) = Some(Arc::clone(&issued));
        issued
    }

    /// Starts answering TLS-ALPN-01 for `name`.
    pub fn arm(&self, name: String, cert: Arc<CertifiedKey>) {
        self.challenges.write().unwrap_or_else(PoisonError::into_inner).insert(name, cert);
    }

    /// Stops answering TLS-ALPN-01 for `name`.
    pub fn disarm(&self, name: &str) {
        self.challenges.write().unwrap_or_else(PoisonError::into_inner).remove(name);
    }

    fn challenge(&self, name: &str) -> Option<Arc<CertifiedKey>> {
        self.challenges.read().unwrap_or_else(PoisonError::into_inner).get(name).cloned()
    }
}

/// The resolver of every ordinary TLS configuration (TCP and QUIC): the current certificate,
/// whatever the SNI, as `with_single_cert` would. `None` before the first issuance, which fails
/// the handshake.
#[derive(Debug)]
pub(crate) struct CurrentCert(pub Arc<Store>);

impl ResolvesServerCert for CurrentCert {
    fn resolve(&self, _hello: ClientHello<'_>) -> Option<Arc<CertifiedKey>> {
        self.0.current().map(|issued| Arc::clone(&issued.key))
    }
}

/// The resolver of the `acme-tls/1` configuration: the challenge certificate for the SNI name
/// (RFC 8737 §3 requires the validation server to send it), or no certificate at all.
#[derive(Debug)]
pub(crate) struct ChallengeCert(pub Arc<Store>);

impl ResolvesServerCert for ChallengeCert {
    fn resolve(&self, hello: ClientHello<'_>) -> Option<Arc<CertifiedKey>> {
        self.0.challenge(&hello.server_name()?.to_ascii_lowercase())
    }
}

/// A certificate from the CA, ready to serve.
pub(crate) struct Issued {
    pub key: Arc<CertifiedKey>,
    pub validity: Validity,
    /// The `dNSName`s of its subjectAltName, lower-case.
    pub names: Vec<String>,
}

impl Issued {
    /// Reads our cache format — the CA's PEM chain (leaf first) followed by the PEM private
    /// key — and checks that the pair matches and that the leaf names every one of `domains`.
    pub fn from_pem(pem: &[u8], domains: &[String]) -> anyhow::Result<Issued> {
        let certs = CertificateDer::pem_slice_iter(pem)
            .collect::<Result<Vec<_>, _>>()
            .context("reading the PEM certificate chain")?;
        let leaf = certs.first().ok_or_else(|| anyhow!("no certificate in the PEM data"))?;
        let (_, x509) = x509_parser::parse_x509_certificate(leaf).map_err(|e| anyhow!("parsing the certificate: {e}"))?;
        let at = |t: x509_parser::time::ASN1Time| UNIX_EPOCH + Duration::from_secs(u64::try_from(t.timestamp()).unwrap_or(0));
        let validity = Validity { not_before: at(x509.validity().not_before), not_after: at(x509.validity().not_after) };
        let names: Vec<String> = match x509.subject_alternative_name().map_err(|e| anyhow!("reading subjectAltName: {e}"))? {
            Some(san) => san
                .value
                .general_names
                .iter()
                .filter_map(|n| match n {
                    GeneralName::DNSName(name) => Some(name.to_ascii_lowercase()),
                    _ => None,
                })
                .collect(),
            None => Vec::new(),
        };
        for domain in domains {
            ensure!(names.contains(domain), "the certificate does not name {domain} (it names {})", names.join(", "));
        }
        let key = PrivateKeyDer::from_pem_slice(pem).context("reading the PEM private key")?;
        let key = Arc::new(tls::certified(certs, key)?);
        Ok(Issued { key, validity, names })
    }
}

/// The TLS-ALPN-01 certificate for one name (RFC 8737 §3): self-signed, the name as its only
/// subjectAltName, and the critical `acmeIdentifier` extension holding SHA-256 of the key
/// authorization. A fresh P-256 key each time; it lives only as long as the validation.
pub(crate) fn challenge_cert(name: &str, key_authorization_digest: &[u8]) -> anyhow::Result<Arc<CertifiedKey>> {
    // rcgen asserts the length; with `panic = "abort"` that would end the process.
    if key_authorization_digest.len() != 32 {
        bail!("the key authorization digest has {} bytes, not 32", key_authorization_digest.len());
    }
    let mut params = CertificateParams::new(vec![name.to_owned()]).context("challenge certificate name")?;
    params.distinguished_name = DistinguishedName::new();
    params.custom_extensions = vec![CustomExtension::new_acme_identifier(key_authorization_digest)];
    let key = KeyPair::generate().context("challenge key")?;
    let cert = params.self_signed(&key).context("signing the challenge certificate")?;
    let key = PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(key.serialize_der()));
    // Not `tls::certified`: its key check parses the certificate with webpki, which (rightly,
    // for a certificate it would verify) rejects the critical acmeIdentifier extension. The
    // pair is consistent by construction here.
    let key = tls::provider().key_provider.load_private_key(key).context("loading the challenge key")?;
    Ok(Arc::new(CertifiedKey::new(vec![cert.der().clone()], key)))
}

/// The time as `2027-01-27 08:12 UTC`, for logs.
pub(crate) fn utc(t: SystemTime) -> String {
    let secs = t.duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs());
    let (days, rem) = (secs / 86_400, secs % 86_400);
    // Civil date from days since 1970-01-01 (Howard Hinnant's algorithm, proleptic Gregorian).
    let z = days as i64 + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!("{year:04}-{month:02}-{day:02} {:02}:{:02} UTC", rem / 3600, rem % 3600 / 60)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use rcgen::{CertifiedIssuer, date_time_ymd};

    /// A CA-signed certificate for `names`, as PEM chain + key (our cache format).
    pub(crate) fn issued_pem(names: &[&str], from: (i32, u8, u8), until: (i32, u8, u8)) -> String {
        let mut ca = CertificateParams::new(Vec::<String>::new()).unwrap();
        ca.is_ca = rcgen::IsCa::Ca(rcgen::BasicConstraints::Unconstrained);
        let ca = CertifiedIssuer::self_signed(ca, KeyPair::generate().unwrap()).unwrap();
        let mut leaf = CertificateParams::new(names.iter().map(|n| n.to_string()).collect::<Vec<_>>()).unwrap();
        leaf.not_before = date_time_ymd(from.0, from.1, from.2);
        leaf.not_after = date_time_ymd(until.0, until.1, until.2);
        let key = KeyPair::generate().unwrap();
        let cert = leaf.signed_by(&key, &ca).unwrap();
        format!("{}{}{}", cert.pem(), ca.pem(), key.serialize_pem())
    }

    #[test]
    fn reads_validity_and_names_of_a_chain() {
        let pem = issued_pem(&["mintaka.tj", "www.mintaka.tj"], (2026, 10, 1), (2026, 12, 30));
        let issued = Issued::from_pem(pem.as_bytes(), &["mintaka.tj".into()]).unwrap();
        assert_eq!(issued.names, ["mintaka.tj", "www.mintaka.tj"]);
        assert_eq!(utc(issued.validity.not_before), "2026-10-01 00:00 UTC");
        assert_eq!(utc(issued.validity.not_after), "2026-12-30 00:00 UTC");
        assert_eq!(issued.key.cert.len(), 2, "leaf and issuer are both served");

        let err = Issued::from_pem(pem.as_bytes(), &["other.tj".into()]).err().unwrap().to_string();
        assert!(err.contains("does not name other.tj"), "{err}");
        assert!(Issued::from_pem(b"not pem", &[]).is_err());
        // A chain with somebody else's key.
        let other = issued_pem(&["mintaka.tj"], (2026, 10, 1), (2026, 12, 30));
        let key_start = other.find("-----BEGIN PRIVATE KEY-----").unwrap();
        let mixed = format!("{}{}", &pem[..pem.find("-----BEGIN PRIVATE KEY-----").unwrap()], &other[key_start..]);
        assert!(Issued::from_pem(mixed.as_bytes(), &[]).is_err());
    }

    #[test]
    fn challenge_certificate_follows_rfc_8737() {
        let digest = [7u8; 32];
        let cert = challenge_cert("mintaka.tj", &digest).unwrap();
        let (_, x509) = x509_parser::parse_x509_certificate(&cert.cert[0]).unwrap();
        let san = x509.subject_alternative_name().unwrap().unwrap();
        assert_eq!(san.value.general_names, [GeneralName::DNSName("mintaka.tj")]);
        // id-pe-acmeIdentifier, critical, an OCTET STRING of the 32-byte digest.
        let ext = x509.extensions().iter().find(|e| e.oid.to_id_string() == "1.3.6.1.5.5.7.1.31").unwrap();
        assert!(ext.critical);
        assert_eq!(ext.value, [&[0x04, 32][..], &digest].concat());
        assert!(challenge_cert("mintaka.tj", &[0; 31]).is_err(), "no panic on a bad digest");
    }

    #[test]
    fn utc_dates() {
        assert_eq!(utc(UNIX_EPOCH), "1970-01-01 00:00 UTC");
        assert_eq!(utc(UNIX_EPOCH + Duration::from_secs(951_782_400)), "2000-02-29 00:00 UTC");
        assert_eq!(utc(UNIX_EPOCH + Duration::from_secs(1_793_102_400 + 3_660)), "2026-10-27 13:01 UTC");
    }
}
