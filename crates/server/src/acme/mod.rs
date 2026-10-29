//! Certificates from an ACME CA (Let's Encrypt), obtained and renewed by the server itself.
//!
//! * **Validation: TLS-ALPN-01** (RFC 8737) on the TLS listener — nothing on port 80. `conn`
//!   reads each ClientHello before choosing a configuration; one that offers only `acme-tls/1`
//!   gets the challenge configuration, whose resolver answers with a self-signed certificate for
//!   the SNI name carrying the critical `acmeIdentifier` extension. That connection is closed
//!   right after the handshake and never reaches HTTP.
//! * **Cache:** the account key and the certificate (the CA's chain, then our P-256 key, PEM) in
//!   `{data_dir}/acme`, 0600 files in a 0700 directory. A cached certificate is served from the
//!   first handshake; a missing, expired or mismatched one is ordered at once.
//! * **Renewal** about 30 days before expiry ([`schedule`]); the new certificate is swapped into
//!   the resolver that every TLS configuration (TCP and QUIC) shares, without a restart.
//! * **Logs** name the CA, the domains, dates and the CA's problem documents — never keys, tokens
//!   or key authorizations.
//!
//! Library: `instant-acme` for RFC 8555 (accounts, orders, JWS) on the hyper + rustls stack the
//! server already links; the resolver, cache and schedule are ours so they use our crypto
//! provider, our file modes and a renewal policy we can test.

pub(crate) mod cache;
pub(crate) mod schedule;
mod store;

use std::sync::Arc;
use std::time::{Duration, SystemTime};

use anyhow::{Context, anyhow, bail};
use instant_acme::{
    Account, AccountBuilder, AccountCredentials, AuthorizationStatus, ChallengeType, Identifier, NewAccount, NewOrder,
    Order, OrderStatus, RetryPolicy,
};
use rustls::server::{ClientHello, ResolvesServerCert};
use tokio::sync::{Mutex, watch};

use self::schedule::human;
use self::store::{ChallengeCert, CurrentCert, Issued, Store, utc};
use crate::config::AcmeSettings;

/// The ALPN protocol of TLS-ALPN-01 validation connections.
pub(crate) const ALPN: &[u8] = b"acme-tls/1";

/// How often the renewal task looks at the wall clock again while waiting (so a suspended or
/// re-clocked machine still renews on time).
const MAX_NAP: Duration = Duration::from_secs(12 * 3600);

/// One whole order may take this long. instant-acme's HTTP client has no timeout of its own, and
/// a CA that stops answering must not stall renewal for good.
const ORDER_TIMEOUT: Duration = Duration::from_secs(600);

/// Polling the CA while it validates and issues: 0.5 s, then ×1.5, for up to two minutes.
const POLL: RetryPolicy = RetryPolicy::new()
    .initial_delay(Duration::from_millis(500))
    .backoff(1.5)
    .timeout(Duration::from_secs(120));

/// True for a TLS-ALPN-01 validation: the client offers `acme-tls/1` and nothing else (RFC 8737
/// §3; Let's Encrypt's validation servers do exactly that). A browser never does.
pub(crate) fn is_challenge(hello: &ClientHello<'_>) -> bool {
    hello.alpn().is_some_and(|mut protocols| protocols.next() == Some(ALPN) && protocols.next().is_none())
}

/// The ACME client of one server: settings, the certificates being served, the account.
pub(crate) struct Acme {
    settings: AcmeSettings,
    store: Arc<Store>,
    /// Loaded or created on first use (that needs the network, so not at startup).
    account: Mutex<Option<Account>>,
}

impl Acme {
    /// Prepares the cache and starts serving the cached certificate, if it is still good.
    /// Never touches the network.
    pub fn load(settings: AcmeSettings) -> anyhow::Result<Acme> {
        cache::create_dir(&settings.cache)
            .with_context(|| format!("cannot create the ACME cache {} (set --data-dir to a writable directory)", settings.cache.display()))?;
        let store = Arc::new(Store::default());
        let path = settings.certificate_path();
        let names = settings.domains.join(", ");
        let cached = cache::read(&path).with_context(|| format!("cannot read {}", path.display()))?;
        match cached.map(|pem| Issued::from_pem(&pem, &settings.domains)) {
            Some(Ok(issued)) if SystemTime::now() < issued.validity.not_after => {
                let issued = store.install(issued);
                tracing::info!(
                    target: "acme",
                    "serving the cached certificate for {}: valid until {}, renewal due {}",
                    issued.names.join(", "),
                    utc(issued.validity.not_after),
                    utc(schedule::renew_at(issued.validity)),
                );
            }
            Some(Ok(issued)) => tracing::warn!(
                target: "acme",
                "the cached certificate for {names} expired {}; ordering a new one",
                utc(issued.validity.not_after)
            ),
            Some(Err(e)) => tracing::warn!(target: "acme", "the cached certificate {} is unusable ({e:#}); ordering a new one", path.display()),
            None => tracing::info!(
                target: "acme",
                "no certificate for {names} yet; ordering one from {}. Until it arrives, TLS handshakes fail",
                settings.directory
            ),
        }
        Ok(Acme { settings, store, account: Mutex::new(None) })
    }

