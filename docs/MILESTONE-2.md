# Milestone 2 — WebAssembly instruments (28 October 2026)

Tests now: 121 Rust (generator, server, router incl. 1,000 real-pair agreement, simulator) and
114 TypeScript, all green; release build within every budget.

## Pipeline

`cargo xtask build` now starts with Rust → WebAssembly: every crate marked
`[package.metadata.mintaka] wasm = true` is compiled in one cargo call (`--profile wasm`), then
`wasm-bindgen --target web` writes typed glue into `web/wasm/<name>/` (generated, gitignored) and
`wasm-opt -O3` optimises the binary. esbuild bundles the glue with the island; the generator ships
the `.wasm` as a hashed asset (`#mintaka-assets` → `<name>.wasm`) with a 250 kB budget, and data
files from `assets/data/*.bin` the same way (3 MB budget). Conventions: docs/CONTRACTS.md §13.

## S4 · Route across Dushanbe — done

- **Data:** OpenStreetMap extract of 2026-10-28 (Overpass API, car-routable roads in a box around
  Dushanbe): 5,493 ways, 43,906 nodes → 8,393 intersections, 18,980 directed arcs (one-way streets
  respected), largest strongly connected component only, full road geometry kept for drawing.
  `cargo xtask graph [--fetch]` rebuilds it; provenance and licence in `assets/data/`.
- **Contraction Hierarchies, our own implementation** (`crates/routing/src/prep.rs`): bounded
  witness searches, lazy priority updates, 17,605 shortcuts, built in 0.6 s. The graph file is
  1.14 MB raw, **422 kB on the wire**.
- **Correctness:** A* and CH return exactly Dijkstra's distance, with valid routes whose weights
  add up, on 4 random graphs × 300 pairs and on **1,000 random pairs across Dushanbe** (the spec's
  "done when").
- **In the browser** (44.9 kB of WebAssembly + 12.9 kB island): three maps race the same route;
  *Watch* replays each search at the same pace, *Race* shows the result at once; counters are
  intersections explored, median time of 9 runs (CH batched ×50 to beat timer resolution) and
  speed-up. Railway station → airport, measured here: Dijkstra 4,867 intersections in 290 µs, A*
  879 in 34 µs, CH **88 in 6.4 µs** — "Same roads. Same answer. 55× less work."
- Not done: the multi-threaded "10,000 random routes" batch mode (WebAssembly threads need nightly
  Rust with `build-std`; the page is already cross-origin isolated, so it can be added later).

## S6 · Overload lab — done

- **Model** (`crates/loadsim`, seeded xoshiro256++, no clock or OS randomness): Poisson arrivals at
  20 requests/s per ×1 (×100 = 2,000/s), lognormal service times (mean 20 ms, p99 ≈ 93 ms); clients
  time out at 2 s and retry twice; both systems see identical arrivals.
  *Naive*: 8 workers, unbounded FIFO, abandoned requests still served (capped at 10,000 queued to
  bound memory — the page says so). *Built our way*: CoDel-style shedding (50 ms target for
  100 ms) with an immediate 503, at most 4 queued per worker, autoscaling to 70 % utilisation with
  a 5 s start-up, 8–32 workers.
- **Verified** against queueing theory: M/M/1 and M/M/4 (Erlang C) waiting times and L = λW over
  long runs, plus determinism, shedding, scale-in and collapse behaviour (16 Rust tests).
- **What visitors see:** at ×40 the naive system answers 0 of ~2,300 offered requests/s (retries
  tripled the load) while ours answers all ~850/s at p99 ≈ 90 ms on 25 workers; Little's law holds
  for ours (L 16 ≈ 804/s × 20 ms) and visibly breaks for the naive one while its queue explodes.
  Back at ×1, the naive system needs ~30 s to drain work nobody is waiting for.
- 40.3 kB of WebAssembly, 10.4 kB island; says "simulation" on screen; no network calls.

## S5 · Try to break this — on hold

The payload analyser (recognise classic attack strings, explain which defence stops each) was
paused when an automated safety check stopped the worker building its pattern corpus. It is
defensive and runs only in the visitor's browser, but it needs the owner's explicit go-ahead
before work resumes. The S5 frame shows its static explanation until then.
