//! Aggregate load figures for `/_telemetry` (docs/CONTRACTS.md §6): request counters, latency
//! histograms, CPU and memory of this process, open connections.
//!
//! Requests record their server time into one of several mutex-guarded HDR histograms (sharded by
//! worker thread so cores don't contend on one lock). Once per wall-clock second a sampler task
//! folds the shards into a 10-slot ring, reads `/proc/self`, and publishes one pre-serialised
//! `event: tick` frame through a `watch` channel that every stream reads.

use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use bytes::Bytes;
use hdrhistogram::Histogram;
use proof::{TelemetryHello, TelemetryTick};
use tokio::sync::watch;

use crate::headers::micros;

/// Seconds of history behind the percentiles.
const WINDOW_SECS: usize = 10;
/// Highest latency the histograms track; longer requests saturate at this value.
const MAX_TRACKED_US: u64 = 60_000_000;
/// Two significant figures: exact below 256 µs, within 1 % above.
const SIGFIG: u8 = 2;

fn histogram() -> anyhow::Result<Histogram<u64>> {
    Ok(Histogram::new_with_bounds(1, MAX_TRACKED_US, SIGFIG)?)
}

/// Process-wide request and connection counters.
pub(crate) struct Metrics {
    req_total: AtomicU64,
    this_second: AtomicU64,
    open_connections: AtomicU64,
    shards: Box<[Mutex<Histogram<u64>>]>,
    next_shard: AtomicUsize,
    /// The 10-second history, owned by the sampler (allocated up front so sampling can't fail).
    window: Mutex<Window>,
    /// The latest `event: tick` frame (`None` until the first second has passed).
    ticks: watch::Sender<Option<Bytes>>,
}

thread_local! {
    static SHARD: std::cell::Cell<Option<usize>> = const { std::cell::Cell::new(None) };
}

impl Metrics {
    /// Counters with `shards` latency histograms (one per worker thread is plenty).
    pub fn new(shards: usize) -> anyhow::Result<Metrics> {
        let shards = (0..shards.max(1)).map(|_| histogram().map(Mutex::new)).collect::<Result<_, _>>()?;
        Ok(Metrics {
            req_total: AtomicU64::new(0),
            this_second: AtomicU64::new(0),
            open_connections: AtomicU64::new(0),
            shards,
            next_shard: AtomicUsize::new(0),
            window: Mutex::new(Window::new()?),
            ticks: watch::channel(None).0,
        })
    }

    /// Counts a completed request; its server time goes into the latency histograms unless the
    /// response is long-lived (an event stream's "time" is its lifetime, not a latency).
    pub fn record(&self, server_time: Duration, long_lived: bool) {
        self.req_total.fetch_add(1, Ordering::Relaxed);
        self.this_second.fetch_add(1, Ordering::Relaxed);
        if long_lived {
            return;
        }
        let shard = SHARD.with(|cell| match cell.get() {
            Some(shard) => shard,
            None => {
                let shard = self.next_shard.fetch_add(1, Ordering::Relaxed);
                cell.set(Some(shard));
                shard
            }
        }) % self.shards.len();
        let mut histogram = self.shards[shard].lock().unwrap_or_else(PoisonError::into_inner);
        histogram.saturating_record(micros(server_time).max(1));
    }

    /// Increments the open-connections gauge until the guard is dropped.
    pub fn connection_opened(&self) -> ConnectionGauge<'_> {
        self.open_connections.fetch_add(1, Ordering::Relaxed);
        ConnectionGauge(&self.open_connections)
    }

    /// A receiver of `event: tick` frames.
    pub fn subscribe(&self) -> watch::Receiver<Option<Bytes>> {
        self.ticks.subscribe()
    }
}

/// Decrements the open-connections gauge on drop.
pub(crate) struct ConnectionGauge<'a>(&'a AtomicU64);

impl Drop for ConnectionGauge<'_> {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::Relaxed);
    }
}

/// The `event: hello` frame.
pub(crate) fn hello_frame(hello: &TelemetryHello) -> Bytes {
    frame("hello", hello)
}

fn frame(event: &str, data: &impl serde::Serialize) -> Bytes {
    let json = serde_json::to_string(data).unwrap_or_else(|_| "null".to_owned());
    Bytes::from(format!("event: {event}\ndata: {json}\n\n"))
}

