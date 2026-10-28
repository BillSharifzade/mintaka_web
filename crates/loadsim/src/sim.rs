//! A seeded discrete-event simulation of one service under load: Poisson arrivals, long-tailed
//! service times on `c` workers, clients that time out and retry, and — for the system built our
//! way — load shedding on queueing delay, backpressure and an autoscaler. Time is in milliseconds.
//!
//! [`Lab`] runs two [`System`]s side by side on the *same* arrival stream and reports one row of
//! numbers per window (see [`FIELDS`]).

use crate::rng::Rng;
use std::cmp::Ordering;
use std::collections::{BinaryHeap, VecDeque};

/// New requests per second at load ×1.
pub const BASE_RATE: f64 = 20.0;
/// Simulated time per reported window.
pub const WINDOW_MS: f64 = 250.0;
/// Percentiles cover the last 2 s of successes; Little's-law quantities the last 4 s.
const PCT_WINDOWS: usize = 8;
const LITTLE_WINDOWS: usize = 16;
/// Latency histogram: log buckets from 0.5 ms, 6 % wide, up to ≈ 57 s.
const BUCKETS: usize = 200;
const BUCKET_BASE: f64 = 0.5;
const BUCKET_RATIO: f64 = 1.06;

/// Numbers per system per window, in this order (rates are per second of simulated time):
///
/// | # | field | | # | field |
/// |---|---|---|---|---|
/// | 0 | new requests /s | | 8 | p99 latency of successes, ms (last 2 s; NaN if none) |
/// | 1 | offered /s (attempts, retries included) | | 9 | waiting in the queue (at the window's end) |
/// | 2 | retries /s | | 10 | busy workers (time average) |
/// | 3 | goodput /s (answered before the client gave up) | | 11 | workers running |
/// | 4 | shed /s (immediate 503s) | | 12 | workers starting |
/// | 5 | timed out /s (a client gave up) | | 13 | L: requests in the system, time average (last 4 s) |
/// | 6 | wasted /s (work finished after its client gave up) | | 14 | λ: requests leaving the system /s (last 4 s) |
/// | 7 | p50 latency of successes, ms (last 2 s; NaN if none) | | 15 | W: mean time in the system, ms (last 4 s) |
pub const FIELDS: usize = 16;
/// One window in [`Lab::step`]'s output: its end time (ms), then the naive system's fields, then ours.
pub const STRIDE: usize = 1 + 2 * FIELDS;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ServiceTime {
    Exponential,
    /// Lognormal with this σ of the underlying normal; the mean stays `mean_service_ms`.
    LogNormal { sigma: f64 },
}

/// CoDel-style shedding: once the oldest waiting request has waited longer than `target_ms` for
/// `interval_ms`, new requests get an immediate 503 and stale ones are dropped instead of served,
/// until the queue's delay is back under the target. Plus a hard queue limit (backpressure).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Shedding {
    pub target_ms: f64,
    pub interval_ms: f64,
    pub queue_per_worker: u32,
}

/// Every `tick_ms`, aim for `target_util` busy: start workers (ready after `start_ms`, up to
/// `max_workers`), and after `idle_ms` of needing fewer, retire one per tick.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Autoscale {
    pub max_workers: u32,
    pub tick_ms: f64,
    pub start_ms: f64,
    pub target_util: f64,
    pub idle_ms: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Config {
    /// Workers at the start, and the autoscaler's minimum.
    pub workers: u32,
    pub mean_service_ms: f64,
    pub service: ServiceTime,
    /// A client gives up on an attempt after this long (`f64::INFINITY`: never).
    pub timeout_ms: f64,
    /// Further attempts after a timeout. A 503 is final: the client is told to come back later.
    pub retries: u32,
    /// Retries go out after a uniform 0…`retry_jitter_ms` pause.
    pub retry_jitter_ms: f64,
    /// Memory guard only: attempts beyond this many waiting are lost (their clients time out).
    pub queue_cap: usize,
    pub shedding: Option<Shedding>,
    pub autoscale: Option<Autoscale>,
}

