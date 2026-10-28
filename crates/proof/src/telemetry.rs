//! `GET /_telemetry` (Server-Sent Events): aggregated, anonymous load figures (docs/CONTRACTS.md §6).
//!
//! The stream is `event: hello` once, then `event: tick` once per second, plus a `: ping` comment
//! every 15 s. No IP addresses, paths or user agents ever appear in it.

use serde::{Deserialize, Serialize};

/// First event on the stream (`event: hello`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TelemetryHello {
    pub build: String,
    pub cores: u32,
    pub region: String,
}

/// One second of aggregated traffic (`event: tick`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TelemetryTick {
    /// Unix time of the end of the measured second.
    pub t: u64,
    /// Requests completed during the last full second.
    pub rps: u64,
    /// Server time (`app` in Server-Timing) percentiles over the last 10 s, in microseconds;
    /// `null` when there were no requests. Long-lived responses (this stream) are excluded.
    pub p50_us: Option<u64>,
    pub p99_us: Option<u64>,
    pub p999_us: Option<u64>,
    /// Process CPU time over the last second ÷ wall time × 100 (can exceed 100 on many cores).
    pub cpu_pct: f64,
    /// Resident set size of the process.
    pub rss_bytes: u64,
    /// Open client connections (TCP, including those still in the TLS handshake).
    pub conns: u64,
    /// Requests completed since the process started.
    pub req_total: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tick_round_trips_with_null_percentiles() {
        let json = r#"{"t":1793102400,"rps":0,"p50_us":null,"p99_us":null,"p999_us":null,
                       "cpu_pct":0.4,"rss_bytes":14680064,"conns":3,"req_total":12345}"#;
        let tick: TelemetryTick = serde_json::from_str(json).unwrap();
        assert_eq!(tick.p50_us, None);
        assert_eq!(tick.rss_bytes, 14_680_064);
        let back = serde_json::to_string(&tick).unwrap();
        assert!(back.contains(r#""p50_us":null"#));
    }
}
