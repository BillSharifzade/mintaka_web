// The server's view of this connection: `GET /_proof` (Cache-Control: no-store).
// Mirrors the Rust types in crates/proof; docs/CONTRACTS.md is the source of truth.

export interface Proof {
  server: {
    /** Configured deployment name, e.g. "fra-1" or "dev". */
    region: string;
    /** Where the machine physically is (configured, not measured). */
    location: { city: string; country: string; lat: number; lon: number } | null;
    build: string;
    /** e.g. "mintaka-server 0.1.0 · rustc 1.99.1 · axum 0.8 · rustls 0.23". */
    software: string;
    /** Time spent producing this /_proof response so far, in microseconds. */
    time_us: number;
    uptime_s: number;
    cores: number;
  };
  /** null when the connection is plain HTTP (local development). */
  tls: {
    version: string; // "TLSv1.3"
    cipher: string; // "TLS13_AES_128_GCM_SHA256"
    group: string; // "X25519MLKEM768"
    post_quantum: boolean;
    alpn: string | null; // "h2"
    handshake_us: number | null; // server-side handshake duration for this connection
  } | null;
  http: { version: string }; // "HTTP/2.0" | "HTTP/1.1"
  connection: {
    /** Process-local counter; not linked to the visitor. */
    id: number;
    /** Requests served on this connection, including this one. */
    requests: number;
    /** Smoothed RTT the Linux kernel measured on this TCP connection (TCP_INFO), µs. */
    kernel_rtt_us: number | null;
    kernel_rttvar_us: number | null;
  };
  privacy: {
    cookies_set: number;
    third_party_origins: number;
    ip_stored: boolean;
    ip_logged: boolean;
  };
  /** Lower-case names of the security headers the server sends on every response. */
  headers: string[];
}

let cached: Promise<Proof> | null = null;

/** Fetch `/_proof` once per page view; pass `{ fresh: true }` to re-measure. */
export function getProof(opts: { fresh?: boolean } = {}): Promise<Proof> {
  if (!cached || opts.fresh) {
    cached = fetch('/_proof', { cache: 'no-store', headers: { accept: 'application/json' } }).then(async (r) => {
      if (!r.ok) throw new Error(`/_proof answered ${r.status}`);
      return (await r.json()) as Proof;
    });
    cached.catch(() => {
      cached = null; // let the next caller retry
    });
  }
  return cached;
}

/** One second of aggregated traffic, from the `tick` events of `GET /_telemetry` (SSE). */
export interface TelemetryTick {
  /** Unix time, seconds. */
  t: number;
  /** Requests completed during the last second. */
  rps: number;
  /** Latency percentiles over the last 10 s, microseconds (null when there were no requests). */
  p50_us: number | null;
  p99_us: number | null;
  p999_us: number | null;
  /** Process CPU over the last second, percent of one core. */
  cpu_pct: number;
  rss_bytes: number;
  /** Open client connections. */
  conns: number;
  /** Requests since the process started. */
  req_total: number;
}

/** First event on the stream: `event: hello`. */
export interface TelemetryHello {
  build: string;
  cores: number;
  region: string;
}
