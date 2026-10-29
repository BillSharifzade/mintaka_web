//! The four puzzles, generated from a per-run seed, and how answers are compared.
//!
//! Each generator is deterministic in its seed, so the server never stores a puzzle: it makes it
//! again to show it and again to check an answer. A generator that must avoid an unlucky instance
//! asks for the seed of the next attempt.

use std::cmp::Reverse;
use std::collections::BinaryHeap;
use std::fmt::Write as _;

use proof::{CTF_CRIB, CtfPuzzle};

/// The puzzle of `stage` (1–4) and its answer, normalised as [`accepts`] compares it.
pub(super) fn generate(stage: u8, seed: impl Fn(u32) -> [u8; 32]) -> (CtfPuzzle, String) {
    match stage {
        1 => rigel(seed),
        2 => saiph(&mut Rng::new(seed(0))),
        3 => betelgeuse(&mut Rng::new(seed(0))),
        _ => meissa(seed),
    }
}

/// Whether `flag` is the answer. Forgiving about form (`mintaka{…}` around it, digit grouping,
/// a unit after the number, the whole decrypted sentence), never about content.
pub(super) fn accepts(stage: u8, answer: &str, flag: &str) -> bool {
    let flag = flag.trim();
    let flag = flag.strip_prefix("mintaka{").and_then(|f| f.strip_suffix('}')).unwrap_or(flag).trim();
    match stage {
        1 | 3 => {
            let digits: String = flag.chars().filter(|c| !matches!(c, '_' | ',' | ' ' | '\u{a0}' | '\u{202f}')).collect();
            digits.parse::<u64>().ok().zip(answer.parse::<u64>().ok()).is_some_and(|(a, b)| a == b)
        }
        2 => flag
            .split_whitespace()
            .last()
            .map(|w| w.trim_matches(|c: char| !c.is_ascii_alphanumeric()).to_ascii_lowercase())
            .is_some_and(|w| w == answer),
        4 => flag
            .split_whitespace()
            .next()
            .and_then(|n| n.parse::<f64>().ok())
            .is_some_and(|x| x.is_finite() && format!("{x:.1}") == answer),
        _ => false,
    }
}

/// xoshiro256**: small, fast, and fully specified, so a seed means the same puzzle forever.
struct Rng([u64; 4]);

impl Rng {
    fn new(seed: [u8; 32]) -> Rng {
        let word = |i: usize| u64::from_le_bytes(seed[i * 8..i * 8 + 8].try_into().unwrap_or([0; 8]));
        let s = [word(0), word(1), word(2), word(3)];
        // The all-zero state is the one state xoshiro cannot leave.
        Rng(if s == [0; 4] { [1, 0, 0, 0] } else { s })
    }

    fn next(&mut self) -> u64 {
        let s = &mut self.0;
        let result = s[1].wrapping_mul(5).rotate_left(7).wrapping_mul(9);
        let t = s[1] << 17;
        s[2] ^= s[0];
        s[3] ^= s[1];
        s[1] ^= s[2];
        s[0] ^= s[3];
        s[2] ^= t;
        s[3] = s[3].rotate_left(45);
        result
    }

    /// Uniform in `0..n` (multiply-shift; the bias is far below anything a puzzle can show).
    fn below(&mut self, n: u64) -> u64 {
        ((u128::from(self.next()) * u128::from(n)) >> 64) as u64
    }

    /// Uniform in `lo..=hi`.
    fn range(&mut self, lo: u64, hi: u64) -> u64 {
        lo + self.below(hi - lo + 1)
    }

    /// Uniform in `[0, 1)`.
    fn unit(&mut self) -> f64 {
        (self.next() >> 11) as f64 / (1u64 << 53) as f64
    }

    /// Standard normal (Box–Muller).
    fn gauss(&mut self) -> f64 {
        let u = 1.0 - self.unit();
        (-2.0 * u.ln()).sqrt() * (std::f64::consts::TAU * self.unit()).cos()
    }
}

fn hex(bytes: impl IntoIterator<Item = u8>) -> String {
    bytes.into_iter().fold(String::new(), |mut s, b| {
        let _ = write!(s, "{b:02x}");
        s
    })
}

