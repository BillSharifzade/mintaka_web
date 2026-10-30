//! The handshake timings in the Lab post on post-quantum TLS (`content/en/lab/3-post-quantum-tls.md`):
//! a TLS 1.3 client that times fresh handshakes against a running server.
//!
//! ```sh
//! cargo run --release -p server -- --dist crates/server/tests/fixtures/dist --data-dir /tmp/mintaka-tls \
//!     --listen 127.0.0.1:18880 --tls-self-signed --tls-listen 127.0.0.1:18843 &   # a server to measure
//! cargo run --release -p server --example handshakes [-- ADDR [ROUNDS]]   # 127.0.0.1:18843, 5,000
//! ```
//!
//! Each round makes one fresh connection per group, X25519MLKEM768 then X25519, each client
//! offering only that group, after 50 warm-up rounds. Resumption is off. The certificate the server
//! presents first is pinned for every later connection, and each handshake's signature is still
//! verified. Per connection: the client's own handshake time (ClientHello written → handshake done)
//! and the server's `tls;dur` from `Server-Timing` on a `GET /_ping`.
use rustls::client::Resumption;
use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::crypto::{CryptoProvider, WebPkiSupportedAlgorithms, aws_lc_rs, verify_tls12_signature, verify_tls13_signature};
use rustls::pki_types::{CertificateDer, ServerName, UnixTime};
use rustls::{ClientConfig, ClientConnection, DigitallySignedStruct, SignatureScheme};
use std::io::{Read, Write};
use std::net::TcpStream;
use std::sync::{Arc, OnceLock};
use std::time::Instant;

const WARM_UP: usize = 50;

/// Trusts the first certificate it sees (the server's self-signed one) and only that one.
#[derive(Debug)]
struct Pinned {
    der: OnceLock<Vec<u8>>,
    algs: WebPkiSupportedAlgorithms,
}

impl ServerCertVerifier for Pinned {
    fn verify_server_cert(&self, ee: &CertificateDer<'_>, _: &[CertificateDer<'_>], _: &ServerName<'_>, _: &[u8], _: UnixTime) -> Result<ServerCertVerified, rustls::Error> {
        if ee.as_ref() == self.der.get_or_init(|| ee.to_vec()).as_slice() {
            Ok(ServerCertVerified::assertion())
        } else {
            Err(rustls::Error::General("not the certificate this server presented first".into()))
        }
    }
    fn verify_tls12_signature(&self, m: &[u8], c: &CertificateDer<'_>, d: &DigitallySignedStruct) -> Result<HandshakeSignatureValid, rustls::Error> {
        verify_tls12_signature(m, c, d, &self.algs)
    }
    fn verify_tls13_signature(&self, m: &[u8], c: &CertificateDer<'_>, d: &DigitallySignedStruct) -> Result<HandshakeSignatureValid, rustls::Error> {
        verify_tls13_signature(m, c, d, &self.algs)
    }
    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.algs.supported_schemes()
    }
}

/// A socket that counts the bytes of the handshake in both directions.
struct Counted<'a> {
    s: &'a mut TcpStream,
    read: usize,
    written: usize,
}

impl Read for Counted<'_> {
    fn read(&mut self, b: &mut [u8]) -> std::io::Result<usize> {
        let n = self.s.read(b)?;
        self.read += n;
        Ok(n)
    }
}

impl Write for Counted<'_> {
    fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
        let n = self.s.write(b)?;
        self.written += n;
        Ok(n)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        self.s.flush()
    }
}

/// Nearest-rank quantile.
fn q(v: &[f64], p: f64) -> f64 {
    let mut v = v.to_vec();
    v.sort_by(f64::total_cmp);
    v[((v.len() - 1) as f64 * p).round() as usize]
}

