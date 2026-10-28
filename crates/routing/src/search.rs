//! Dijkstra, A* and the Contraction Hierarchies query, sharing one reusable search state.
//! State arrays are stamped with an epoch, so a new query costs nothing to reset.

use crate::graph::{Graph, NONE};
use std::cmp::Reverse;
use std::collections::BinaryHeap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Algo {
    Dijkstra,
    AStar,
    Ch,
}

/// What one settled node contributes to the picture of the search.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    /// A base arc (Dijkstra, A*): draw the road segment.
    Arc(u32),
    /// A hierarchy arc from `from` to `to` (CH): draw the jump as a straight line.
    Jump(u32, u32),
}

#[derive(Default)]
struct Side {
    dist: Vec<u32>,
    parent: Vec<u32>, // the arc used to reach a node (base arc, or index into up/dn)
    from: Vec<u32>,   // the node it came from
    stamp: Vec<u32>,
    heap: BinaryHeap<Reverse<(u32, u32)>>,
}

impl Side {
    fn new(n: usize) -> Side {
        Side { dist: vec![0; n], parent: vec![NONE; n], from: vec![NONE; n], stamp: vec![0; n], heap: BinaryHeap::new() }
    }
    #[inline]
    fn get(&self, v: usize, epoch: u32) -> u32 {
        if self.stamp[v] == epoch { self.dist[v] } else { u32::MAX }
    }
    #[inline]
    fn relax(&mut self, v: usize, d: u32, key: u32, arc: u32, from: u32, epoch: u32) {
        if d < self.get(v, epoch) {
            self.stamp[v] = epoch;
            self.dist[v] = d;
            self.parent[v] = arc;
            self.from[v] = from;
            self.heap.push(Reverse((key, v as u32)));
        }
    }
}

pub struct Search {
    fwd: Side,
    bwd: Side,
    epoch: u32,
    /// Settled-node trace of the last query, when recording.
    pub steps: Vec<Step>,
    /// Base arcs of the last route, source to target.
    pub path: Vec<u32>,
    /// Length of the last route in decimetres (`NONE` if unreachable).
    pub dist: u32,
    /// Nodes settled by the last query (both directions for CH).
    pub settled: u32,
}

impl Search {
    pub fn new(g: &Graph) -> Search {
        Search { fwd: Side::new(g.nodes()), bwd: Side::new(g.nodes()), epoch: 0, steps: vec![], path: vec![], dist: NONE, settled: 0 }
    }

    fn begin(&mut self) {
        self.epoch = self.epoch.wrapping_add(1);
        if self.epoch == 0 {
            self.fwd.stamp.fill(0);
            self.bwd.stamp.fill(0);
            self.epoch = 1;
        }
        self.fwd.heap.clear();
        self.bwd.heap.clear();
        self.steps.clear();
        self.path.clear();
        self.dist = NONE;
        self.settled = 0;
    }

    /// Run one query; with `record`, keep the settle order in `steps`.
    pub fn run(&mut self, g: &Graph, algo: Algo, s: u32, t: u32, record: bool) {
        self.begin();
        match algo {
            Algo::Dijkstra => self.uni(g, s, t, false, record),
            Algo::AStar => self.uni(g, s, t, true, record),
            Algo::Ch => self.ch(g, s, t, record),
        }
    }

    fn uni(&mut self, g: &Graph, s: u32, t: u32, astar: bool, record: bool) {
        let e = self.epoch;
        let target = g.xy[t as usize];
        // Straight-line distance in decimetres, rounded down: never more than the true length.
        let h = |v: usize| -> u32 {
            if !astar {
                return 0;
            }
            let p = g.xy[v];
            let (dx, dy) = (p[0] - target[0], p[1] - target[1]);
            ((dx * dx + dy * dy).sqrt() * 10.0) as u32
        };
        self.fwd.relax(s as usize, 0, h(s as usize), NONE, NONE, e);
        while let Some(Reverse((key, v))) = self.fwd.heap.pop() {
            let v = v as usize;
            let d = self.fwd.dist[v];
            if key > d.saturating_add(h(v)) {
                continue; // stale entry
            }
            self.settled += 1;
            if record && self.fwd.parent[v] != NONE {
                self.steps.push(Step::Arc(self.fwd.parent[v]));
            }
            if v == t as usize {
                self.dist = d;
                let mut x = v;
                while self.fwd.parent[x] != NONE {
                    self.path.push(self.fwd.parent[x]);
                    x = self.fwd.from[x] as usize;
                }
                self.path.reverse();
                return;
            }
            for a in g.off[v] as usize..g.off[v + 1] as usize {
                let u = g.to[a] as usize;
                let nd = d + g.w[a];
                if nd < self.fwd.get(u, e) {
                    self.fwd.relax(u, nd, nd + h(u), a as u32, v as u32, e);
                }
            }
        }
    }