// ── 1 · Rigel, efficiency: the cheapest route across a sky of nebula densities ───────────────

/// Side of the square sky.
const SKY: usize = 100;

fn rigel(seed: impl Fn(u32) -> [u8; 32]) -> (CtfPuzzle, String) {
    let mut attempt = 0;
    loop {
        let mut rng = Rng::new(seed(attempt));
        let mut grid: Vec<u8> = (0..SKY * SKY).map(|_| rng.range(4, 9) as u8).collect();
        // Cheap rivers wander through the nebula, east–west and north–south. They mostly go their
        // way but sometimes turn back, so the cheapest route doubles back too, and a sweep that
        // only moves right and down (the tempting shortcut) gets a different number.
        for river in 0..8 {
            let across = river % 2 == 0;
            let (mut along, mut side) = (0i64, rng.below(SKY as u64) as i64);
            while along < SKY as i64 {
                let (r, c) = if across { (side, along) } else { (along, side) };
                grid[r as usize * SKY + c as usize] = rng.range(1, 2) as u8;
                match rng.below(10) {
                    0..=4 => along += 1,
                    5 | 6 => side -= 1,
                    7 | 8 => side += 1,
                    _ => along = (along - 1).max(0),
                }
                side = side.clamp(0, SKY as i64 - 1);
            }
        }
        let best = cheapest(&grid, SKY);
        if best != sweep(&grid, SKY) || attempt == 63 {
            let rows: Vec<String> = grid.chunks(SKY).map(|row| row.iter().map(|&d| char::from(b'0' + d)).collect()).collect();
            return (CtfPuzzle::Rigel { grid: rows.join("\n") }, best.to_string());
        }
        attempt += 1;
    }
}

/// Dijkstra from the top-left cell (free) to the bottom-right one (paid), four neighbours.
fn cheapest(grid: &[u8], n: usize) -> u32 {
    let mut dist = vec![u32::MAX; n * n];
    let mut queue = BinaryHeap::from([Reverse((0u32, 0usize))]);
    dist[0] = 0;
    while let Some(Reverse((d, at))) = queue.pop() {
        if at == n * n - 1 {
            return d;
        }
        if d > dist[at] {
            continue;
        }
        let (r, c) = (at / n, at % n);
        let next = [(r > 0).then(|| at - n), (r + 1 < n).then(|| at + n), (c > 0).then(|| at - 1), (c + 1 < n).then(|| at + 1)];
        for to in next.into_iter().flatten() {
            let nd = d + u32::from(grid[to]);
            if nd < dist[to] {
                dist[to] = nd;
                queue.push(Reverse((nd, to)));
            }
        }
    }
    dist[n * n - 1]
}

/// The shortcut that looks right: dynamic programming over right and down moves only.
fn sweep(grid: &[u8], n: usize) -> u32 {
    let mut best = vec![0u32; n * n];
    for r in 0..n {
        for c in 0..n {
            let at = r * n + c;
            best[at] = match (r, c) {
                (0, 0) => 0,
                (0, _) => best[at - 1] + u32::from(grid[at]),
                (_, 0) => best[at - n] + u32::from(grid[at]),
                _ => best[at - 1].min(best[at - n]) + u32::from(grid[at]),
            };
        }
    }
    best[n * n - 1]
}

// ── 2 · Saiph, security: two messages under one one-time pad ─────────────────────────────────

/// No 0/o, 1/l/i: the word is read off a decrypted line by people.
const WORD_LETTERS: &[u8] = b"abcdefghjkmnpqrstuvwxyz23456789";

fn saiph(rng: &mut Rng) -> (CtfPuzzle, String) {
    let word: String = (0..12).map(|_| char::from(WORD_LETTERS[rng.below(WORD_LETTERS.len() as u64) as usize])).collect();
    let secret = format!("Saiph holds. Second flag: {word}");
    let pad: Vec<u8> = (0..CTF_CRIB.len().max(secret.len())).map(|_| rng.next() as u8).collect();
    let a = hex(CTF_CRIB.bytes().zip(&pad).map(|(m, k)| m ^ k));
    let b = hex(secret.bytes().zip(&pad).map(|(m, k)| m ^ k));
    (CtfPuzzle::Saiph { a, b }, word)
}