    /// The resolver of every ordinary TLS configuration.
    pub fn resolver(&self) -> Arc<dyn ResolvesServerCert> {
        Arc::new(CurrentCert(Arc::clone(&self.store)))
    }

    /// The resolver of the `acme-tls/1` configuration.
    pub fn challenge_resolver(&self) -> Arc<dyn ResolvesServerCert> {
        Arc::new(ChallengeCert(Arc::clone(&self.store)))
    }

    /// Orders and renews until shutdown.
    pub async fn run(self: Arc<Self>, mut shutdown: watch::Receiver<bool>) {
        let mut failures = 0u32;
        loop {
            let current = self.store.current().map(|c| c.validity);
            let wait = schedule::next_attempt(SystemTime::now(), current, failures);
            let nap = wait.min(MAX_NAP);
            if !nap.is_zero() {
                tokio::select! {
                    () = tokio::time::sleep(nap) => {}
                    () = stopped(&mut shutdown) => return,
                }
                if nap < wait {
                    continue;
                }
            }
            let names = self.settings.domains.join(", ");
            tracing::info!(target: "acme", "ordering a certificate for {names} from {}", self.settings.directory);
            let started = std::time::Instant::now();
            let attempt = tokio::select! {
                result = tokio::time::timeout(ORDER_TIMEOUT, self.issue()) => {
                    result.unwrap_or_else(|_| Err(anyhow!("no answer from the CA within {}", human(ORDER_TIMEOUT))))
                }
                () = stopped(&mut shutdown) => return,
            };
            match attempt {
                Ok(issued) => {
                    failures = 0;
                    tracing::info!(
                        target: "acme",
                        "new certificate for {} in {:.1} s, now served on every new connection: valid until {}, renewal due {}",
                        issued.names.join(", "),
                        started.elapsed().as_secs_f64(),
                        utc(issued.validity.not_after),
                        utc(schedule::renew_at(issued.validity)),
                    );
                }
                Err(e) => {
                    failures = failures.saturating_add(1);
                    let retry = schedule::next_attempt(SystemTime::now(), current, failures);
                    let serving = match current {
                        Some(v) if SystemTime::now() < v.not_after => format!("still serving the old one until {}", utc(v.not_after)),
                        _ => "no certificate to serve".to_owned(),
                    };
                    tracing::error!(target: "acme", "ordering a certificate for {names} failed: {e:#} ({serving}; next attempt in {})", human(retry));
                }
            }
        }
    }

    /// One order, start to finish: account, authorizations (TLS-ALPN-01), CSR, download,
    /// cache, swap.
    async fn issue(&self) -> anyhow::Result<Arc<Issued>> {
        let domains = &self.settings.domains;
        let account = self.account().await?;
        let identifiers: Vec<Identifier> = domains.iter().cloned().map(Identifier::Dns).collect();
        let mut order = account.new_order(&NewOrder::new(&identifiers)).await.context("placing the order")?;

        let mut armed = Armed { store: &self.store, names: Vec::new() };
        {
            let mut authorizations = order.authorizations();
            while let Some(authz) = authorizations.next().await {
                let mut authz = authz.context("fetching an authorization")?;
                let name = authz.identifier().to_string();
                match authz.status {
                    AuthorizationStatus::Pending => {}
                    // Validated by an earlier order of this account and still fresh.
                    AuthorizationStatus::Valid => continue,
                    status => bail!("the CA reports the authorization for {name} as {status:?}"),
                }
                let mut challenge = authz
                    .challenge(ChallengeType::TlsAlpn01)
                    .ok_or_else(|| anyhow!("the CA offers no tls-alpn-01 challenge for {name}"))?;
                let cert = store::challenge_cert(&name, challenge.key_authorization().digest().as_ref())?;
                armed.arm(name.clone(), cert);
                challenge.set_ready().await.with_context(|| format!("asking the CA to validate {name}"))?;
            }
        }
        match order.poll_ready(&POLL).await {
            Ok(OrderStatus::Ready) => {}
            Ok(status) => bail!("the order is {status:?}{}", reasons(&mut order).await),
            Err(e) => bail!("validation failed: {e}{}", reasons(&mut order).await),
        }
        drop(armed);

        // ECDSA P-256 (spec §10.2), a new key for every certificate.
        let key = rcgen::KeyPair::generate().context("generating the certificate key")?;
        let mut params = rcgen::CertificateParams::new(domains.clone()).context("certificate names")?;
        params.distinguished_name = rcgen::DistinguishedName::new();
        let csr = params.serialize_request(&key).context("building the CSR")?;
        order.finalize_csr(csr.der()).await.context("finalizing the order")?;
        let chain = order.poll_certificate(&POLL).await.context("downloading the certificate")?;

        let mut pem = chain;
        if !pem.ends_with('\n') {
            pem.push('\n');
        }
        pem.push_str(&key.serialize_pem());
        let issued = Issued::from_pem(pem.as_bytes(), domains).context("the CA's certificate")?;
        let path = self.settings.certificate_path();
        cache::write(&path, pem.as_bytes()).with_context(|| format!("caching the certificate in {}", path.display()))?;
        Ok(self.store.install(issued))
    }