impl Config {
    /// One unbounded FIFO queue on a fixed pool; abandoned requests are still served.
    pub const fn naive() -> Config {
        Config {
            workers: 8,
            mean_service_ms: 20.0,
            service: ServiceTime::LogNormal { sigma: 0.8 },
            timeout_ms: 2000.0,
            retries: 2,
            retry_jitter_ms: 250.0,
            queue_cap: 10_000,
            shedding: None,
            autoscale: None,
        }
    }

    /// The same service and clients, with shedding, backpressure and autoscaling.
    pub const fn ours() -> Config {
        Config {
            shedding: Some(Shedding { target_ms: 50.0, interval_ms: 100.0, queue_per_worker: 4 }),
            autoscale: Some(Autoscale { max_workers: 32, tick_ms: 1000.0, start_ms: 5000.0, target_util: 0.7, idle_ms: 10_000.0 }),
            ..Config::naive()
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Handle {
    slot: u32,
    epoch: u32,
}

#[derive(Clone, Copy)]
enum Kind {
    Done(Handle),
    Timeout(Handle),
    Retry { origin: f64, tries: u32 },
    Ready,
    Tick,
}

struct Event {
    at: f64,
    seq: u64,
    kind: Kind,
}

// A min-heap on (time, insertion order): ties resolve the same way on every run.
impl Ord for Event {
    fn cmp(&self, other: &Self) -> Ordering {
        other.at.total_cmp(&self.at).then(other.seq.cmp(&self.seq))
    }
}
impl PartialOrd for Event {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl PartialEq for Event {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}
impl Eq for Event {}

/// One attempt by a client. `served`: the server is done with it; `answered`: the client is.
struct Attempt {
    origin: f64,
    enq: f64,
    tries: u32,
    epoch: u32,
    served: bool,
    answered: bool,
}

#[derive(Clone)]
struct Window {
    arrivals: u32,
    offered: u32,
    retries: u32,
    good: u32,
    shed: u32,
    timeouts: u32,
    wasted: u32,
    area_n: f64,
    area_busy: f64,
    departed: u32,
    sojourn: f64,
    span: f64,
    hist: Vec<u32>,
}

impl Window {
    fn new() -> Window {
        Window {
            arrivals: 0,
            offered: 0,
            retries: 0,
            good: 0,
            shed: 0,
            timeouts: 0,
            wasted: 0,
            area_n: 0.0,
            area_busy: 0.0,
            departed: 0,
            sojourn: 0.0,
            span: 0.0,
            hist: vec![0; BUCKETS],
        }
    }
}

/// Running totals since the start, for tests against closed forms.
#[derive(Clone, Copy, Debug, Default)]
pub struct Totals {
    pub time: f64,
    pub area_n: f64,
    pub departed: u64,
    pub sojourn: f64,
    pub good: u64,
}

pub struct System {
    cfg: Config,
    rng: Rng,
    now: f64,
    seq: u64,
    events: BinaryHeap<Event>,
    slab: Vec<Attempt>,
    free: Vec<u32>,
    queue: VecDeque<Handle>,
    in_system: u32,
    workers: u32,
    busy: u32,
    starting: u32,
    retiring: u32,
    above_since: Option<f64>,
    shedding: bool,
    tick_busy: f64,
    tick_shed: u32,
    low_since: Option<f64>,
    cur: Window,
    past: VecDeque<Window>,
    totals: Totals,
}

impl System {
    pub fn new(cfg: Config, seed: u64) -> System {
        let mut s = System {
            cfg,
            rng: Rng::new(seed),
            now: 0.0,
            seq: 0,
            events: BinaryHeap::new(),
            slab: Vec::new(),
            free: Vec::new(),
            queue: VecDeque::new(),
            in_system: 0,
            workers: cfg.workers,
            busy: 0,
            starting: 0,
            retiring: 0,
            above_since: None,
            shedding: false,
            tick_busy: 0.0,
            tick_shed: 0,
            low_since: None,
            cur: Window::new(),
            past: VecDeque::new(),
            totals: Totals::default(),
        };
        if let Some(a) = cfg.autoscale {
            s.push(a.tick_ms, Kind::Tick);
        }
        s
    }

    pub fn totals(&self) -> Totals {
        self.totals
    }

    pub fn workers(&self) -> u32 {
        self.workers
    }

    fn push(&mut self, at: f64, kind: Kind) {
        self.seq += 1;
        self.events.push(Event { at, seq: self.seq, kind });
    }

    /// Integrate the time averages up to `t`.
    fn advance(&mut self, t: f64) {
        let dt = t - self.now;
        if dt > 0.0 {
            let n = f64::from(self.in_system);
            let busy = f64::from(self.busy) * dt;
            self.cur.area_n += n * dt;
            self.cur.area_busy += busy;
            self.cur.span += dt;
            self.tick_busy += busy;
            self.totals.area_n += n * dt;
            self.totals.time += dt;
            self.now = t;
        }
    }

    /// Process everything up to `end`, with these new requests (sorted, all before `end`).
    pub fn run(&mut self, end: f64, arrivals: &[f64]) {
        let mut next = arrivals.iter().copied().peekable();
        loop {
            let event = self.events.peek().map(|e| e.at).filter(|&t| t <= end);
            match (next.peek().copied(), event) {
                (Some(a), Some(e)) if e < a => self.pop(),
                (Some(a), _) => {
                    next.next();
                    self.advance(a);
                    self.cur.arrivals += 1;
                    self.send(a, 0);
                }
                (None, Some(_)) => self.pop(),
                (None, None) => break,
            }
        }
        self.advance(end);
    }

    fn pop(&mut self) {
        let Some(e) = self.events.pop() else { return };
        self.advance(e.at);
        match e.kind {
            Kind::Done(h) => self.done(h),
            Kind::Timeout(h) => self.timeout(h),
            Kind::Retry { origin, tries } => self.send(origin, tries),
            Kind::Ready => {
                self.starting -= 1;
                self.workers += 1;
                self.dispatch();
            }
            Kind::Tick => self.tick(),
        }
    }

    fn service(&mut self) -> f64 {
        let mean = self.cfg.mean_service_ms;
        match self.cfg.service {
            ServiceTime::Exponential => self.rng.exp(mean),
            // E[exp(μ + σZ)] = exp(μ + σ²/2) = mean
            ServiceTime::LogNormal { sigma } => (mean.ln() - sigma * sigma / 2.0 + sigma * self.rng.normal()).exp(),
        }
    }

    fn alloc(&mut self, origin: f64, tries: u32) -> Handle {
        let fresh = |epoch| Attempt { origin, enq: 0.0, tries, epoch, served: false, answered: false };
        match self.free.pop() {
            Some(slot) => {
                let epoch = self.slab[slot as usize].epoch;
                self.slab[slot as usize] = fresh(epoch);
                Handle { slot, epoch }
            }
            None => {
                self.slab.push(fresh(0));
                Handle { slot: (self.slab.len() - 1) as u32, epoch: 0 }
            }
        }
    }

    fn get(&mut self, h: Handle) -> Option<&mut Attempt> {
        self.slab.get_mut(h.slot as usize).filter(|a| a.epoch == h.epoch)
    }

    /// Free the slot once both sides are done; a new generation makes old handles stale.
    fn release(&mut self, h: Handle) {
        let a = &mut self.slab[h.slot as usize];
        if a.epoch == h.epoch && a.served && a.answered {
            a.epoch = a.epoch.wrapping_add(1);
            self.free.push(h.slot);
        }
    }

    /// CoDel's rule on one observed queueing delay.
    fn codel(&mut self, wait: f64, s: Shedding) {
        if wait < s.target_ms {
            self.above_since = None;
            self.shedding = false;
        } else {
            match self.above_since {
                None => self.above_since = Some(self.now + s.interval_ms),
                Some(t) if self.now >= t => self.shedding = true,
                Some(_) => {}
            }
        }
    }

    fn head_wait(&self) -> f64 {
        self.queue.front().map_or(0.0, |h| self.now - self.slab[h.slot as usize].enq)
    }

    fn shed(&mut self) {
        self.cur.shed += 1;
        self.tick_shed += 1;
    }

    fn depart(&mut self, sojourn: f64) {
        self.in_system -= 1;
        self.cur.departed += 1;
        self.cur.sojourn += sojourn;
        self.totals.departed += 1;
        self.totals.sojourn += sojourn;
    }

    /// A client sends an attempt.
    fn send(&mut self, origin: f64, tries: u32) {
        self.cur.offered += 1;
        if tries > 0 {
            self.cur.retries += 1;
        }
        if let Some(s) = self.cfg.shedding {
            let wait = self.head_wait();
            self.codel(wait, s);
            let full = self.queue.len() >= (s.queue_per_worker * self.workers.max(1)) as usize;
            if self.shedding || full {
                return self.shed(); // an immediate 503: it never enters the system
            }
        }
        let h = self.alloc(origin, tries);
        if self.cfg.timeout_ms.is_finite() {
            self.push(self.now + self.cfg.timeout_ms, Kind::Timeout(h));
        }
        let now = self.now;
        let a = &mut self.slab[h.slot as usize];
        if self.queue.len() >= self.cfg.queue_cap {
            a.served = true; // lost to the memory guard; its client will time out
            return;
        }
        a.enq = now;
        self.queue.push_back(h);
        self.in_system += 1;
        self.dispatch();
    }

    /// Idle workers take requests from the head of the queue.
    fn dispatch(&mut self) {
        while self.busy < self.workers {
            let Some(h) = self.queue.pop_front() else { break };
            let wait = self.now - self.slab[h.slot as usize].enq;
            if let Some(s) = self.cfg.shedding {
                self.codel(wait, s);
                if self.shedding {
                    // Waited past the target while the queue stands: a 503 now beats a late answer.
                    let a = &mut self.slab[h.slot as usize];
                    a.served = true;
                    let tell = !a.answered;
                    a.answered = true;
                    if tell {
                        self.shed();
                    }
                    self.depart(wait);
                    self.release(h);
                    continue;
                }
            }
            self.busy += 1;
            let s = self.service();
            self.push(self.now + s, Kind::Done(h));
        }
    }

    fn done(&mut self, h: Handle) {
        self.busy -= 1;
        let now = self.now;
        let a = &mut self.slab[h.slot as usize];
        a.served = true;
        let (sojourn, latency, in_time) = (now - a.enq, now - a.origin, !a.answered);
        a.answered = true;
        if in_time {
            self.cur.good += 1;
            self.totals.good += 1;
            self.cur.hist[bucket(latency)] += 1;
        } else {
            self.cur.wasted += 1;
        }
        self.depart(sojourn);
        self.release(h);
        if self.retiring > 0 {
            self.retiring -= 1;
            self.workers -= 1;
        }
        self.dispatch();
    }

    fn timeout(&mut self, h: Handle) {
        let Some(a) = self.get(h) else { return };
        if a.answered {
            return;
        }
        a.answered = true;
        let (origin, tries) = (a.origin, a.tries);
        self.cur.timeouts += 1;
        self.release(h);
        if tries < self.cfg.retries {
            let pause = self.rng.unit() * self.cfg.retry_jitter_ms;
            self.push(self.now + pause, Kind::Retry { origin, tries: tries + 1 });
        }
    }

    fn tick(&mut self) {
        let Some(a) = self.cfg.autoscale else { return };
        // Work offered in the last tick, in workers' worth: what was served plus what was shed.
        let demand = (self.tick_busy + f64::from(self.tick_shed) * self.cfg.mean_service_ms) / a.tick_ms;
        let want = ((demand / a.target_util).ceil() as u32).clamp(self.cfg.workers, a.max_workers);
        let staying = self.workers - self.retiring;
        if want > staying + self.starting {
            let more = want - staying - self.starting;
            self.starting += more;
            for _ in 0..more {
                self.push(self.now + a.start_ms, Kind::Ready);
            }
            self.low_since = None;
        } else if want < staying && self.starting == 0 {
            let since = *self.low_since.get_or_insert(self.now);
            if self.now - since >= a.idle_ms {
                if self.busy < self.workers - self.retiring {
                    self.workers -= 1; // an idle worker goes now
                } else {
                    self.retiring += 1; // a busy one goes when it finishes
                }
            }
        } else {
            self.low_since = None;
        }
        self.tick_busy = 0.0;
        self.tick_shed = 0;
        self.push(self.now + a.tick_ms, Kind::Tick);
    }

    /// Close the current window and report it (see [`FIELDS`]).
    pub fn close(&mut self) -> [f64; FIELDS] {
        let w = std::mem::replace(&mut self.cur, Window::new());
        let per_s = if w.span > 0.0 { 1000.0 / w.span } else { 0.0 };
        let rate = |n: u32| f64::from(n) * per_s;
        let row = [
            rate(w.arrivals),
            rate(w.offered),
            rate(w.retries),
            rate(w.good),
            rate(w.shed),
            rate(w.timeouts),
            rate(w.wasted),
        ];
        let busy = if w.span > 0.0 { w.area_busy / w.span } else { 0.0 };
        self.past.push_back(w);
        if self.past.len() > LITTLE_WINDOWS {
            self.past.pop_front();
        }
        let mut hist = [0u32; BUCKETS];
        for w in self.past.iter().rev().take(PCT_WINDOWS) {
            for (sum, n) in hist.iter_mut().zip(&w.hist) {
                *sum += n;
            }
        }
        let (mut area, mut span, mut departed, mut sojourn) = (0.0, 0.0, 0u32, 0.0);
        for w in &self.past {
            area += w.area_n;
            span += w.span;
            departed += w.departed;
            sojourn += w.sojourn;
        }
        [
            row[0],
            row[1],
            row[2],
            row[3],
            row[4],
            row[5],
            row[6],
            percentile(&hist, 0.5),
            percentile(&hist, 0.99),
            self.queue.len() as f64,
            busy,
            f64::from(self.workers),
            f64::from(self.starting),
            if span > 0.0 { area / span } else { 0.0 },
            if span > 0.0 { f64::from(departed) * 1000.0 / span } else { 0.0 },
            if departed > 0 { sojourn / f64::from(departed) } else { f64::NAN },
        ]
    }
}

fn bucket(ms: f64) -> usize {
    if ms <= BUCKET_BASE {
        return 0;
    }
    ((ms / BUCKET_BASE).ln() / BUCKET_RATIO.ln()).floor().min((BUCKETS - 1) as f64) as usize
}

/// The `q` quantile of a histogram, at its bucket's geometric middle (NaN when empty).
pub fn percentile(hist: &[u32], q: f64) -> f64 {
    let total: u64 = hist.iter().map(|&n| u64::from(n)).sum();
    if total == 0 {
        return f64::NAN;
    }
    let rank = ((q * total as f64).ceil() as u64).max(1);
    let mut seen = 0u64;
    for (i, &n) in hist.iter().enumerate() {
        seen += u64::from(n);
        if seen >= rank {
            return BUCKET_BASE * BUCKET_RATIO.powf(i as f64 + 0.5);
        }
    }
    f64::NAN
}

/// Poisson arrivals whose rate can change at any moment (memoryless: just draw the next gap).
struct Arrivals {
    rng: Rng,
    per_ms: f64,
    next: f64,
}

impl Arrivals {
    fn set_rate(&mut self, now: f64, per_s: f64) {
        self.per_ms = (per_s / 1000.0).max(0.0);
        self.next = if self.per_ms > 0.0 { now + self.rng.exp(1.0 / self.per_ms) } else { f64::INFINITY };
    }

    fn until(&mut self, end: f64, out: &mut Vec<f64>) {
        while self.next < end {
            out.push(self.next);
            self.next += self.rng.exp(1.0 / self.per_ms);
        }
    }
}

/// The naive system and ours, facing the same stream of new requests.
pub struct Lab {
    pub naive: System,
    pub ours: System,
    arrivals: Arrivals,
    now: f64,
    next_window: f64,
    buf: Vec<f64>,
}

impl Lab {
    pub fn new(seed: u64) -> Lab {
        let mut r = Rng::new(seed);
        let mut lab = Lab {
            naive: System::new(Config::naive(), r.next_u64()),
            ours: System::new(Config::ours(), r.next_u64()),
            arrivals: Arrivals { rng: Rng::new(r.next_u64()), per_ms: 0.0, next: f64::INFINITY },
            now: 0.0,
            next_window: WINDOW_MS,
            buf: Vec::new(),
        };
        lab.set_rate(BASE_RATE);
        lab
    }

    pub fn now(&self) -> f64 {
        self.now
    }

    /// New requests per second, from now on.
    pub fn set_rate(&mut self, per_s: f64) {
        self.arrivals.set_rate(self.now, per_s);
    }

    /// Advance simulated time by `ms` (at most 10 s per call); returns every window that closed,
    /// [`STRIDE`] numbers each.
    pub fn step(&mut self, ms: f64) -> Vec<f64> {
        let end = self.now + ms.clamp(0.0, 10_000.0);
        let mut out = Vec::new();
        while self.now < end {
            let seg = end.min(self.next_window);
            self.arrivals.until(seg, &mut self.buf);
            self.naive.run(seg, &self.buf);
            self.ours.run(seg, &self.buf);
            self.buf.clear();
            self.now = seg;
            if seg >= self.next_window {
                out.push(self.next_window);
                out.extend(self.naive.close());
                out.extend(self.ours.close());
                self.next_window += WINDOW_MS;
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A plain M/M/c queue: exponential service, patient clients, no shedding.
    fn mmc(c: u32, mean_service_ms: f64) -> Config {
        Config {
            workers: c,
            mean_service_ms,
            service: ServiceTime::Exponential,
            timeout_ms: f64::INFINITY,
            retries: 0,
            retry_jitter_ms: 0.0,
            queue_cap: usize::MAX,
            shedding: None,
            autoscale: None,
        }
    }

    /// Run one system on Poisson arrivals for `seconds`; mean time in system (ms) and totals.
    fn run(cfg: Config, per_s: f64, seconds: f64, seed: u64) -> (f64, Totals) {
        let mut sys = System::new(cfg, seed);
        let mut arr = Arrivals { rng: Rng::new(seed ^ 0xa5a5), per_ms: 0.0, next: 0.0 };
        arr.set_rate(0.0, per_s);
        let mut buf = Vec::new();
        let mut t = 0.0;
        while t < seconds * 1000.0 {
            t += 1000.0;
            arr.until(t, &mut buf);
            sys.run(t, &buf);
            buf.clear();
            sys.close();
        }
        let tot = sys.totals();
        (tot.sojourn / tot.departed as f64, tot)
    }

    fn close(actual: f64, expected: f64, tol: f64, what: &str) {
        let err = (actual - expected).abs() / expected;
        assert!(err <= tol, "{what}: {actual:.3} vs {expected:.3} ({:.1} % off)", err * 100.0);
    }

    /// Erlang C: the chance an arrival waits in M/M/c with offered load a = λ/μ.
    fn erlang_c(c: u32, a: f64) -> f64 {
        let mut term = 1.0;
        let mut sum = 0.0;
        for k in 0..c {
            if k > 0 {
                term *= a / f64::from(k);
            }
            sum += term;
        }
        let top = term * a / f64::from(c) * (f64::from(c) / (f64::from(c) - a));
        top / (sum + top)
    }

    #[test]
    fn mm1_matches_the_closed_form() {
        // λ = 25/s, μ = 50/s: ρ = 0.5, W = 1/(μ − λ) = 40 ms, L = ρ/(1 − ρ) = 1.
        for seed in [1, 2] {
            let (w, tot) = run(mmc(1, 20.0), 25.0, 4000.0, seed);
            close(w, 40.0, 0.05, "M/M/1 W");
            close(tot.area_n / tot.time, 1.0, 0.06, "M/M/1 L");
        }
    }

    #[test]
    fn mmc_matches_erlang_c() {
        // c = 4, λ = 120/s, μ = 50/s: a = 2.4, ρ = 0.6.
        let (c, lambda, mu) = (4, 120.0 / 1000.0, 50.0 / 1000.0);
        let a = lambda / mu;
        let wq = erlang_c(c, a) / (f64::from(c) * mu - lambda);
        let w = wq + 1.0 / mu;
        let (measured, tot) = run(mmc(c, 20.0), 120.0, 3000.0, 3);
        close(measured, w, 0.05, "M/M/4 W");
        close(measured - 20.0, wq, 0.12, "M/M/4 Wq");
        close(tot.area_n / tot.time, lambda * w, 0.06, "M/M/4 L = λW");
        assert!((erlang_c(1, 0.5) - 0.5).abs() < 1e-12, "Erlang C for one server is ρ");
    }

    #[test]
    fn lognormal_service_keeps_its_mean() {
        let mut s = System::new(Config::naive(), 9);
        let n = 200_000;
        let mean: f64 = (0..n).map(|_| s.service()).sum::<f64>() / f64::from(n);
        close(mean, 20.0, 0.02, "lognormal mean");
    }

    #[test]
    fn percentiles_from_the_histogram() {
        let mut hist = vec![0u32; BUCKETS];
        for ms in 1..=1000 {
            hist[bucket(f64::from(ms))] += 1;
        }
        close(percentile(&hist, 0.5), 500.0, 0.06, "p50");
        close(percentile(&hist, 0.99), 990.0, 0.06, "p99");
        assert!(percentile(&vec![0; BUCKETS], 0.5).is_nan());
        assert_eq!(bucket(0.1), 0);
        assert_eq!(bucket(1e9), BUCKETS - 1);
    }

    /// Run `seconds` in 1 s steps, as the page does (one call may cover at most 10 s).
    fn windows(lab: &mut Lab, seconds: f64) -> Vec<f64> {
        let mut out = Vec::new();
        for _ in 0..seconds as u32 {
            out.extend(lab.step(1000.0));
        }
        out
    }

    /// Mean of field `f` for system `sys` (0 naive, 1 ours) over the last `n` windows.
    fn avg(out: &[f64], sys: usize, f: usize, n: usize) -> f64 {
        let rows: Vec<&[f64]> = out.chunks(STRIDE).collect();
        let tail = &rows[rows.len() - n..];
        tail.iter().map(|r| r[1 + sys * FIELDS + f]).sum::<f64>() / n as f64
    }

    #[test]
    fn same_seed_same_run() {
        let go = |seed| {
            let mut lab = Lab::new(seed);
            let mut out = lab.step(3000.0);
            lab.set_rate(600.0);
            out.extend(lab.step(4000.0));
            lab.set_rate(20.0);
            out.extend(lab.step(1234.5));
            out
        };
        let (a, b, c) = (go(42), go(42), go(43));
        assert_eq!(a.len(), b.len());
        assert!(a.iter().zip(&b).all(|(x, y)| x == y || (x.is_nan() && y.is_nan())));
        assert!(a.iter().zip(&c).any(|(x, y)| x != y && !(x.is_nan() && y.is_nan())));
        assert_eq!(a.len() % STRIDE, 0);
        assert_eq!(a.len() / STRIDE, (8234.5 / WINDOW_MS) as usize);
    }

    #[test]
    fn both_systems_see_the_same_new_requests() {
        let mut lab = Lab::new(5);
        lab.set_rate(400.0);
        let out = windows(&mut lab, 5.0);
        for row in out.chunks(STRIDE) {
            assert_eq!(row[1], row[1 + FIELDS]);
        }
    }

    #[test]
    fn at_low_load_both_answer_everything_quickly() {
        let mut lab = Lab::new(7);
        let out = windows(&mut lab, 20.0); // ×1: 20 requests/s on 8 workers
        for sys in [0, 1] {
            let (new, good, shed, lost) = (avg(&out, sys, 0, 40), avg(&out, sys, 3, 40), avg(&out, sys, 4, 40), avg(&out, sys, 5, 40));
            close(good, new, 0.1, "goodput ≈ demand");
            assert_eq!(shed + lost, 0.0);
            assert!(avg(&out, sys, 8, 8) < 200.0, "p99 stays short");
        }
    }

    #[test]
    fn overload_collapses_the_naive_system_but_not_ours() {
        let mut lab = Lab::new(11);
        lab.set_rate(BASE_RATE * 40.0); // ×40 = 800/s; the naive pool serves at most ≈ 400/s
        let out = windows(&mut lab, 40.0);
        let n = 20; // the last 5 s
        // Naive: almost nothing answered in time, retries amplify the load, abandoned work wasted.
        assert!(avg(&out, 0, 3, n) < 0.1 * 800.0, "naive goodput {}", avg(&out, 0, 3, n));
        assert!(avg(&out, 0, 1, n) > 1.5 * 800.0, "retries amplify the offered load");
        assert!(avg(&out, 0, 6, n) > 300.0, "the pool is busy with abandoned work");
        assert!(avg(&out, 0, 10, n) > 7.9, "and every worker is busy");
        // Ours: scaled out, answers what it accepts quickly, refuses the rest with a 503.
        assert!(avg(&out, 1, 11, n) > 16.0, "autoscaled to {}", avg(&out, 1, 11, n));
        assert!(avg(&out, 1, 3, n) > 0.85 * 800.0, "our goodput {}", avg(&out, 1, 3, n));
        assert!(avg(&out, 1, 8, n) < 250.0, "our p99 {}", avg(&out, 1, 8, n));
        assert!(avg(&out, 1, 5, n) < 1.0, "no timeouts");
    }

    #[test]
    fn beyond_the_cap_ours_sheds_and_its_p99_stays_flat() {
        let mut lab = Lab::new(13);
        lab.set_rate(BASE_RATE * 100.0); // 2000/s: needs ≈ 40 busy workers, the cap is 32
        let out = windows(&mut lab, 30.0);
        let n = 20;
        assert_eq!(avg(&out, 1, 11, n), 32.0);
        assert!(avg(&out, 1, 4, n) > 200.0, "the excess gets 503s");
        assert!(avg(&out, 1, 8, n) < 250.0, "p99 of accepted {}", avg(&out, 1, 8, n));
        // Early on, before new workers boot, shedding alone holds the line.
        let first: Vec<f64> = out.chunks(STRIDE).take(16).map(|r| r[1 + FIELDS + 8]).filter(|v| !v.is_nan()).collect();
        assert!(first.iter().all(|&p| p < 300.0), "{first:?}");
    }

    #[test]
    fn ours_scales_back_in_when_idle() {
        let mut lab = Lab::new(17);
        lab.set_rate(BASE_RATE * 40.0);
        windows(&mut lab, 20.0);
        assert!(lab.ours.workers() > 16);
        lab.set_rate(BASE_RATE);
        windows(&mut lab, 60.0);
        assert_eq!(lab.ours.workers(), 8);
    }

    #[test]
    fn little_holds_when_steady_and_breaks_in_collapse() {
        let mut lab = Lab::new(19);
        lab.set_rate(BASE_RATE * 10.0);
        let out = windows(&mut lab, 30.0);
        for sys in [0, 1] {
            let (l, lambda, w) = (avg(&out, sys, 13, 1), avg(&out, sys, 14, 1), avg(&out, sys, 15, 1));
            close(l, lambda * w / 1000.0, 0.1, "L = λW, steady");
        }
        lab.set_rate(BASE_RATE * 60.0);
        let out = windows(&mut lab, 6.0);
        let (l, lambda, w) = (avg(&out, 0, 13, 1), avg(&out, 0, 14, 1), avg(&out, 0, 15, 1));
        assert!(l > 1.5 * lambda * w / 1000.0, "a growing queue: L {l} > λW {}", lambda * w / 1000.0);
    }

    #[test]
    fn no_arrivals_at_rate_zero() {
        let mut lab = Lab::new(23);
        lab.set_rate(0.0);
        let out = windows(&mut lab, 5.0);
        assert!(out.chunks(STRIDE).all(|r| r[1] == 0.0 && r[1 + FIELDS] == 0.0));
    }

    #[test]
    fn one_step_covers_at_most_ten_seconds() {
        let mut lab = Lab::new(31);
        assert_eq!(lab.step(60_000.0).len(), 40 * STRIDE);
        assert_eq!(lab.now(), 10_000.0);
        assert!(lab.step(-5.0).is_empty());
    }

    #[test]
    fn step_returns_whole_windows_only() {
        let mut lab = Lab::new(29);
        assert!(lab.step(100.0).is_empty());
        assert_eq!(lab.step(200.0).len(), STRIDE); // crosses 250 ms
        assert_eq!(lab.step(0.0).len(), 0);
        assert_eq!(lab.now(), 300.0);
    }
}
