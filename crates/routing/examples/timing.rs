//! The timings in the Lab post on Contraction Hierarchies (`content/en/lab/2-contraction-hierarchies.md`).
//!
//! ```sh
//! cargo run --release -p routing --features prep --example timing [-- PAIRS]
//! ```
//!
//! * Preprocessing: the Overpass extract in `target/osm/dushanbe-roads.json` (what `cargo xtask
//!   graph` builds from) → graph → hierarchy, five times, and whether the result is byte for byte
//!   the committed `assets/data/dushanbe.bin`. Skipped when there is no extract; a fresh one
//!   (`cargo xtask graph --fetch`) is today's map, so it builds a different graph.
//! * Queries on the committed graph: PAIRS random start and destination pairs (default 10,000;
//!   xorshift, seed 42), each algorithm in its own pass after 1,000 warm-up queries, every query
//!   timed with `Instant::now()`, rebuilding the route as road arcs included.
//! * The home page's preset route, railway station → airport: median of 101 runs.
use routing::search::nearest;
use routing::{Algo, Graph, NONE, Search};
use std::path::Path;
use std::time::{Duration, Instant};

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
}

/// Nearest-rank percentile of sorted values.
fn pct(sorted: &[f64], p: f64) -> f64 {
    sorted[((sorted.len() as f64 - 1.0) * p).round() as usize]
}

fn sorted(mut v: Vec<f64>) -> Vec<f64> {
    v.sort_by(f64::total_cmp);
    v
}

fn summary(name: &str, v: &[f64], unit: &str) {
    let mean = v.iter().sum::<f64>() / v.len() as f64;
    println!(
        "  {name:<14} median {:>9.2} {unit}  mean {mean:>9.2}  p90 {:>9.2}  p99 {:>9.2}  max {:>9.2}",
        pct(v, 0.5),
        pct(v, 0.9),
        pct(v, 0.99),
        v[v.len() - 1]
    );
}

fn main() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let pairs: usize = std::env::args().nth(1).map_or(10_000, |s| s.parse().expect("PAIRS is a number"));
    let ms = |d: Duration| d.as_secs_f64() * 1e3;
    let committed = std::fs::read(root.join("assets/data/dushanbe.bin")).expect("assets/data/dushanbe.bin");

    // Preprocessing: Overpass JSON → graph → contraction (five runs each, median).
    match std::fs::read_to_string(root.join("target/osm/dushanbe-roads.json")) {
        Err(_) => println!("preprocessing: skipped, no target/osm/dushanbe-roads.json (`cargo xtask graph --fetch` downloads one)"),
        Ok(json) => {
            let mut parse = vec![];
            let mut full = vec![];
            let mut built = None;
            for _ in 0..5 {
                let t = Instant::now();
                let v: serde_json::Value = serde_json::from_str(&json).expect("Overpass JSON");
                parse.push(t.elapsed());
                drop(v);
            }
            for _ in 0..5 {
                let t = Instant::now();
                let out = routing::prep::from_overpass(&json).expect("a road graph");
                full.push(t.elapsed());
                built = Some(out);
            }
            parse.sort();
            full.sort();
            let (g, st) = built.expect("five runs");
            println!("preprocessing (5 runs each, median):");
            println!("  JSON parse only      {:.0} ms  (all: {:?})", ms(parse[2]), parse.iter().map(|d| ms(*d).round()).collect::<Vec<_>>());
            println!("  from_overpass total  {:.0} ms  (all: {:?})", ms(full[2]), full.iter().map(|d| ms(*d).round()).collect::<Vec<_>>());
            println!("  => graph + SCC + contraction ~ {:.0} ms", ms(full[2]) - ms(parse[2]));
            println!(
                "  ways {} OSM nodes {} intersections {} arcs {} segments {} points {} shortcut insertions {}",
                st.osm_ways, st.osm_nodes, st.nodes, st.arcs, st.segments, st.points, st.shortcuts
            );
            let rebuilt = g.to_bytes();
            println!("  rebuilt {} bytes, committed {} bytes, identical: {}", rebuilt.len(), committed.len(), rebuilt == committed);
        }
    }

    let g = Graph::from_bytes(&committed).expect("a valid graph file");
    let (up, down) = (g.up_to.len(), g.dn_from.len());
    let shortcuts = g.up_mid.iter().chain(&g.dn_mid).filter(|&&m| m != NONE).count();
    println!(
        "graph: {} intersections, {} arcs; hierarchy {} upward + {} downward arcs, {shortcuts} of them shortcuts",
        g.nodes(),
        g.to.len(),
        up,
        down
    );

    // Queries: the same random pairs for each algorithm, one pass per algorithm.
    let mut rng = Rng(42);
    let qs: Vec<(u32, u32)> = (0..pairs).map(|_| (rng.below(g.nodes()) as u32, rng.below(g.nodes()) as u32)).collect();
    let mut s = Search::new(&g);
    let mut dists: Vec<Vec<u32>> = vec![];
    let mut medians = vec![];
    println!("queries: {pairs} random pairs (xorshift, seed 42), each algorithm in its own pass after 1,000 warm-up queries");
    for (name, algo) in [("Dijkstra", Algo::Dijkstra), ("A*", Algo::AStar), ("CH", Algo::Ch)] {
        for &(a, b) in qs.iter().take(1000) {
            s.run(&g, algo, a, b, false);
        }
        let mut times = Vec::with_capacity(pairs);
        let mut settled = Vec::with_capacity(pairs);
        let mut d = Vec::with_capacity(pairs);
        let pass = Instant::now();
        for &(a, b) in &qs {
            let t = Instant::now();
            s.run(&g, algo, a, b, false);
            times.push(t.elapsed().as_secs_f64() * 1e6);
            settled.push(f64::from(s.settled));
            d.push(s.dist);
        }
        let total = pass.elapsed();
        let (times, settled) = (sorted(times), sorted(settled));
        println!(" {name}: pass {:.1} ms ({:.0} queries/s)", ms(total), pairs as f64 / total.as_secs_f64());
        summary("time", &times, "µs");
        summary("settled", &settled, "  ");
        medians.push((name, pct(&times, 0.5), pct(&settled, 0.5)));
        dists.push(d);
    }
    println!("  all three agree on every distance: {}", dists[0] == dists[1] && dists[0] == dists[2]);
    for (name, t, n) in &medians {
        println!("  {name:<9} median {t:.2} µs, {n:.0} settled (Dijkstra ÷ this: time {:.1}×, settled {:.1}×)", medians[0].1 / t, medians[0].2 / n);
    }

    // The preset route of the home page (web/instruments/routing/index.ts).
    let at = |lat: f64, lon: f64| {
        let xy = g.project(lat, lon);
        nearest(&g, xy[0], xy[1])
    };
    let (a, b) = (at(38.55688, 68.79968), at(38.54558, 68.8274));
    println!("railway station {a} → airport {b} (101 runs each, median):");
    for (name, algo) in [("Dijkstra", Algo::Dijkstra), ("A*", Algo::AStar), ("CH", Algo::Ch)] {
        let mut ts = vec![];
        for _ in 0..101 {
            let t = Instant::now();
            s.run(&g, algo, a, b, false);
            ts.push(t.elapsed().as_secs_f64() * 1e6);
        }
        let ts = sorted(ts);
        println!("  {name:<9} {:.2} µs, {} settled, {:.1} m, {} road arcs", ts[50], s.settled, f64::from(s.dist) / 10.0, s.path.len());
    }
}