    /// The ACME account: from the cache, or created (and cached) on first use.
    async fn account(&self) -> anyhow::Result<Account> {
        let mut slot = self.account.lock().await;
        if let Some(account) = &*slot {
            return Ok(account.clone());
        }
        let path = self.settings.account_path();
        let account = match cache::read(&path).with_context(|| format!("cannot read {}", path.display()))? {
            Some(json) => {
                let credentials: AccountCredentials = serde_json::from_slice(&json)
                    .map_err(|_| anyhow!("{} is not an ACME account file", path.display()))?;
                let account = self.builder()?.from_credentials(credentials).await.context("restoring the ACME account")?;
                // The account URL is what a CAA `accounturi` parameter pins (README).
                tracing::info!(target: "acme", "using ACME account {}", account.id());
                account
            }
            None => {
                let contact: Vec<&str> = self.settings.contact.iter().map(String::as_str).collect();
                // Running with --acme-domain is agreeing to the CA's subscriber agreement (README).
                let new = NewAccount { contact: &contact, terms_of_service_agreed: true, only_return_existing: false };
                let (account, credentials) = self
                    .builder()?
                    .create(&new, self.settings.directory.clone(), None)
                    .await
                    .context("creating the ACME account")?;
                let json = serde_json::to_vec(&credentials).context("serialising the ACME account")?;
                cache::write(&path, &json).with_context(|| format!("caching the ACME account in {}", path.display()))?;
                tracing::info!(target: "acme", "created ACME account {} (key cached in {})", account.id(), path.display());
                account
            }
        };
        *slot = Some(account.clone());
        Ok(account)
    }

    fn builder(&self) -> anyhow::Result<AccountBuilder> {
        match &self.settings.ca_root {
            Some(pem) => Account::builder_with_root(pem).with_context(|| format!("--acme-ca-root {}", pem.display())),
            None => Account::builder().context("the system's root certificates"),
        }
    }
}

/// The challenge certificates of one order; withdrawn however the order ends.
struct Armed<'a> {
    store: &'a Store,
    names: Vec<String>,
}

impl Armed<'_> {
    fn arm(&mut self, name: String, cert: Arc<rustls::sign::CertifiedKey>) {
        self.store.arm(name.clone(), cert);
        self.names.push(name);
    }
}

impl Drop for Armed<'_> {
    fn drop(&mut self) {
        for name in &self.names {
            self.store.disarm(name);
        }
    }
}

/// The CA's reasons for failed challenges, as `; mintaka.tj: urn:…:connection: …`.
async fn reasons(order: &mut Order) -> String {
    let mut out = String::new();
    let mut authorizations = order.authorizations();
    while let Some(Ok(mut authz)) = authorizations.next().await {
        let name = authz.identifier().to_string();
        let Ok(state) = authz.refresh().await else { continue };
        for problem in state.challenges.iter().filter_map(|c| c.error.as_ref()) {
            out.push_str(&format!("; {name}: {problem}"));
        }
    }
    out
}

/// Completes once the server starts shutting down.
async fn stopped(shutdown: &mut watch::Receiver<bool>) {
    let _ = shutdown.wait_for(|stopping| *stopping).await;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_a_lone_acme_tls_offer_is_a_challenge() {
        // A ClientHello can't be built by hand; drive rustls' own Acceptor with real ones.
        fn hello(alpn: &[&[u8]]) -> bool {
            let mut config = rustls::ClientConfig::builder_with_provider(Arc::new(crate::tls::provider()))
                .with_protocol_versions(&[&rustls::version::TLS13])
                .unwrap()
                .with_root_certificates(rustls::RootCertStore::empty())
                .with_no_client_auth();
            config.alpn_protocols = alpn.iter().map(|p| p.to_vec()).collect();
            let mut client = rustls::ClientConnection::new(Arc::new(config), "mintaka.tj".try_into().unwrap()).unwrap();
            let mut bytes = Vec::new();
            client.write_tls(&mut bytes).unwrap();
            let mut acceptor = rustls::server::Acceptor::default();
            acceptor.read_tls(&mut bytes.as_slice()).unwrap();
            let accepted = acceptor.accept().ok().flatten().expect("a complete ClientHello");
            is_challenge(&accepted.client_hello())
        }
        assert!(hello(&[ALPN]));
        assert!(!hello(&[]));
        assert!(!hello(&[b"h2", b"http/1.1"]));
        assert!(!hello(&[b"h2", ALPN]), "a browser-like offer stays on the normal path");
    }
}
