//! Build time only (`--features prep`, run by `cargo xtask graph`): OpenStreetMap → road graph →
//! Contraction Hierarchy → bytes. Our own implementation, so every step is ours to explain.

use crate::graph::{Graph, NONE, REVERSED};
use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap};

pub struct Stats {
    pub osm_ways: usize,
    pub osm_nodes: usize,
    pub nodes: usize,
    pub arcs: usize,
    pub segments: usize,
    pub points: usize,
    pub shortcuts: usize,
}

/// One road segment between two intersections.
pub struct Segment {
    pub a: u32,
    pub b: u32,
    pub pts: Vec<[f32; 2]>,
    pub fwd: bool,
    pub bwd: bool,
}

/// Parse an Overpass JSON extract (ways with `highway`, plus their nodes) into a graph.
pub fn from_overpass(json: &str) -> Result<(Graph, Stats), String> {
    let doc: serde_json::Value = serde_json::from_str(json).map_err(|e| format!("Overpass JSON: {e}"))?;
    let els = doc["elements"].as_array().ok_or("no elements")?;
    let mut coords: HashMap<i64, (f64, f64)> = HashMap::new();
    let mut ways = vec![];
    for e in els {
        match e["type"].as_str() {
            Some("node") => {
                coords.insert(e["id"].as_i64().unwrap_or(0), (e["lat"].as_f64().unwrap_or(0.0), e["lon"].as_f64().unwrap_or(0.0)));
            }
            Some("way") => {
                let nodes: Vec<i64> = e["nodes"].as_array().map(|a| a.iter().filter_map(|n| n.as_i64()).collect()).unwrap_or_default();
                let tag = |k: &str| e["tags"][k].as_str().unwrap_or("").to_owned();
                let hw = tag("highway");
                let ow = tag("oneway");
                let (fwd, bwd) = match ow.as_str() {
                    "yes" | "true" | "1" => (true, false),
                    "-1" | "reverse" => (false, true),
                    "no" | "false" | "0" => (true, true),
                    _ if tag("junction") == "roundabout" || tag("junction") == "circular" || hw == "motorway" || hw == "motorway_link" => (true, false),
                    _ => (true, true),
                };
                if nodes.len() >= 2 {
                    ways.push((nodes, fwd, bwd));
                }
            }
            _ => {}
        }
    }
    let osm_ways = ways.len();
    let osm_nodes = coords.len();

    // Local projection around the centre of the extract.
    let (mut lat_lo, mut lat_hi, mut lon_lo, mut lon_hi) = (f64::MAX, f64::MIN, f64::MAX, f64::MIN);
    for &(la, lo) in coords.values() {
        lat_lo = lat_lo.min(la);
        lat_hi = lat_hi.max(la);
        lon_lo = lon_lo.min(lo);
        lon_hi = lon_hi.max(lo);
    }
    let (lat0, lon0) = ((lat_lo + lat_hi) / 2.0, (lon_lo + lon_hi) / 2.0);
    let (m_lat, m_lon) = crate::graph::metres_per_degree(lat0);
    let project = |id: i64| -> Option<[f32; 2]> {
        coords.get(&id).map(|&(la, lo)| [((lo - lon0) * m_lon) as f32, (-(la - lat0) * m_lat) as f32])
    };

    // Intersections: way ends, and nodes shared by several ways (or visited twice by one).
    let mut uses: HashMap<i64, u32> = HashMap::new();
    for (nodes, ..) in &ways {
        for n in nodes {
            *uses.entry(*n).or_default() += 1;
        }
    }
    let mut ids: HashMap<i64, u32> = HashMap::new();
    let mut xy: Vec<[f32; 2]> = vec![];
    let mut id_of = |osm: i64, xy: &mut Vec<[f32; 2]>| -> Option<u32> {
        if let Some(&i) = ids.get(&osm) {
            return Some(i);
        }
        let p = project(osm)?;
        let i = xy.len() as u32;
        xy.push(p);
        ids.insert(osm, i);
        Some(i)
    };
    let mut segs: Vec<Segment> = vec![];
    for (nodes, fwd, bwd) in &ways {
        let mut start = 0;
        for k in 1..nodes.len() {
            let last = k == nodes.len() - 1;
            if !last && uses.get(&nodes[k]).copied().unwrap_or(0) < 2 {
                continue;
            }
            let pts: Vec<[f32; 2]> = nodes[start..=k].iter().filter_map(|&n| project(n)).collect();
            if pts.len() >= 2
                && nodes[start] != nodes[k]
                && let (Some(a), Some(b)) = (id_of(nodes[start], &mut xy), id_of(nodes[k], &mut xy))
            {
                segs.push(Segment { a, b, pts, fwd: *fwd, bwd: *bwd });
            }
            start = k;
        }
    }
    let (g, shortcuts) = build(xy, segs, lat0, lon0)?;
    let stats = Stats {
        osm_ways,
        osm_nodes,
        nodes: g.nodes(),
        arcs: g.to.len(),
        segments: g.seg_off.len() - 1,
        points: g.pts.len(),
        shortcuts,
    };
    Ok((g, stats))
}

