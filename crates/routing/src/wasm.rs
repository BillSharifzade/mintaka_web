//! The JavaScript surface. Coordinates are metres in the graph's projection; lines are flat
//! `[x1, y1, x2, y2, …]` arrays the island draws on a canvas.

use crate::graph::Graph;
use crate::search::{Algo, Search, Step, nearest};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct Router {
    g: Graph,
    s: Search,
}

fn algo(code: u8) -> Algo {
    match code {
        1 => Algo::AStar,
        2 => Algo::Ch,
        _ => Algo::Dijkstra,
    }
}

fn push_polyline(out: &mut Vec<f32>, g: &Graph, arc: usize) {
    let mut prev: Option<[f32; 2]> = None;
    for p in g.arc_points(arc) {
        if let Some(q) = prev {
            out.extend_from_slice(&[q[0], q[1], p[0], p[1]]);
        }
        prev = Some(p);
    }
}

#[wasm_bindgen]
impl Router {
    #[wasm_bindgen(constructor)]
    pub fn new(bytes: &[u8]) -> Result<Router, JsError> {
        let g = Graph::from_bytes(bytes).map_err(|e| JsError::new(&e))?;
        let s = Search::new(&g);
        Ok(Router { g, s })
    }

    pub fn nodes(&self) -> u32 {
        self.g.nodes() as u32
    }

    pub fn arcs(&self) -> u32 {
        self.g.to.len() as u32
    }

    /// Hierarchy arcs that are shortcuts (have a contracted middle node).
    pub fn shortcuts(&self) -> u32 {
        self.g.up_mid.iter().chain(&self.g.dn_mid).filter(|&&m| m != crate::NONE).count() as u32
    }

    /// `[min x, min y, max x, max y]` of every road point, metres.
    pub fn bounds(&self) -> Vec<f32> {
        let (mut b0, mut b1, mut b2, mut b3) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
        for i in 0..self.g.pts.len() {
            let p = self.g.point(i);
            b0 = b0.min(p[0]);
            b1 = b1.min(p[1]);
            b2 = b2.max(p[0]);
            b3 = b3.max(p[1]);
        }
        vec![b0, b1, b2, b3]
    }

    /// Every road segment as lines, for the base map.
    pub fn roads(&self) -> Vec<f32> {
        let g = &self.g;
        let mut out = Vec::with_capacity(g.pts.len() * 4);
        for s in 0..g.seg_off.len() - 1 {
            for i in g.seg_off[s] as usize + 1..g.seg_off[s + 1] as usize {
                let (a, b) = (g.point(i - 1), g.point(i));
                out.extend_from_slice(&[a[0], a[1], b[0], b[1]]);
            }
        }
        out
    }

    pub fn nearest(&self, x: f32, y: f32) -> u32 {
        nearest(&self.g, x, y)
    }

    /// Latitude/longitude → metres in the graph's projection.
    pub fn project(&self, lat: f64, lon: f64) -> Vec<f32> {
        self.g.project(lat, lon).to_vec()
    }

    pub fn node_xy(&self, v: u32) -> Vec<f32> {
        let p = self.g.xy[v as usize];
        vec![p[0], p[1]]
    }

    /// Run a query (0 Dijkstra, 1 A*, 2 CH). Returns the number of nodes settled.
    pub fn run(&mut self, algorithm: u8, s: u32, t: u32, record: bool) -> u32 {
        self.s.run(&self.g, algo(algorithm), s, t, record);
        self.s.settled
    }

    /// Route length of the last query in decimetres (4294967295 if unreachable).
    pub fn distance(&self) -> u32 {
        self.s.dist
    }

    /// What the last recorded query looked at, in settle order, as lines; and where each
    /// settled node's lines end (`marks[i]` = number of floats after node i).
    pub fn explored(&self) -> Vec<f32> {
        let mut out = vec![];
        for step in &self.s.steps {
            match *step {
                Step::Arc(a) => push_polyline(&mut out, &self.g, a as usize),
                Step::Jump(u, v) => {
                    let (p, q) = (self.g.xy[u as usize], self.g.xy[v as usize]);
                    out.extend_from_slice(&[p[0], p[1], q[0], q[1]]);
                }
            }
        }
        out
    }

    pub fn explored_marks(&self) -> Vec<u32> {
        let mut marks = Vec::with_capacity(self.s.steps.len());
        let mut n = 0u32;
        for step in &self.s.steps {
            n += match *step {
                Step::Arc(a) => {
                    let s = (self.g.seg[a as usize] >> 1) as usize;
                    (self.g.seg_off[s + 1] - self.g.seg_off[s] - 1) * 4
                }
                Step::Jump(..) => 4,
            };
            marks.push(n);
        }
        marks
    }

    /// The last route as lines.
    pub fn path(&self) -> Vec<f32> {
        let mut out = vec![];
        for &a in &self.s.path {
            push_polyline(&mut out, &self.g, a as usize);
        }
        out
    }
}