// ── 3 · Betelgeuse, highload: what one connection's congestion window allows ─────────────────

/// Linux's `struct tcp_info` as the `libc` crate declares it for x86-64 glibc: 280 bytes,
/// little-endian. The kernel only ever appends to it, so older headers agree on every field the
/// puzzle needs.
const TCP_INFO_LEN: usize = 280;

/// Byte offsets of the fields the puzzle fills (checked against `libc::tcp_info` in the tests).
mod off {
    pub const STATE: usize = 0;
    pub const OPTIONS: usize = 5;
    pub const WSCALE: usize = 6;
    pub const RTO: usize = 8;
    pub const ATO: usize = 12;
    pub const SND_MSS: usize = 16;
    pub const RCV_MSS: usize = 20;
    pub const UNACKED: usize = 24;
    pub const LAST_DATA_SENT: usize = 44;
    pub const LAST_DATA_RECV: usize = 52;
    pub const LAST_ACK_RECV: usize = 56;
    pub const PMTU: usize = 60;
    pub const RCV_SSTHRESH: usize = 64;
    pub const RTT: usize = 68;
    pub const RTTVAR: usize = 72;
    pub const SND_SSTHRESH: usize = 76;
    pub const SND_CWND: usize = 80;
    pub const ADVMSS: usize = 84;
    pub const REORDERING: usize = 88;
    pub const RCV_RTT: usize = 92;
    pub const RCV_SPACE: usize = 96;
    pub const TOTAL_RETRANS: usize = 100;
    pub const PACING_RATE: usize = 104;
    pub const MAX_PACING_RATE: usize = 112;
    pub const BYTES_ACKED: usize = 120;
    pub const BYTES_RECEIVED: usize = 128;
    pub const SEGS_OUT: usize = 136;
    pub const SEGS_IN: usize = 140;
    pub const MIN_RTT: usize = 148;
    pub const DATA_SEGS_IN: usize = 152;
    pub const DATA_SEGS_OUT: usize = 156;
    pub const DELIVERY_RATE: usize = 160;
    pub const BUSY_TIME: usize = 168;
    pub const DELIVERED: usize = 192;
    pub const BYTES_SENT: usize = 200;
    pub const BYTES_RETRANS: usize = 208;
    pub const SND_WND: usize = 228;
    pub const RCV_WND: usize = 232;
}