fn length(pts: &[[f32; 2]]) -> f64 {
    pts.windows(2).map(|w| (((w[1][0] - w[0][0]) as f64).powi(2) + ((w[1][1] - w[0][1]) as f64).powi(2)).sqrt()).sum()
}

/// Keep the largest strongly connected component, lay out the arrays, contract.
pub fn build(xy: Vec<[f32; 2]>, segs: Vec<Segment>, lat0: f64, lon0: f64) -> Result<(Graph, usize), String> {
    let n = xy.len();
    // Directed arcs: (from, to, weight dm rounded up, segment ref)
    let mut arcs: Vec<(u32, u32, u32, u32)> = vec![];
    for (i, s) in segs.iter().enumerate() {
        let w = (length(&s.pts) * 10.0).ceil().max(1.0) as u32;
        if s.fwd {
            arcs.push((s.a, s.b, w, (i as u32) << 1));
        }
        if s.bwd {
            arcs.push((s.b, s.a, w, (i as u32) << 1 | REVERSED));
        }
    }
    let keep = largest_scc(n, &arcs);
    let mut new_id = vec![NONE; n];
    let mut nxy = vec![];
    for v in 0..n {
        if keep[v] {
            new_id[v] = nxy.len() as u32;
            nxy.push(xy[v]);
        }
    }
    arcs.retain(|a| keep[a.0 as usize] && keep[a.1 as usize]);
    for a in &mut arcs {
        a.0 = new_id[a.0 as usize];
        a.1 = new_id[a.1 as usize];
    }
    let n = nxy.len();
    if n < 2 {
        return Err("graph has fewer than two connected nodes".into());
    }
    arcs.sort_unstable();
    let mut off = vec![0u32; n + 1];
    for a in &arcs {
        off[a.0 as usize + 1] += 1;
    }
    for i in 0..n {
        off[i + 1] += off[i];
    }

    // Segment polylines, quantised to whole metres from the south-west corner (fits in u16).
    let (mut mx, mut my) = (f32::MAX, f32::MAX);
    for s in &segs {
        for p in &s.pts {
            mx = mx.min(p[0]);
            my = my.min(p[1]);
        }
    }
    let mut seg_off = vec![0u32];
    let mut pts = vec![];
    for s in &segs {
        for p in &s.pts {
            let q = [(p[0] - mx).round(), (p[1] - my).round()];
            if q[0] > u16::MAX as f32 || q[1] > u16::MAX as f32 {
                return Err("extract larger than 65 km".into());
            }
            pts.push([q[0] as u16, q[1] as u16]);
        }
        seg_off.push(pts.len() as u32);
    }

    let mut g = Graph {
        lat0,
        lon0,
        xy: nxy,
        off,
        to: arcs.iter().map(|a| a.1).collect(),
        w: arcs.iter().map(|a| a.2).collect(),
        seg: arcs.iter().map(|a| a.3).collect(),
        seg_off,
        pts,
        origin: [mx, my],
        ..Default::default()
    };
    let shortcuts = contract(&mut g);
    Ok((g, shortcuts))
}