fn main() {
    let mut args = std::env::args().skip(1);
    let addr = args.next().unwrap_or_else(|| "127.0.0.1:18843".into());
    let rounds: usize = args.next().map_or(5_000, |s| s.parse().expect("ROUNDS is a number"));
    let groups = [("X25519MLKEM768", aws_lc_rs::kx_group::X25519MLKEM768), ("X25519", aws_lc_rs::kx_group::X25519)];
    let base = aws_lc_rs::default_provider();
    let pinned = Arc::new(Pinned { der: OnceLock::new(), algs: base.signature_verification_algorithms });
    let configs: Vec<Arc<ClientConfig>> = groups
        .iter()
        .map(|(_, group)| {
            let provider = CryptoProvider { kx_groups: vec![*group], ..aws_lc_rs::default_provider() };
            let mut c = ClientConfig::builder_with_provider(Arc::new(provider))
                .with_protocol_versions(&[&rustls::version::TLS13])
                .expect("TLS 1.3")
                .dangerous()
                .with_custom_certificate_verifier(Arc::clone(&pinned) as Arc<dyn ServerCertVerifier>)
                .with_no_client_auth();
            c.resumption = Resumption::disabled();
            c.alpn_protocols = vec![b"http/1.1".to_vec()];
            Arc::new(c)
        })
        .collect();

    // A server started in the same breath may still be binding its port.
    for attempt in 0.. {
        match TcpStream::connect(&addr) {
            Ok(_) => break,
            Err(e) if attempt == 100 => panic!("connecting to {addr}: {e} (is the server running?)"),
            Err(_) => std::thread::sleep(std::time::Duration::from_millis(100)),
        }
    }

    let mut client = vec![vec![]; 2];
    let mut server = vec![vec![]; 2];
    let mut bytes = [(0, 0); 2];
    for round in 0..rounds + WARM_UP {
        for (i, cfg) in configs.iter().enumerate() {
            let mut sock = TcpStream::connect(&addr).unwrap_or_else(|e| panic!("connecting to {addr}: {e} (is the server running?)"));
            sock.set_nodelay(true).expect("TCP_NODELAY");
            let mut conn = ClientConnection::new(Arc::clone(cfg), "localhost".try_into().expect("a name")).expect("a client");
            let mut counted = Counted { s: &mut sock, read: 0, written: 0 };
            let t = Instant::now();
            while conn.is_handshaking() {
                conn.complete_io(&mut counted).expect("handshake");
            }
            let hs = t.elapsed().as_secs_f64() * 1e3;
            let (r, w) = (counted.read, counted.written);
            let mut tls = rustls::Stream::new(&mut conn, &mut sock);
            tls.write_all(b"GET /_ping HTTP/1.1\r\nHost: localhost\r\nUser-Agent: Mozilla/5.0 (handshake timing)\r\nConnection: close\r\n\r\n")
                .expect("request");
            let mut resp = Vec::new();
            let _ = tls.read_to_end(&mut resp); // `Connection: close`: read until the server hangs up
            let text = String::from_utf8_lossy(&resp);
            let srv = text
                .split("tls;dur=")
                .nth(1)
                .and_then(|r| r.split([';', ',', '\r']).next())
                .and_then(|v| v.parse::<f64>().ok())
                .expect("Server-Timing tls;dur on the first response");
            if round >= WARM_UP {
                client[i].push(hs);
                server[i].push(srv);
                bytes[i] = (r, w);
            }
        }
    }
    for (i, (name, _)) in groups.iter().enumerate() {
        println!(
            "{name:<15} n={} client handshake ms: median {:.3} p10 {:.3} p90 {:.3} p99 {:.3} | server tls;dur ms: median {:.3} p10 {:.3} p90 {:.3} p99 {:.3} | handshake bytes read {} written {} (last connection)",
            client[i].len(),
            q(&client[i], 0.5),
            q(&client[i], 0.1),
            q(&client[i], 0.9),
            q(&client[i], 0.99),
            q(&server[i], 0.5),
            q(&server[i], 0.1),
            q(&server[i], 0.9),
            q(&server[i], 0.99),
            bytes[i].0,
            bytes[i].1
        );
    }
    let d: Vec<f64> = client[0].iter().zip(&client[1]).map(|(a, b)| a - b).collect();
    let ds: Vec<f64> = server[0].iter().zip(&server[1]).map(|(a, b)| a - b).collect();
    println!("paired difference (X25519MLKEM768 − X25519), median: client {:.3} ms, server {:.3} ms", q(&d, 0.5), q(&ds, 0.5));
}