/// Runs until `shutdown` turns true: once per wall-clock second, publishes a tick.
pub(crate) async fn run_sampler(app: Arc<crate::app::AppState>, mut shutdown: watch::Receiver<bool>) {
    let metrics = &app.metrics;
    let mut cpu = CpuClock::new();
    let mut interval = tokio::time::interval_at(next_second(), Duration::from_secs(1));
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    loop {
        tokio::select! {
            _ = interval.tick() => {}
            _ = shutdown.changed() => return,
        }
        let rps = metrics.this_second.swap(0, Ordering::Relaxed);
        let (p50_us, p99_us, p999_us) =
            metrics.window.lock().unwrap_or_else(PoisonError::into_inner).rotate(&metrics.shards);
        let tick = TelemetryTick {
            t: unix_now().as_secs_f64().round() as u64,
            rps,
            p50_us,
            p99_us,
            p999_us,
            cpu_pct: cpu.percent(),
            rss_bytes: procfs::rss_bytes().unwrap_or(0),
            conns: metrics.open_connections.load(Ordering::Relaxed),
            req_total: metrics.req_total.load(Ordering::Relaxed),
        };
        metrics.ticks.send_replace(Some(frame("tick", &tick)));
    }
}

fn unix_now() -> Duration {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default()
}

/// The next whole second of wall-clock time, as a tokio instant.
fn next_second() -> tokio::time::Instant {
    let into_second = Duration::from_nanos(u64::from(unix_now().subsec_nanos()));
    tokio::time::Instant::now() + (Duration::from_secs(1) - into_second)
}

/// The last [`WINDOW_SECS`] one-second histograms.
struct Window {
    ring: Vec<Histogram<u64>>,
    current: usize,
    total: Histogram<u64>,
}

impl Window {
    fn new() -> anyhow::Result<Self> {
        let ring = (0..WINDOW_SECS).map(|_| histogram()).collect::<Result<_, _>>()?;
        Ok(Window { ring, current: 0, total: histogram()? })
    }

    /// Moves the shards' last second into the ring; returns p50/p99/p99.9 over the window.
    fn rotate(&mut self, shards: &[Mutex<Histogram<u64>>]) -> (Option<u64>, Option<u64>, Option<u64>) {
        self.current = (self.current + 1) % self.ring.len();
        let slot = &mut self.ring[self.current];
        slot.reset();
        for shard in shards {
            let mut shard = shard.lock().unwrap_or_else(PoisonError::into_inner);
            // Same bounds everywhere, so `add` cannot fail.
            let _ = slot.add(&*shard);
            shard.reset();
        }
        self.total.reset();
        for second in &self.ring {
            let _ = self.total.add(second);
        }
        if self.total.is_empty() {
            return (None, None, None);
        }
        let q = |q| Some(self.total.value_at_quantile(q));
        (q(0.5), q(0.99), q(0.999))
    }
}

/// CPU time of this process between two calls, as a percentage of one core.
struct CpuClock {
    last: Option<(Instant, u64)>,
    ticks_per_second: u64,
}

impl CpuClock {
    fn new() -> Self {
        CpuClock { last: procfs::cpu_ticks().map(|t| (Instant::now(), t)), ticks_per_second: procfs::clock_ticks() }
    }

    fn percent(&mut self) -> f64 {
        let now = Instant::now();
        let Some(ticks) = procfs::cpu_ticks() else { return 0.0 };
        let pct = match self.last {
            Some((then, before)) => {
                let wall = now.duration_since(then).as_secs_f64();
                let cpu = ticks.saturating_sub(before) as f64 / self.ticks_per_second as f64;
                if wall > 0.0 { cpu / wall * 100.0 } else { 0.0 }
            }
            None => 0.0,
        };
        self.last = Some((now, ticks));
        (pct * 10.0).round() / 10.0
    }
}

/// Readers for `/proc/self` (Linux). Plain file reads; `None` where unavailable.
pub(crate) mod procfs {
    use std::sync::OnceLock;

    const AT_PAGESZ: usize = 6;
    const AT_CLKTCK: usize = 17;