/// Tarjan's algorithm, iterative; marks the nodes of the largest component.
fn largest_scc(n: usize, arcs: &[(u32, u32, u32, u32)]) -> Vec<bool> {
    let mut adj = vec![vec![]; n];
    for a in arcs {
        adj[a.0 as usize].push(a.1 as usize);
    }
    let mut index = vec![usize::MAX; n];
    let mut low = vec![0; n];
    let mut on = vec![false; n];
    let mut stack = vec![];
    let mut comp = vec![usize::MAX; n];
    let (mut next, mut ncomp) = (0, 0);
    for root in 0..n {
        if index[root] != usize::MAX {
            continue;
        }
        let mut call = vec![(root, 0usize)];
        while let Some(&mut (v, ref mut i)) = call.last_mut() {
            if *i == 0 && index[v] == usize::MAX {
                index[v] = next;
                low[v] = next;
                next += 1;
                stack.push(v);
                on[v] = true;
            }
            if *i < adj[v].len() {
                let u = adj[v][*i];
                *i += 1;
                if index[u] == usize::MAX {
                    call.push((u, 0));
                } else if on[u] {
                    low[v] = low[v].min(index[u]);
                }
            } else {
                call.pop();
                if let Some(&(p, _)) = call.last() {
                    low[p] = low[p].min(low[v]);
                }
                if low[v] == index[v] {
                    while let Some(x) = stack.pop() {
                        on[x] = false;
                        comp[x] = ncomp;
                        if x == v {
                            break;
                        }
                    }
                    ncomp += 1;
                }
            }
        }
    }
    let mut size = vec![0; ncomp];
    for &c in &comp {
        size[c] += 1;
    }
    let big = (0..ncomp).max_by_key(|&c| size[c]).unwrap_or(0);
    comp.iter().map(|&c| c == big).collect()
}

/// Contract every node, least important first. Returns the number of shortcuts added.
/// Importance = 2·(shortcuts needed) − (arcs removed) + (neighbours already contracted).
fn contract(g: &mut Graph) -> usize {
    let n = g.nodes();
    let mut out: Vec<HashMap<u32, (u32, u32)>> = vec![HashMap::new(); n]; // to → (w, mid)
    let mut inn: Vec<HashMap<u32, (u32, u32)>> = vec![HashMap::new(); n]; // from → (w, mid)
    #[allow(clippy::needless_range_loop)] // u indexes the CSR offsets and two lists
    for u in 0..n {
        for a in g.off[u] as usize..g.off[u + 1] as usize {
            let (v, w) = (g.to[a], g.w[a]);
            let e = out[u].entry(v).or_insert((w, NONE));
            if w < e.0 {
                *e = (w, NONE);
            }
            let e = inn[v as usize].entry(u as u32).or_insert((w, NONE));
            if w < e.0 {
                *e = (w, NONE);
            }
        }
    }
    let mut done = vec![false; n];
    let mut deleted = vec![0i32; n];
    let mut rank = vec![NONE; n];
    let mut up: Vec<Vec<(u32, u32, u32)>> = vec![vec![]; n];
    let mut dn: Vec<Vec<(u32, u32, u32)>> = vec![vec![]; n];
    let mut witness = Witness::new(n);
    let mut shortcuts_total = 0;

    let mut heap: BinaryHeap<Reverse<(i32, u32)>> = BinaryHeap::new();
    for v in 0..n {
        let p = priority(v, &out, &inn, &done, &deleted, &mut witness, false).0;
        heap.push(Reverse((p, v as u32)));
    }
    let mut order = 0u32;
    while let Some(Reverse((p, v))) = heap.pop() {
        let v = v as usize;
        if done[v] {
            continue;
        }
        // Lazy update: if the node got more important since it was queued, requeue it.
        let (np, _) = priority(v, &out, &inn, &done, &deleted, &mut witness, false);
        if let Some(Reverse((top, _))) = heap.peek()
            && np > p
            && np > *top
        {
            heap.push(Reverse((np, v as u32)));
            continue;
        }
        let (_, new) = priority(v, &out, &inn, &done, &deleted, &mut witness, true);
        done[v] = true;
        rank[v] = order;
        order += 1;
        for (&t, &(w, mid)) in &out[v] {
            up[v].push((t, w, mid));
            inn[t as usize].remove(&(v as u32));
            deleted[t as usize] += 1;
        }
        for (&f, &(w, mid)) in &inn[v] {
            dn[v].push((f, w, mid));
            out[f as usize].remove(&(v as u32));
            deleted[f as usize] += 1;
        }
        out[v].clear();
        inn[v].clear();
        for (a, b, w) in new {
            let e = out[a as usize].entry(b).or_insert((u32::MAX, NONE));
            if w < e.0 {
                *e = (w, v as u32);
                inn[b as usize].insert(a, (w, v as u32));
                shortcuts_total += 1;
            }
        }
    }

    let flat = |lists: &mut Vec<Vec<(u32, u32, u32)>>| {
        let mut off = vec![0u32];
        let (mut a, mut b, mut c) = (vec![], vec![], vec![]);
        for l in lists.iter_mut() {
            l.sort_unstable();
            for &(x, w, m) in l.iter() {
                a.push(x);
                b.push(w);
                c.push(m);
            }
            off.push(a.len() as u32);
        }
        (off, a, b, c)
    };
    let (uo, ut, uw, um) = flat(&mut up);
    let (dno, df, dw, dm) = flat(&mut dn);
    g.rank = rank;
    g.up_off = uo;
    g.up_to = ut;
    g.up_w = uw;
    g.up_mid = um;
    g.dn_off = dno;
    g.dn_from = df;
    g.dn_w = dw;
    g.dn_mid = dm;
    shortcuts_total
}

