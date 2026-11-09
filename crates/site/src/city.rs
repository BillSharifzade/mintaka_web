//! The lab's opening experiment needs Dushanbe's streets in a few dozen kilobytes, not the routing
//! instrument's 1.1 MB graph (which carries the Contraction Hierarchy). This derives a compact copy
//! at build time from `assets/data/dushanbe.bin`: every intersection and every street, the street
//! shapes simplified to 10 m and positions rounded to 4 m (both under half a pixel with the whole
//! city on screen), one-way streets kept.
//!
//! The streets are written as a walk: breadth-first from one intersection, each street is stored
//! once, by the end reached first. When a street leads somewhere new, that intersection's id is
//! simply the next one, and its position a short hop from the street's start — so most streets
//! cost a few bytes and no reference at all. Numbers of one kind are kept together (columns), which
//! compresses better.
//!
//! Format `MKCT` v2, read by web/city/graph.ts (all integers LEB128; "z" = zigzag-signed):
//!   magic "MKCT", u8 version, N, S, width, height (units), unit (metres), then the byte length of
//!   each of the 7 columns, then the columns:
//!   0 per intersection in id order: how many streets it owns
//!   1 per walk root (an intersection no street led to): z dx, z dy from the previous root
//!   2 per street: k << 3 | new << 2 | dir   (dir 0 both ways, 1 owner → other, 2 other → owner;
//!                                            k interior points)
//!   3 per street to a new intersection: z dx, z dy from the owner
//!   4 per street to a known intersection: its id − the owner's id
//!   5, 6 per interior point: z dx, then z dy (separate columns), each from the previous point

use routing::Graph;
use std::collections::VecDeque;

pub const MAGIC: &[u8; 4] = b"MKCT";
pub const VERSION: u8 = 2;
/// How far a simplified street may stray from its surveyed shape, metres.
const TOLERANCE: f64 = 10.0;
/// Positions are stored in units of this many metres.
const UNIT: f32 = 4.0;

pub struct Stats {
    pub nodes: usize,
    pub streets: usize,
    pub points: usize,
    pub bytes: usize,
}

struct Street {
    a: usize,
    b: usize,
    /// 0 both ways, 1 a → b, 2 b → a.
    dir: u64,
    interior: Vec<[i64; 2]>,
}

pub fn encode(g: &Graph) -> (Vec<u8>, Stats) {
    let n = g.nodes();
    let unit = UNIT;
    let (min_x, min_y) = g.xy.iter().fold((f32::MAX, f32::MAX), |(a, b), p| (a.min(p[0]), b.min(p[1])));
    let (max_x, max_y) = g.xy.iter().fold((f32::MIN, f32::MIN), |(a, b), p| (a.max(p[0]), b.max(p[1])));
    let at = |p: [f32; 2]| [((p[0] - min_x) / unit).round() as i64, ((p[1] - min_y) / unit).round() as i64];

    // Streets (one per polyline): ends, the directions it can be driven, simplified shape.
    let nseg = g.seg_off.len() - 1;
    let mut ends: Vec<Option<(usize, usize)>> = vec![None; nseg];
    let mut fwd = vec![false; nseg];
    let mut back = vec![false; nseg];
    for u in 0..n {
        for a in g.off[u] as usize..g.off[u + 1] as usize {
            let s = (g.seg[a] >> 1) as usize;
            let rev = g.seg[a] & routing::graph::REVERSED != 0;
            let v = g.to[a] as usize;
            ends[s] = Some(if rev { (v, u) } else { (u, v) });
            if rev { back[s] = true } else { fwd[s] = true }
        }
    }
    let mut streets: Vec<Street> = Vec::new();
    let mut points = 0;
    let mut touching: Vec<Vec<usize>> = vec![Vec::new(); n];
    for s in 0..nseg {
        let Some((a, b)) = ends[s] else { continue };
        let pts: Vec<[i64; 2]> = (g.seg_off[s] as usize..g.seg_off[s + 1] as usize).map(|i| at(g.point(i))).collect();
        let kept = simplify(&pts, TOLERANCE / unit as f64);
        let interior = kept[1..kept.len().saturating_sub(1)].to_vec();
        points += interior.len();
        let dir = match (fwd[s], back[s]) {
            (true, true) => 0,
            (true, false) => 1,
            _ => 2,
        };
        touching[a].push(streets.len());
        if b != a {
            touching[b].push(streets.len());
        }
        streets.push(Street { a, b, dir, interior });
    }

    // The walk: breadth-first, roots taken north to south so each is near the last. Visiting order
    // is id order, so a decoder reading intersections in id order follows the same walk.
    let mut id = vec![usize::MAX; n];
    let mut by_id: Vec<usize> = Vec::with_capacity(n);
    let mut owned: Vec<Vec<usize>> = Vec::with_capacity(n);
    let mut emitted = vec![false; streets.len()];
    let mut seeds: Vec<usize> = (0..n).collect();
    seeds.sort_by_key(|&i| (at(g.xy[i])[1], at(g.xy[i])[0]));
    let mut seed_at = 0;
    let mut queue = VecDeque::new();
    while by_id.len() < n {
        while id[seeds[seed_at]] != usize::MAX {
            seed_at += 1;
        }
        let root = seeds[seed_at];
        id[root] = by_id.len();
        by_id.push(root);
        queue.push_back(root);
        while let Some(u) = queue.pop_front() {
            let mut mine = Vec::new();
            for &s in &touching[u] {
                if emitted[s] {
                    continue;
                }
                emitted[s] = true;
                let v = if streets[s].a == u { streets[s].b } else { streets[s].a };
                if id[v] == usize::MAX {
                    id[v] = by_id.len();
                    by_id.push(v);
                    queue.push_back(v);
                }
                mine.push(s);
            }
            owned.push(mine);
        }
    }

    let mut cols: [Vec<u8>; 7] = Default::default();
    let mut known = vec![false; n];
    let mut prev_root = [0i64; 2];
    for (k, &u) in by_id.iter().enumerate() {
        if !known[u] {
            let p = at(g.xy[u]);
            zig(&mut cols[1], p[0] - prev_root[0]);
            zig(&mut cols[1], p[1] - prev_root[1]);
            prev_root = p;
            known[u] = true;
        }
        leb(&mut cols[0], owned[k].len() as u64);
        for &s in &owned[k] {
            let st = &streets[s];
            let forward = st.a == u;
            let v = if forward { st.b } else { st.a };
            // Direction and shape, as seen from the owner.
            let dir = match (st.dir, forward) {
                (0, _) => 0,
                (d, true) => d,
                (1, false) => 2,
                _ => 1,
            };
            let fresh = !known[v];
            leb(&mut cols[2], (st.interior.len() as u64) << 3 | (fresh as u64) << 2 | dir);
            let pu = at(g.xy[u]);
            if fresh {
                let pv = at(g.xy[v]);
                zig(&mut cols[3], pv[0] - pu[0]);
                zig(&mut cols[3], pv[1] - pu[1]);
                known[v] = true;
            } else {
                leb(&mut cols[4], (id[v] - k) as u64);
            }
            let mut last = pu;
            let shape: Box<dyn Iterator<Item = &[i64; 2]>> = if forward { Box::new(st.interior.iter()) } else { Box::new(st.interior.iter().rev()) };
            for p in shape {
                zig(&mut cols[5], p[0] - last[0]);
                zig(&mut cols[6], p[1] - last[1]);
                last = *p;
            }
        }
    }

    let mut b = Vec::with_capacity(48 * 1024);
    b.extend_from_slice(MAGIC);
    b.push(VERSION);
    let header = [n as u64, streets.len() as u64, ((max_x - min_x) / unit).round() as u64, ((max_y - min_y) / unit).round() as u64, unit as u64];
    for v in header.into_iter().chain(cols.iter().map(|c| c.len() as u64)) {
        leb(&mut b, v);
    }
    for c in &cols {
        b.extend_from_slice(c);
    }
    let stats = Stats { nodes: n, streets: streets.len(), points, bytes: b.len() };
    (b, stats)
}

