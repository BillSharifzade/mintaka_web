//! What `/_proof` reports about a QUIC handshake, measured rather than assumed.
//!
//! quinn 0.11 hands out `quinn::crypto::rustls::HandshakeData`, which carries only the ALPN
//! protocol and the SNI name: the negotiated cipher suite is not exposed at all, and the
//! key-exchange group only behind quinn-proto's private `__rustls-post-quantum-test` feature
//! (whose accessor `expect`s, which with `panic = "abort"` would be a way to stop the server).
//! The rustls connection inside quinn's session is private too.
//!
//! So [`MeasuredConfig`] wraps quinn's own rustls crypto config and its sessions, delegating
//! every call, and keeps two things on the side:
//!
//! * the handshake bytes the server wrote in the Initial packet space — the HelloRetryRequest,
//!   if any, and the ServerHello — from which [`server_hello`] reads the cipher suite and the
//!   key-share group this connection really uses (nothing is inferred from configuration or from
//!   what the client offered; if the bytes cannot be read, both are reported as `null`);
//! * the instants the complete ClientHello arrived and the client's Finished was processed,
//!   so the handshake time means the same as on TCP.

use std::any::Any;
use std::sync::Arc;
use std::time::{Duration, Instant};

use quinn::crypto::rustls::{HandshakeData, QuicServerConfig};
use quinn::crypto::{self, ExportKeyingMaterialError, HeaderKey, KeyPair, Keys, PacketKey, UnsupportedVersion};
use quinn::{ConnectionId, Side};
use quinn_proto::TransportError;
use quinn_proto::transport_parameters::TransportParameters;
use rustls::{CipherSuite, NamedGroup};

/// The TLS facts of one QUIC connection, returned by `quinn::Connection::handshake_data()`.
#[derive(Debug, Clone)]
pub(crate) struct Handshake {
    /// The negotiated ALPN protocol (always `h3`: it is all the configuration offers).
    pub alpn: Option<Vec<u8>>,
    /// From our ServerHello; `None` if it could not be read.
    pub hello: Option<ServerHello>,
    /// Complete ClientHello → client Finished, measured on the server.
    pub duration: Option<Duration>,
}

/// The fields of a TLS 1.3 ServerHello that `/_proof` reports.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ServerHello {
    pub cipher: CipherSuite,
    /// From the `key_share` extension.
    pub group: Option<NamedGroup>,
}

/// quinn's rustls server config, with sessions that keep [`Handshake`] facts.
pub(crate) struct MeasuredConfig(pub Arc<QuicServerConfig>);

impl crypto::ServerConfig for MeasuredConfig {
    fn initial_keys(&self, version: u32, dst_cid: &ConnectionId) -> Result<Keys, UnsupportedVersion> {
        self.0.initial_keys(version, dst_cid)
    }

    fn retry_tag(&self, version: u32, orig_dst_cid: &ConnectionId, packet: &[u8]) -> [u8; 16] {
        self.0.retry_tag(version, orig_dst_cid, packet)
    }

    fn start_session(self: Arc<Self>, version: u32, params: &TransportParameters) -> Box<dyn crypto::Session> {
        Box::new(MeasuredSession {
            inner: Arc::clone(&self.0).start_session(version, params),
            initial: Some(Vec::new()),
            hello: None,
            client_hello: None,
            finished: None,
        })
    }
}

/// The Initial-space bytes we keep at most (an HRR plus a ServerHello with an ML-KEM key share
/// are about 1.3 kB).
const MAX_INITIAL: usize = 16 * 1024;

struct MeasuredSession {
    inner: Box<dyn crypto::Session>,
    /// Handshake bytes written before the first key change; `None` once read.
    initial: Option<Vec<u8>>,
    hello: Option<ServerHello>,
    client_hello: Option<Instant>,
    finished: Option<Instant>,
}

impl crypto::Session for MeasuredSession {
    fn initial_keys(&self, dst_cid: &ConnectionId, side: Side) -> Keys {
        self.inner.initial_keys(dst_cid, side)
    }

    fn handshake_data(&self) -> Option<Box<dyn Any>> {
        // `None` until quinn's own session has it (the HandshakeDataReady contract).
        let data = self.inner.handshake_data()?.downcast::<HandshakeData>().ok()?;
        let duration = match (self.client_hello, self.finished) {
            (Some(start), Some(end)) => Some(end.saturating_duration_since(start)),
            _ => None,
        };
        Some(Box::new(Handshake { alpn: data.protocol, hello: self.hello, duration }))
    }

    fn peer_identity(&self) -> Option<Box<dyn Any>> {
        self.inner.peer_identity()
    }

    fn early_crypto(&self) -> Option<(Box<dyn HeaderKey>, Box<dyn PacketKey>)> {
        self.inner.early_crypto()
    }

    fn early_data_accepted(&self) -> Option<bool> {
        self.inner.early_data_accepted()
    }