/// Shortcuts contracting `v` would need, and its priority. With `apply`, returns them.
fn priority(
    v: usize,
    out: &[HashMap<u32, (u32, u32)>],
    inn: &[HashMap<u32, (u32, u32)>],
    done: &[bool],
    deleted: &[i32],
    witness: &mut Witness,
    apply: bool,
) -> (i32, Vec<(u32, u32, u32)>) {
    let mut needed = vec![];
    for (&u, &(w1, _)) in &inn[v] {
        let max = out[v].iter().filter(|(t, _)| **t != u).map(|(_, &(w2, _))| w1 + w2).max();
        let Some(max) = max else { continue };
        witness.run(u, v as u32, max, out, done);
        for (&t, &(w2, _)) in &out[v] {
            if t != u && witness.dist(t) > w1 + w2 {
                needed.push((u, t, w1 + w2));
            }
        }
    }
    let removed = (inn[v].len() + out[v].len()) as i32;
    let p = 2 * needed.len() as i32 - removed + deleted[v];
    (p, if apply { needed } else { vec![] })
}

/// Bounded Dijkstra among uncontracted nodes, avoiding the node being contracted.
struct Witness {
    dist: Vec<u32>,
    stamp: Vec<u32>,
    epoch: u32,
    heap: BinaryHeap<Reverse<(u32, u32)>>,
}

impl Witness {
    fn new(n: usize) -> Witness {
        Witness { dist: vec![0; n], stamp: vec![0; n], epoch: 0, heap: BinaryHeap::new() }
    }
    fn dist(&self, v: u32) -> u32 {
        if self.stamp[v as usize] == self.epoch { self.dist[v as usize] } else { u32::MAX }
    }
    fn run(&mut self, s: u32, avoid: u32, max: u32, out: &[HashMap<u32, (u32, u32)>], done: &[bool]) {
        self.epoch += 1;
        self.heap.clear();
        self.stamp[s as usize] = self.epoch;
        self.dist[s as usize] = 0;
        self.heap.push(Reverse((0, s)));
        let mut settled = 0;
        while let Some(Reverse((d, v))) = self.heap.pop() {
            if d > self.dist(v) || d > max {
                if d > max {
                    break;
                }
                continue;
            }
            settled += 1;
            if settled > 600 {
                break; // a missing witness only costs an extra shortcut, never correctness
            }
            for (&t, &(w, _)) in &out[v as usize] {
                if t == avoid || done[t as usize] {
                    continue;
                }
                let nd = d + w;
                if nd < self.dist(t) {
                    self.stamp[t as usize] = self.epoch;
                    self.dist[t as usize] = nd;
                    self.heap.push(Reverse((nd, t)));
                }
            }
        }
    }
}