fn betelgeuse(rng: &mut Rng) -> (CtfPuzzle, String) {
    let mut b = [0u8; TCP_INFO_LEN];
    let put32 = |b: &mut [u8; TCP_INFO_LEN], at: usize, v: u64| b[at..at + 4].copy_from_slice(&(v as u32).to_le_bytes());
    let put64 = |b: &mut [u8; TCP_INFO_LEN], at: usize, v: u64| b[at..at + 8].copy_from_slice(&v.to_le_bytes());

    let mss = [1208, 1368, 1388, 1428, 1448][rng.below(5) as usize];
    let rtt = rng.range(9_000, 160_000);
    let rttvar = rtt / 4 + rng.below(rtt / 8 + 1);
    let cwnd = rng.range(10, 400);
    let slow_start = rng.below(3) == 0;
    // The answer: a window of `cwnd` segments of `mss` bytes, once per smoothed round trip.
    let ceiling = cwnd * mss * 1_000_000 / rtt;

    b[off::STATE] = 1; // TCP_ESTABLISHED
    b[off::OPTIONS] = 7; // timestamps, SACK, window scaling
    b[off::WSCALE] = 0x77; // send and receive window scale 7
    put32(&mut b, off::RTO, (rtt + 4 * rttvar).max(200_000));
    put32(&mut b, off::ATO, 40_000);
    put32(&mut b, off::SND_MSS, mss);
    put32(&mut b, off::RCV_MSS, [536, mss][rng.below(2) as usize]);
    put32(&mut b, off::UNACKED, rng.below(cwnd));
    put32(&mut b, off::LAST_DATA_SENT, rng.below(2_000));
    put32(&mut b, off::LAST_DATA_RECV, rng.below(2_000));
    put32(&mut b, off::LAST_ACK_RECV, rng.below(2_000));
    put32(&mut b, off::PMTU, 1500);
    put32(&mut b, off::RCV_SSTHRESH, 64_076);
    put32(&mut b, off::RTT, rtt);
    put32(&mut b, off::RTTVAR, rttvar);
    put32(&mut b, off::SND_SSTHRESH, if slow_start { 0x7fff_ffff } else { rng.range(cwnd / 2, cwnd) });
    put32(&mut b, off::SND_CWND, cwnd);
    put32(&mut b, off::ADVMSS, mss);
    put32(&mut b, off::REORDERING, 3);
    put32(&mut b, off::RCV_RTT, rng.range(rtt / 2, rtt * 2));
    put32(&mut b, off::RCV_SPACE, 14_480 + mss * rng.below(40));
    put32(&mut b, off::TOTAL_RETRANS, rng.below(4));
    // Linux paces at 200 % of cwnd/RTT in slow start and 120 % after it.
    put64(&mut b, off::PACING_RATE, if slow_start { ceiling * 2 } else { ceiling * 12 / 10 });
    put64(&mut b, off::MAX_PACING_RATE, u64::MAX);
    let acked = rng.range(1 << 20, 1 << 32);
    put64(&mut b, off::BYTES_ACKED, acked);
    put64(&mut b, off::BYTES_RECEIVED, rng.range(2_000, 200_000));
    let segs_out = acked / mss + rng.below(64);
    put32(&mut b, off::SEGS_OUT, segs_out);
    put32(&mut b, off::SEGS_IN, segs_out / 2 + rng.below(100));
    put32(&mut b, off::MIN_RTT, rtt * rng.range(60, 95) / 100);
    put32(&mut b, off::DATA_SEGS_IN, rng.range(2, 40));
    put32(&mut b, off::DATA_SEGS_OUT, segs_out - rng.below(8));
    put64(&mut b, off::DELIVERY_RATE, ceiling * rng.range(30, 90) / 100);
    put64(&mut b, off::BUSY_TIME, rng.range(10_000, 5_000_000));
    put32(&mut b, off::DELIVERED, segs_out - rng.below(4));
    let retrans = rng.below(4) * mss;
    put64(&mut b, off::BYTES_SENT, acked + retrans);
    put64(&mut b, off::BYTES_RETRANS, retrans);
    put32(&mut b, off::SND_WND, rng.range(64, 3_072) << 10);
    put32(&mut b, off::RCV_WND, 64_256);
    (CtfPuzzle::Betelgeuse { tcp_info: hex(b) }, ceiling.to_string())
}

// ── 4 · Meissa, design: Fitts's law from a usability test ────────────────────────────────────

const TRIALS: usize = 40;

fn meissa(seed: impl Fn(u32) -> [u8; 32]) -> (CtfPuzzle, String) {
    let mut attempt = 0;
    loop {
        let mut rng = Rng::new(seed(attempt));
        let (a, b) = (rng.range(80, 250) as f64, rng.range(90, 220) as f64);
        let trials: Vec<[u32; 3]> = (0..TRIALS)
            .map(|_| {
                let (d, w) = (rng.range(48, 1024) as u32, rng.range(8, 128) as u32);
                let mt = (a + b * index_of_difficulty(d, w) + 30.0 * rng.gauss()).round().max(60.0);
                [d, w, mt as u32]
            })
            .collect();
        let slope = slope(&trials);
        let throughput = 1000.0 / slope;
        // Stay clear of a rounding edge, so last-bit differences between languages' log2 cannot
        // decide the answer.
        let tenths = throughput * 10.0;
        if (slope > 0.0 && (tenths - tenths.floor() - 0.5).abs() > 0.02) || attempt == 63 {
            return (CtfPuzzle::Meissa { trials }, format!("{throughput:.1}"));
        }
        attempt += 1;
    }
}