    /// `(AT_PAGESZ, AT_CLKTCK)` from the auxiliary vector the kernel gave this process; the same
    /// values `sysconf(_SC_PAGESIZE)` and `sysconf(_SC_CLK_TCK)` return, read without `unsafe`.
    fn auxv() -> (u64, u64) {
        static AUXV: OnceLock<(u64, u64)> = OnceLock::new();
        *AUXV.get_or_init(|| {
            let (mut page, mut clk) = (4096, 100);
            if let Ok(raw) = std::fs::read("/proc/self/auxv") {
                const W: usize = std::mem::size_of::<usize>();
                let (pairs, _) = raw.as_chunks::<{ 2 * W }>();
                for pair in pairs {
                    let word = |b: &[u8]| b.try_into().map(usize::from_ne_bytes).unwrap_or(0);
                    let (key, value) = (word(&pair[..W]), word(&pair[W..]));
                    match key {
                        AT_PAGESZ if value > 0 => page = value as u64,
                        AT_CLKTCK if value > 0 => clk = value as u64,
                        _ => {}
                    }
                }
            }
            (page, clk)
        })
    }

    /// Clock ticks per second (`USER_HZ`) for `/proc/self/stat`.
    pub fn clock_ticks() -> u64 {
        auxv().1
    }

    /// `utime + stime` of the whole process, in clock ticks.
    pub fn cpu_ticks() -> Option<u64> {
        let stat = std::fs::read_to_string("/proc/self/stat").ok()?;
        // The command name (field 2) is parenthesised and may contain spaces.
        let after_comm = &stat[stat.rfind(')')? + 1..];
        let mut fields = after_comm.split_ascii_whitespace().skip(11); // fields 3..=13
        let utime: u64 = fields.next()?.parse().ok()?;
        let stime: u64 = fields.next()?.parse().ok()?;
        Some(utime + stime)
    }

    /// Resident set size (`/proc/self/statm` field 2 × page size).
    pub fn rss_bytes() -> Option<u64> {
        let statm = std::fs::read_to_string("/proc/self/statm").ok()?;
        let pages: u64 = statm.split_ascii_whitespace().nth(1)?.parse().ok()?;
        Some(pages * auxv().0)
    }
}

/// Caps concurrent `/_telemetry` streams.
pub(crate) struct StreamLimit {
    active: Arc<AtomicUsize>,
    max: usize,
}

/// One admitted stream; releases its slot on drop.
pub(crate) struct StreamPermit(Arc<AtomicUsize>);

impl StreamLimit {
    pub fn new(max: usize) -> Self {
        StreamLimit { active: Arc::new(AtomicUsize::new(0)), max }
    }

    pub fn try_acquire(&self) -> Option<StreamPermit> {
        if self.active.fetch_add(1, Ordering::AcqRel) < self.max {
            Some(StreamPermit(Arc::clone(&self.active)))
        } else {
            self.active.fetch_sub(1, Ordering::AcqRel);
            None
        }
    }
}

impl Drop for StreamPermit {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::AcqRel);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn window_percentiles_cover_ten_seconds() {
        let metrics = Metrics::new(2).unwrap();
        let mut window = Window::new().unwrap();
        assert_eq!(window.rotate(&metrics.shards), (None, None, None));
        for us in 1..=100 {
            metrics.record(Duration::from_micros(us), false);
        }
        metrics.record(Duration::from_secs(3600), true); // long-lived: counted, not timed
        let (p50, p99, p999) = window.rotate(&metrics.shards);
        assert_eq!(p50, Some(50));
        assert_eq!(p99, Some(99));
        assert_eq!(p999, Some(100));
        assert_eq!(metrics.req_total.load(Ordering::Relaxed), 101);
        // The sample stays in the window for 10 rotations, then ages out.
        for _ in 0..9 {
            assert_eq!(window.rotate(&metrics.shards).0, Some(50));
        }
        assert_eq!(window.rotate(&metrics.shards).0, None);
    }

    #[test]
    fn stream_limit() {
        let limit = StreamLimit::new(2);
        let a = limit.try_acquire().unwrap();
        let _b = limit.try_acquire().unwrap();
        assert!(limit.try_acquire().is_none());
        drop(a);
        assert!(limit.try_acquire().is_some());
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn proc_readers() {
        assert!(procfs::rss_bytes().unwrap() > 0);
        assert!(procfs::cpu_ticks().is_some());
        assert!(procfs::clock_ticks() > 0);
    }

    #[test]
    fn connection_gauge() {
        let metrics = Metrics::new(1).unwrap();
        {
            let _a = metrics.connection_opened();
            let _b = metrics.connection_opened();
            assert_eq!(metrics.open_connections.load(Ordering::Relaxed), 2);
        }
        assert_eq!(metrics.open_connections.load(Ordering::Relaxed), 0);
    }
}
