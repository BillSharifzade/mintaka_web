//! Correctness: on every graph, A* and Contraction Hierarchies must return exactly Dijkstra's
//! distance, and every route must be a connected chain of real arcs whose weights add up.

use crate::graph::{Graph, NONE};
use crate::search::{Algo, Search};

/// Deterministic xorshift, so failures reproduce.
pub(crate) struct Rng(u64);
impl Rng {
    pub(crate) fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    pub(crate) fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
}

fn check_route(g: &Graph, s: &Search, from: u32, to: u32) {
    if from == to {
        assert_eq!(s.dist, 0);
        return;
    }
    assert!(!s.path.is_empty(), "no route {from} → {to}");
    let first = s.path[0] as usize;
    assert!((g.off[from as usize] as usize..g.off[from as usize + 1] as usize).contains(&first), "route does not start at {from}");
    for w in s.path.windows(2) {
        let at = g.to[w[0] as usize] as usize;
        assert!((g.off[at] as usize..g.off[at + 1] as usize).contains(&(w[1] as usize)), "route breaks at node {at}");
    }
    assert_eq!(g.to[*s.path.last().unwrap() as usize], to, "route does not end at {to}");
    let total: u32 = s.path.iter().map(|&a| g.w[a as usize]).sum();
    assert_eq!(total, s.dist, "route weights do not add up to the distance");
}

pub(crate) fn compare(g: &Graph, pairs: usize, seed: u64) {
    let mut rng = Rng(seed);
    let mut s = Search::new(g);
    for _ in 0..pairs {
        let (a, b) = (rng.below(g.nodes()) as u32, rng.below(g.nodes()) as u32);
        s.run(g, Algo::Dijkstra, a, b, false);
        let d = s.dist;
        assert_ne!(d, NONE, "graph should be strongly connected");
        check_route(g, &s, a, b);
        s.run(g, Algo::AStar, a, b, false);
        assert_eq!(s.dist, d, "A* {a} → {b}");
        check_route(g, &s, a, b);
        s.run(g, Algo::Ch, a, b, true);
        assert_eq!(s.dist, d, "CH {a} → {b}");
        check_route(g, &s, a, b);
    }
}

#[cfg(feature = "prep")]
#[test]
fn random_grids_agree() {
    use crate::prep::{Segment, build};
    for seed in 1..=4u64 {
        let mut rng = Rng(seed * 7919);
        let side = 24;
        let mut xy = vec![];
        for y in 0..side {
            for x in 0..side {
                let j = |r: &mut Rng| (r.below(400) as f32 - 200.0) / 10.0;
                xy.push([x as f32 * 100.0 + j(&mut rng), y as f32 * 100.0 + j(&mut rng)]);
            }
        }
        let mut segs = vec![];
        for y in 0..side {
            for x in 0..side {
                let a = (y * side + x) as u32;
                for (dx, dy) in [(1, 0), (0, 1)] {
                    if x + dx >= side || y + dy >= side || rng.below(10) == 0 {
                        continue; // some roads are missing
                    }
                    let b = ((y + dy) * side + x + dx) as u32;
                    let (pa, pb) = (xy[a as usize], xy[b as usize]);
                    // a bend in the middle, so lengths differ from straight lines
                    let mid = [(pa[0] + pb[0]) / 2.0 + (rng.below(30) as f32 - 15.0), (pa[1] + pb[1]) / 2.0 + (rng.below(30) as f32 - 15.0)];
                    let oneway = rng.below(6) == 0;
                    segs.push(Segment { a, b, pts: vec![pa, mid, pb], fwd: true, bwd: !oneway });
                }
            }
        }
        let (g, _) = build(xy, segs, 38.5, 68.8).unwrap();
        let bytes = g.to_bytes();
        let g2 = Graph::from_bytes(&bytes).unwrap();
        assert_eq!(g, g2, "binary round trip");
        compare(&g2, 300, seed);
    }
}

/// The real thing: 1,000 random pairs across Dushanbe (spec S4 "done when").
#[test]
fn dushanbe_graph_agrees() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets/data/dushanbe.bin");
    let Ok(bytes) = std::fs::read(path) else {
        eprintln!("skipped: {path} not built yet (cargo xtask graph)");
        return;
    };
    let g = Graph::from_bytes(&bytes).unwrap();
    compare(&g, 1000, 42);
}
