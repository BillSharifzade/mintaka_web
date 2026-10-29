//! `GET /_proof`: what the server knows about the request it is answering (docs/CONTRACTS.md §5).
//!
//! Mirrored by hand in `web/shared/proof.ts`. Every field is filled live by the server for the
//! connection the request arrived on; nothing here is an example value.

use serde::{Deserialize, Serialize};

/// The `/_proof` response body.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Proof {
    pub server: ServerFacts,
    /// `null` on plain HTTP (local development).
    pub tls: Option<TlsFacts>,
    pub http: HttpFacts,
    pub connection: ConnectionFacts,
    pub privacy: PrivacyFacts,
    /// Lower-case names of the security headers sent on this very response (no HSTS over plain
    /// HTTP), in the order the contract lists them, then `alt-svc` when HTTP/3 is announced.
    pub headers: Vec<String>,
}

/// The machine and build answering the request.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ServerFacts {
    /// Configured deployment name, e.g. `fra-1` or `dev`.
    pub region: String,
    /// Where the machine physically is — configuration, not a measurement. `null` if unset.
    pub location: Option<Location>,
    /// The `build` of the loaded `dist/manifest.json`.
    pub build: String,
    /// e.g. `mintaka-server 0.1.0 · rustc 1.99.1 · axum 0.8 · rustls 0.23`.
    pub software: String,
    /// Server time spent on this request until the body was serialised, in microseconds.
    pub time_us: u64,
    pub uptime_s: u64,
    /// CPUs this process may run on (`std::thread::available_parallelism`).
    pub cores: u32,
}

/// A configured physical location (`--location "Dushanbe,TJ,38.5598,68.7870"`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Location {
    pub city: String,
    /// ISO 3166-1 alpha-2 code, e.g. `TJ`.
    pub country: String,
    pub lat: f64,
    pub lon: f64,
}

/// The TLS session of this connection, as negotiated by rustls.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TlsFacts {
    /// `TLSv1.3` or `TLSv1.2` (HTTP/3: always `TLSv1.3`, the only version QUIC allows).
    pub version: String,
    /// IANA-style rustls name, e.g. `TLS13_AES_128_GCM_SHA256`. `null` if the TLS stack did not
    /// report it (HTTP/3: read from our own ServerHello, `null` only if that failed).
    pub cipher: Option<String>,
    /// Key-exchange group, e.g. `X25519MLKEM768`. `null` like `cipher`.
    pub group: Option<String>,
    /// True for any hybrid or pure ML-KEM group (false when `group` is `null`).
    pub post_quantum: bool,
    /// Negotiated ALPN protocol (`h2`, `http/1.1`, `h3`), `null` if the client offered none.
    pub alpn: Option<String>,
    /// Server-side handshake time for this connection: from the complete ClientHello to the
    /// client's Finished (so it includes one network round trip), in microseconds.
    pub handshake_us: Option<u64>,
}

/// The HTTP version of this request.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HttpFacts {
    /// `HTTP/3.0`, `HTTP/2.0` or `HTTP/1.1`.
    pub version: String,
}

/// The connection (TCP, or QUIC for HTTP/3) this request arrived on.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConnectionFacts {
    /// Process-local counter, never derived from the client.
    pub id: u64,
    /// Requests received on this connection, including this one.
    pub requests: u64,
    /// The kernel's smoothed RTT for this socket (`TCP_INFO.tcpi_rtt`), `null` if unavailable
    /// — always on HTTP/3, which has no TCP socket.
    pub kernel_rtt_us: Option<u32>,
    /// `TCP_INFO.tcpi_rttvar`, `null` if unavailable.
    pub kernel_rttvar_us: Option<u32>,
    /// HTTP/3 only: QUIC's own smoothed RTT for this connection (quinn's RFC 9002 estimator,
    /// from acknowledgements of our packets), `null` on TCP.
    #[serde(default)]
    pub quic_rtt_us: Option<u32>,
}