    fn is_handshaking(&self) -> bool {
        self.inner.is_handshaking()
    }

    fn read_handshake(&mut self, buf: &[u8]) -> Result<bool, TransportError> {
        let before = Instant::now();
        // `true` exactly once: the ClientHello is complete (quinn then knows the SNI/ALPN).
        let ready = self.inner.read_handshake(buf)?;
        if ready && self.client_hello.is_none() {
            self.client_hello = Some(before);
        }
        if self.finished.is_none() && !self.inner.is_handshaking() {
            self.finished = Some(Instant::now());
        }
        Ok(ready)
    }

    fn transport_parameters(&self) -> Result<Option<TransportParameters>, TransportError> {
        self.inner.transport_parameters()
    }

    fn write_handshake(&mut self, buf: &mut Vec<u8>) -> Option<Keys> {
        let start = buf.len();
        let keys = self.inner.write_handshake(buf);
        if let Some(initial) = &mut self.initial {
            initial.extend_from_slice(&buf[start..]);
            // rustls returns the Handshake keys together with the last Initial-space bytes.
            if keys.is_some() || initial.len() > MAX_INITIAL {
                self.hello = server_hello(initial);
                self.initial = None;
            }
        }
        keys
    }

    fn next_1rtt_keys(&mut self) -> Option<KeyPair<Box<dyn PacketKey>>> {
        self.inner.next_1rtt_keys()
    }

    fn is_valid_retry(&self, orig_dst_cid: &ConnectionId, header: &[u8], payload: &[u8]) -> bool {
        self.inner.is_valid_retry(orig_dst_cid, header, payload)
    }

    fn export_keying_material(&self, output: &mut [u8], label: &[u8], context: &[u8]) -> Result<(), ExportKeyingMaterialError> {
        self.inner.export_keying_material(output, label, context)
    }
}

/// `SHA-256("HelloRetryRequest")`: the `random` of a HelloRetryRequest (RFC 8446 §4.1.3).
const HRR_RANDOM: [u8; 32] = [
    0xcf, 0x21, 0xad, 0x74, 0xe5, 0x9a, 0x61, 0x11, 0xbe, 0x1d, 0x8c, 0x02, 0x1e, 0x65, 0xb8, 0x91, 0xc2, 0xa2, 0x11,
    0x16, 0x7a, 0xbb, 0x8c, 0x5e, 0x07, 0x9e, 0x09, 0xe2, 0xc8, 0xa8, 0x33, 0x9c,
];
const SERVER_HELLO: u8 = 2;
const KEY_SHARE: u16 = 51;

/// Reads the last ServerHello that is not a HelloRetryRequest from a run of TLS handshake
/// messages (`msg_type`, 24-bit length, body — CRYPTO frames carry them without records).
pub(crate) fn server_hello(mut bytes: &[u8]) -> Option<ServerHello> {
    let mut found = None;
    while let [kind, a, b, c, rest @ ..] = bytes {
        let len = usize::from(*a) << 16 | usize::from(*b) << 8 | usize::from(*c);
        let body = rest.get(..len)?;
        bytes = &rest[len..];
        if *kind == SERVER_HELLO
            && let Some((hrr, hello)) = parse_server_hello(body)
            && !hrr
        {
            found = Some(hello);
        }
    }
    found
}

/// One ServerHello body: `(is a HelloRetryRequest, fields)`.
fn parse_server_hello(body: &[u8]) -> Option<(bool, ServerHello)> {
    let mut r = Reader(body);
    r.take(2)?; // legacy_version
    let hrr = r.take(32)? == HRR_RANDOM;
    let session_id = usize::from(r.u8()?);
    r.take(session_id)?;
    let cipher = CipherSuite::from(r.u16()?);
    r.u8()?; // legacy_compression_method
    let mut extensions = Reader(r.vec16()?);
    let mut group = None;
    while !extensions.0.is_empty() {
        let kind = extensions.u16()?;
        let mut data = Reader(extensions.vec16()?);
        if kind == KEY_SHARE {
            // ServerHello: KeyShareEntry { group, key_exchange }; HelloRetryRequest: the group.
            group = Some(NamedGroup::from(data.u16()?));
        }
    }
    Some((hrr, ServerHello { cipher, group }))
}

struct Reader<'a>(&'a [u8]);