fn leb(b: &mut Vec<u8>, mut v: u64) {
    loop {
        let byte = (v & 0x7f) as u8;
        v >>= 7;
        if v == 0 {
            b.push(byte);
            return;
        }
        b.push(byte | 0x80);
    }
}

fn zig(b: &mut Vec<u8>, v: i64) {
    leb(b, ((v << 1) ^ (v >> 63)) as u64);
}

/// Douglas–Peucker: the fewest points that keep the line within `tol` of the original.
fn simplify(pts: &[[i64; 2]], tol: f64) -> Vec<[i64; 2]> {
    if pts.len() <= 2 {
        return pts.to_vec();
    }
    let mut keep = vec![false; pts.len()];
    keep[0] = true;
    keep[pts.len() - 1] = true;
    let mut stack = vec![(0usize, pts.len() - 1)];
    while let Some((a, z)) = stack.pop() {
        let (ax, ay) = (pts[a][0] as f64, pts[a][1] as f64);
        let (zx, zy) = (pts[z][0] as f64, pts[z][1] as f64);
        let (dx, dy) = (zx - ax, zy - ay);
        let len = dx.hypot(dy);
        let (mut far, mut best) = (0usize, 0.0f64);
        for (i, p) in pts.iter().enumerate().take(z).skip(a + 1) {
            let (px, py) = (p[0] as f64 - ax, p[1] as f64 - ay);
            let d = if len == 0.0 { px.hypot(py) } else { (px * dy - py * dx).abs() / len };
            if d > best {
                best = d;
                far = i;
            }
        }
        if best > tol {
            keep[far] = true;
            stack.push((a, far));
            stack.push((far, z));
        }
    }
    pts.iter().zip(keep).filter_map(|(p, k)| k.then_some(*p)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn varints_round_trip_by_hand() {
        let mut b = Vec::new();
        leb(&mut b, 300);
        zig(&mut b, -3);
        assert_eq!(b, [0xac, 0x02, 0x05]);
    }

    #[test]
    fn simplification_keeps_corners_and_drops_straight_points() {
        let line = [[0, 0], [5, 0], [10, 0], [10, 10]];
        assert_eq!(simplify(&line, 2.0), vec![[0, 0], [10, 0], [10, 10]]);
        let wiggle = [[0, 0], [5, 1], [10, 0]];
        assert_eq!(simplify(&wiggle, 2.0), vec![[0, 0], [10, 0]]);
    }

    #[test]
    fn the_city_fits_in_a_few_dozen_kilobytes() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets/data/dushanbe.bin");
        let Ok(bytes) = std::fs::read(path) else { return };
        let g = Graph::from_bytes(&bytes).unwrap();
        let (out, st) = encode(&g);
        assert_eq!(st.nodes, g.nodes());
        assert!(st.streets > 10_000, "{}", st.streets);
        assert_eq!(&out[..4], MAGIC);
        assert!(out.len() < 60_000, "{} bytes", out.len());
        let mut br = Vec::new();
        {
            let mut w = brotli::CompressorWriter::new(&mut br, 4096, 11, 22);
            std::io::Write::write_all(&mut w, &out).unwrap();
        }
        eprintln!("city: {} nodes, {} streets, {} points, {} bytes, {} brotli", st.nodes, st.streets, st.points, st.bytes, br.len());
    }
}