/// Shannon's formulation, in bits.
fn index_of_difficulty(d: u32, w: u32) -> f64 {
    (f64::from(d) / f64::from(w) + 1.0).log2()
}

/// Ordinary least squares slope of movement time on the index of difficulty (ms per bit).
fn slope(trials: &[[u32; 3]]) -> f64 {
    let n = trials.len() as f64;
    let xs: Vec<f64> = trials.iter().map(|t| index_of_difficulty(t[0], t[1])).collect();
    let ys: Vec<f64> = trials.iter().map(|t| f64::from(t[2])).collect();
    let (mx, my) = (xs.iter().sum::<f64>() / n, ys.iter().sum::<f64>() / n);
    let sxy: f64 = xs.iter().zip(&ys).map(|(x, y)| (x - mx) * (y - my)).sum();
    let sxx: f64 = xs.iter().map(|x| (x - mx) * (x - mx)).sum();
    sxy / sxx
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seeds(n: u8) -> impl Iterator<Item = impl Fn(u32) -> [u8; 32]> {
        (0..n).map(|i| move |attempt: u32| {
            let mut s = [i; 32];
            s[..4].copy_from_slice(&attempt.to_le_bytes());
            s
        })
    }

    fn unhex(s: &str) -> Vec<u8> {
        (0..s.len()).step_by(2).map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap()).collect()
    }

    #[test]
    fn dijkstra_on_a_known_grid() {
        // A small published example (cheapest total 40), and a grid where only a detour wins.
        let rows = "1163751742 1381373672 2136511328 3694931569 7463417111 1319128137 1359912421 3125421639 1293138521 2311944581";
        let grid: Vec<u8> = rows.split(' ').flat_map(|r| r.bytes().map(|b| b - b'0')).collect();
        assert_eq!(cheapest(&grid, 10), 40);
        // Down the left edge, up through the middle, down the right edge: twelve cells of cost 1.
        #[rustfmt::skip]
        let detour = [
            1, 9, 9, 9, 9,
            1, 9, 1, 1, 1,
            1, 9, 1, 9, 1,
            1, 1, 1, 9, 1,
            9, 9, 9, 9, 1,
        ];
        assert_eq!(cheapest(&detour, 5), 12);
        assert_eq!(sweep(&detour, 5), 16, "right/down only must cross a 9");
    }

    #[test]
    fn rigel_grids_punish_the_shortcut() {
        let mut fooled = 0;
        for seed in seeds(12) {
            let (CtfPuzzle::Rigel { grid }, answer) = rigel(seed) else { panic!() };
            let rows: Vec<&str> = grid.split('\n').collect();
            assert_eq!(rows.len(), SKY);
            assert!(rows.iter().all(|r| r.len() == SKY && r.bytes().all(|b| (b'1'..=b'9').contains(&b))));
            let cells: Vec<u8> = rows.iter().flat_map(|r| r.bytes().map(|b| b - b'0')).collect();
            assert_eq!(answer, cheapest(&cells, SKY).to_string());
            fooled += usize::from(sweep(&cells, SKY) != cheapest(&cells, SKY));
        }
        assert_eq!(fooled, 12, "every grid should defeat a right/down sweep");
    }

    #[test]
    fn saiph_decrypts_with_the_crib() {
        for seed in seeds(8) {
            let (CtfPuzzle::Saiph { a, b }, word) = saiph(&mut Rng::new(seed(0))) else { panic!() };
            let (a, b) = (unhex(&a), unhex(&b));
            assert_eq!(a.len(), CTF_CRIB.len());
            let plain: Vec<u8> = a.iter().zip(&b).zip(CTF_CRIB.bytes()).map(|((x, y), m)| x ^ y ^ m).collect();
            let plain = String::from_utf8(plain).unwrap();
            assert_eq!(plain, format!("Saiph holds. Second flag: {word}"));
            assert!(accepts(2, &word, &plain), "the whole line counts");
            assert!(accepts(2, &word, &word.to_uppercase()));
        }
    }

    #[test]
    fn betelgeuse_matches_glibc_tcp_info() {
        #[cfg(all(target_os = "linux", target_env = "gnu", target_arch = "x86_64"))]
        {
            use std::mem::{offset_of, size_of};
            assert_eq!(size_of::<libc::tcp_info>(), TCP_INFO_LEN);
            assert_eq!(offset_of!(libc::tcp_info, tcpi_snd_mss), off::SND_MSS);
            assert_eq!(offset_of!(libc::tcp_info, tcpi_rtt), off::RTT);
            assert_eq!(offset_of!(libc::tcp_info, tcpi_rttvar), off::RTTVAR);
            assert_eq!(offset_of!(libc::tcp_info, tcpi_snd_ssthresh), off::SND_SSTHRESH);
            assert_eq!(offset_of!(libc::tcp_info, tcpi_snd_cwnd), off::SND_CWND);
            assert_eq!(offset_of!(libc::tcp_info, tcpi_pacing_rate), off::PACING_RATE);
            assert_eq!(offset_of!(libc::tcp_info, tcpi_min_rtt), off::MIN_RTT);
            assert_eq!(offset_of!(libc::tcp_info, tcpi_delivery_rate), off::DELIVERY_RATE);
            assert_eq!(offset_of!(libc::tcp_info, tcpi_busy_time), off::BUSY_TIME);
            assert_eq!(offset_of!(libc::tcp_info, tcpi_delivered), off::DELIVERED);
            assert_eq!(offset_of!(libc::tcp_info, tcpi_bytes_sent), off::BYTES_SENT);
            assert_eq!(offset_of!(libc::tcp_info, tcpi_bytes_retrans), off::BYTES_RETRANS);
            assert_eq!(offset_of!(libc::tcp_info, tcpi_snd_wnd), off::SND_WND);
            assert_eq!(offset_of!(libc::tcp_info, tcpi_rcv_wnd), off::RCV_WND);
        }
        for seed in seeds(8) {
            let (CtfPuzzle::Betelgeuse { tcp_info }, answer) = betelgeuse(&mut Rng::new(seed(0))) else { panic!() };
            let b = unhex(&tcp_info);
            assert_eq!(b.len(), TCP_INFO_LEN);
            let u32_at = |at: usize| u64::from(u32::from_le_bytes(b[at..at + 4].try_into().unwrap()));
            let window = u32_at(off::SND_CWND) * u32_at(off::SND_MSS);
            assert_eq!(answer, (window * 1_000_000 / u32_at(off::RTT)).to_string());
            assert!(u32_at(off::RTT) >= 9_000 && u32_at(off::MIN_RTT) < u32_at(off::RTT));
        }
    }

    #[test]
    fn meissa_answers_are_one_decimal_and_clear_of_edges() {
        for seed in seeds(16) {
            let (CtfPuzzle::Meissa { trials }, answer) = meissa(seed) else { panic!() };
            assert_eq!(trials.len(), TRIALS);
            let tp: f64 = answer.parse().unwrap();
            assert!((3.0..=12.0).contains(&tp), "throughput {tp} bits/s is outside what the generator aims for");
            assert_eq!(answer, format!("{:.1}", 1000.0 / slope(&trials)));
            assert!(accepts(4, &answer, &format!("{answer} bits/s")));
        }
    }

    #[test]
    fn answers_are_compared_by_content() {
        assert!(accepts(1, "4821", "4821"));
        assert!(accepts(1, "4821", " mintaka{4,821} "));
        assert!(accepts(3, "1234567", "1_234_567"));
        assert!(!accepts(1, "4821", "4822"));
        assert!(!accepts(1, "4821", "48.21"));
        assert!(!accepts(1, "4821", ""));
        assert!(accepts(4, "7.3", "7.30"));
        assert!(accepts(4, "7.3", "7.34"));
        assert!(!accepts(4, "7.3", "7.4"));
        assert!(!accepts(4, "7.3", "NaN"));
        assert!(!accepts(2, "k3x9q2m7v1zd", "k3x9q2m7v1z"));
        assert!(!accepts(9, "1", "1"));
    }
}