    /// Bidirectional query on the hierarchy: forward upward from `s`, backward upward from `t`.
    fn ch(&mut self, g: &Graph, s: u32, t: u32, record: bool) {
        let e = self.epoch;
        self.fwd.relax(s as usize, 0, 0, NONE, NONE, e);
        self.bwd.relax(t as usize, 0, 0, NONE, NONE, e);
        let mut best = u32::MAX;
        let mut meet = NONE;
        loop {
            let fmin = self.fwd.heap.peek().map_or(u32::MAX, |r| r.0.0);
            let bmin = self.bwd.heap.peek().map_or(u32::MAX, |r| r.0.0);
            if fmin.min(bmin) >= best {
                break;
            }
            let forward = fmin <= bmin;
            let (side, other) = if forward { (&mut self.fwd, &self.bwd) } else { (&mut self.bwd, &self.fwd) };
            let Some(Reverse((d, v))) = side.heap.pop() else { break };
            let v = v as usize;
            if d > side.dist[v] {
                continue;
            }
            self.settled += 1;
            if record && side.parent[v] != NONE {
                self.steps.push(Step::Jump(side.from[v], v as u32));
            }
            let od = other.get(v, e);
            if od != u32::MAX && d + od < best {
                best = d + od;
                meet = v as u32;
            }
            if forward {
                for a in g.up_off[v] as usize..g.up_off[v + 1] as usize {
                    side.relax(g.up_to[a] as usize, d + g.up_w[a], d + g.up_w[a], a as u32, v as u32, e);
                }
            } else {
                for a in g.dn_off[v] as usize..g.dn_off[v + 1] as usize {
                    side.relax(g.dn_from[a] as usize, d + g.dn_w[a], d + g.dn_w[a], a as u32, v as u32, e);
                }
            }
        }
        if meet == NONE {
            return;
        }
        self.dist = best;
        // Hierarchy arcs from s up to the meeting node, then from it down to t.
        let mut ups = vec![];
        let mut x = meet as usize;
        while self.fwd.parent[x] != NONE {
            let a = self.fwd.parent[x] as usize;
            let from = self.fwd.from[x];
            ups.push((from, x as u32, g.up_w[a], g.up_mid[a]));
            x = from as usize;
        }
        ups.reverse();
        let mut x = meet as usize;
        while self.bwd.parent[x] != NONE {
            let a = self.bwd.parent[x] as usize;
            let next = self.bwd.from[x];
            ups.push((x as u32, next, g.dn_w[a], g.dn_mid[a]));
            x = next as usize;
        }
        for (u, v, w, mid) in ups {
            unpack(g, u, v, w, mid, &mut self.path);
        }
    }
}

/// Expand the hierarchy arc `u → v` (weight `w`, contracted node `mid`) into base arcs.
pub fn unpack(g: &Graph, u: u32, v: u32, w: u32, mid: u32, out: &mut Vec<u32>) {
    let mut stack = vec![(u, v, w, mid)];
    while let Some((u, v, w, mid)) = stack.pop() {
        if mid == NONE {
            let arc = (g.off[u as usize]..g.off[u as usize + 1]).find(|&a| g.to[a as usize] == v && g.w[a as usize] == w);
            if let Some(a) = arc {
                out.push(a);
            }
            continue;
        }
        // u → mid is stored at mid as an arc from a higher-ranked node; mid → v as an upward arc.
        let m = mid as usize;
        let left = (g.dn_off[m]..g.dn_off[m + 1]).filter(|&a| g.dn_from[a as usize] == u);
        let mut pick = None;
        'outer: for a in left {
            let w1 = g.dn_w[a as usize];
            for b in g.up_off[m]..g.up_off[m + 1] {
                if g.up_to[b as usize] == v && w1 + g.up_w[b as usize] == w {
                    pick = Some((a as usize, b as usize));
                    break 'outer;
                }
            }
        }
        if let Some((a, b)) = pick {
            // Push the right half first so the left half is expanded first (path order).
            stack.push((mid, v, g.up_w[b], g.up_mid[b]));
            stack.push((u, mid, g.dn_w[a], g.dn_mid[a]));
        }
    }
}

/// The node nearest to a point (metres). Linear scan: a few thousand nodes, called on a tap.
pub fn nearest(g: &Graph, x: f32, y: f32) -> u32 {
    let mut best = (f32::MAX, 0u32);
    for (i, p) in g.xy.iter().enumerate() {
        let d = (p[0] - x).powi(2) + (p[1] - y).powi(2);
        if d < best.0 {
            best = (d, i as u32);
        }
    }
    best.1
}