/// What the server does not do with the visitor's data.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PrivacyFacts {
    pub cookies_set: u32,
    /// Origins other than our own that the Content-Security-Policy sent with this response allows.
    pub third_party_origins: u32,
    pub ip_stored: bool,
    pub ip_logged: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_the_contract_shape() {
        let json = r#"{
          "server": { "region": "dev",
            "location": { "city": "Dushanbe", "country": "TJ", "lat": 38.5598, "lon": 68.787 },
            "build": "2026-10-27.3f2a1b9c",
            "software": "mintaka-server 0.1.0 · rustc 1.99.1 · axum 0.8 · rustls 0.23",
            "time_us": 41, "uptime_s": 3600, "cores": 16 },
          "tls": { "version": "TLSv1.3", "cipher": "TLS13_AES_128_GCM_SHA256",
            "group": "X25519MLKEM768", "post_quantum": true, "alpn": "h2", "handshake_us": 1830 },
          "http": { "version": "HTTP/2.0" },
          "connection": { "id": 812, "requests": 3, "kernel_rtt_us": 48210, "kernel_rttvar_us": 1200 },
          "privacy": { "cookies_set": 0, "third_party_origins": 0, "ip_stored": false, "ip_logged": false },
          "headers": ["content-security-policy", "strict-transport-security"]
        }"#;
        let proof: Proof = serde_json::from_str(json).unwrap();
        assert_eq!(proof.tls.as_ref().unwrap().group.as_deref(), Some("X25519MLKEM768"));
        let back = serde_json::to_value(&proof).unwrap();
        assert_eq!(back["server"]["location"]["lon"], 68.787);
        assert_eq!(back["connection"]["kernel_rtt_us"], 48210);
        assert!(back["connection"]["quic_rtt_us"].is_null(), "TCP: present and null");
    }

    #[test]
    fn plain_http_has_null_tls() {
        let proof = Proof {
            server: ServerFacts {
                region: "dev".into(),
                location: None,
                build: "b".into(),
                software: "s".into(),
                time_us: 1,
                uptime_s: 0,
                cores: 1,
            },
            tls: None,
            http: HttpFacts { version: "HTTP/1.1".into() },
            connection: ConnectionFacts { id: 1, requests: 1, kernel_rtt_us: None, kernel_rttvar_us: None, quic_rtt_us: None },
            privacy: PrivacyFacts { cookies_set: 0, third_party_origins: 0, ip_stored: false, ip_logged: false },
            headers: vec![],
        };
        let v = serde_json::to_value(&proof).unwrap();
        assert!(v["tls"].is_null());
        assert!(v["server"]["location"].is_null());
        assert!(v["connection"]["kernel_rtt_us"].is_null());
    }

    #[test]
    fn http3_shape() {
        let json = r#"{
          "server": { "region": "dev", "location": null, "build": "b", "software": "s",
            "time_us": 30, "uptime_s": 1, "cores": 16 },
          "tls": { "version": "TLSv1.3", "cipher": "TLS13_AES_128_GCM_SHA256",
            "group": "X25519MLKEM768", "post_quantum": true, "alpn": "h3", "handshake_us": 900 },
          "http": { "version": "HTTP/3.0" },
          "connection": { "id": 9, "requests": 1, "kernel_rtt_us": null, "kernel_rttvar_us": null, "quic_rtt_us": 412 },
          "privacy": { "cookies_set": 0, "third_party_origins": 0, "ip_stored": false, "ip_logged": false },
          "headers": ["content-security-policy", "strict-transport-security", "alt-svc"]
        }"#;
        let proof: Proof = serde_json::from_str(json).unwrap();
        assert_eq!(proof.connection.quic_rtt_us, Some(412));
        assert_eq!(proof.connection.kernel_rtt_us, None);
        let unreported = r#"{ "version": "TLSv1.3", "cipher": null, "group": null, "post_quantum": false, "alpn": "h3", "handshake_us": null }"#;
        let tls: TlsFacts = serde_json::from_str(unreported).unwrap();
        assert_eq!((tls.cipher, tls.group), (None, None));
    }
}