impl<'a> Reader<'a> {
    fn take(&mut self, n: usize) -> Option<&'a [u8]> {
        let (head, tail) = self.0.split_at_checked(n)?;
        self.0 = tail;
        Some(head)
    }

    fn u8(&mut self) -> Option<u8> {
        Some(self.take(1)?[0])
    }

    fn u16(&mut self) -> Option<u16> {
        let b = self.take(2)?;
        Some(u16::from_be_bytes([b[0], b[1]]))
    }

    fn vec16(&mut self) -> Option<&'a [u8]> {
        let n = usize::from(self.u16()?);
        self.take(n)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rustls::quic::{ClientConnection, Connection, ServerConnection, Version};

    /// A real rustls QUIC handshake in memory: client with `groups`, our server configuration.
    /// Returns the server's Initial-space bytes and what rustls itself says was negotiated.
    fn handshake(groups: Vec<&'static dyn rustls::crypto::SupportedKxGroup>) -> (Vec<u8>, CipherSuite, NamedGroup) {
        let settings = crate::config::TlsSettings {
            listen: "127.0.0.1:0".parse().unwrap(),
            source: crate::config::CertSource::SelfSigned,
            allow_tls12: false,
            http3: Some(Default::default()),
        };
        let setup = crate::tls::setup(&settings).unwrap();
        let mut provider = rustls::crypto::aws_lc_rs::default_provider();
        provider.kx_groups = groups;
        let mut client_config = rustls::ClientConfig::builder_with_provider(Arc::new(provider))
            .with_protocol_versions(&[&rustls::version::TLS13])
            .unwrap()
            .with_root_certificates(rustls::RootCertStore::empty())
            .with_no_client_auth();
        client_config.alpn_protocols = vec![b"h3".to_vec()];
        let mut client = Connection::Client(
            ClientConnection::new(Arc::new(client_config), Version::V1, "localhost".try_into().unwrap(), vec![]).unwrap(),
        );
        let mut server = Connection::Server(ServerConnection::new(setup.quic.unwrap(), Version::V1, vec![]).unwrap());

        let mut initial = Vec::new();
        let mut server_keyed = false;
        for _ in 0..4 {
            let mut to_server = Vec::new();
            client.write_hs(&mut to_server);
            if !to_server.is_empty() {
                server.read_hs(&to_server).unwrap();
            }
            let mut to_client = Vec::new();
            let keys = server.write_hs(&mut to_client);
            if !server_keyed {
                initial.extend_from_slice(&to_client);
                server_keyed = keys.is_some();
            }
            if !to_client.is_empty() {
                // The client cannot verify our self-signed certificate; it has seen what it needs.
                let _ = client.read_hs(&to_client);
            }
            if server_keyed {
                break;
            }
        }
        let Connection::Server(s) = &server else { unreachable!() };
        let suite = s.negotiated_cipher_suite().unwrap().suite();
        let group = s.negotiated_key_exchange_group().unwrap().name();
        (initial, suite, group)
    }

    #[test]
    fn reads_what_rustls_negotiated() {
        use rustls::crypto::aws_lc_rs::kx_group;
        let (bytes, suite, group) = handshake(vec![kx_group::X25519MLKEM768, kx_group::X25519]);
        assert_eq!(group, NamedGroup::X25519MLKEM768);
        assert_eq!(server_hello(&bytes), Some(ServerHello { cipher: suite, group: Some(group) }));

        let (bytes, suite, group) = handshake(vec![kx_group::X25519]);
        assert_eq!(server_hello(&bytes), Some(ServerHello { cipher: suite, group: Some(NamedGroup::X25519) }));
        assert_eq!(group, NamedGroup::X25519);
    }

    #[test]
    fn skips_a_hello_retry_request() {
        use rustls::crypto::aws_lc_rs::kx_group;
        // The client's first key share is a group we don't offer, so the server asks again.
        let (bytes, suite, group) = handshake(vec![kx_group::SECP384R1, kx_group::X25519]);
        assert_eq!(group, NamedGroup::X25519);
        let mut kinds = Vec::new();
        let mut rest = &bytes[..];
        while let [kind, a, b, c, tail @ ..] = rest {
            let len = usize::from(*a) << 16 | usize::from(*b) << 8 | usize::from(*c);
            kinds.push((*kind, parse_server_hello(&tail[..len]).map(|(hrr, _)| hrr)));
            rest = &tail[len..];
        }
        assert_eq!(kinds, [(SERVER_HELLO, Some(true)), (SERVER_HELLO, Some(false))], "HRR, then the ServerHello");
        assert_eq!(server_hello(&bytes), Some(ServerHello { cipher: suite, group: Some(NamedGroup::X25519) }));
    }

    #[test]
    fn truncated_or_foreign_bytes_report_nothing() {
        assert_eq!(server_hello(&[]), None);
        assert_eq!(server_hello(&[SERVER_HELLO, 0, 0, 40, 3, 3]), None, "length past the end");
        assert_eq!(server_hello(&[1, 0, 0, 0]), None, "a ClientHello type");
        let mut body = vec![3, 3];
        body.extend_from_slice(&[0; 32]);
        body.extend_from_slice(&[0, 0x13, 0x01, 0, 0, 0]);
        let mut msg = vec![SERVER_HELLO, 0, 0, body.len() as u8];
        msg.extend_from_slice(&body);
        assert_eq!(
            server_hello(&msg),
            Some(ServerHello { cipher: CipherSuite::TLS13_AES_128_GCM_SHA256, group: None }),
            "no key_share: no group"
        );
    }
}
