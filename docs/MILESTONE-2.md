# Milestone 2 — WebAssembly instruments (28 October 2026)

Tests now: 129 Rust (generator, server, router incl. 1,000 real-pair agreement, simulator, fuzzer)
and 127 TypeScript, all green; release build within every budget.

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

## S5 · Attack our code — done

The original "Try to break this" payload analyser was dropped. In its slot, between the
offence/defence lists and "This page's armour", S5 now shows a fuzzer attacking our own code.

- **What it is** (`crates/fuzzer`, no dependencies): a small coverage-guided fuzzer in the spirit of
  libFuzzer/AFL, attacking a parser for a tiny binary star-chart format. The parser ships in two
  builds: v1 trusts a length byte from its input (`target.rs:129`), v2 is the one-line fix two lines
  below. 39 hand-placed coverage points; seeded xorshift64\*, so every run replays from its seed.
- **Three chapters in the browser:** *Attack* starts from four meaningless bytes (`star`), learns
  the format with no dictionary, lights Orion path by path from Meissa down to Saiph, and crashes
  v1 — the card shows Rust's own panic message captured through the panic hook
  (`panicked at crates/fuzzer/src/target.rs:129:42`), the input minimised to its 1-minimal core,
  and the fix as a two-line diff. *Fix* runs v2 for 10,000,000 inputs: 0 crashes, 38 of 39 paths
  (the dark one, a chart named exactly MINTAKA, explains what coverage feedback can't see).
  *Compare* runs the same engine blind: 2 of 39 paths. Plus *Try to crash it yourself*, whose
  crash message links to `#careers`.
- **Measured here:** the bug is found after 7,492–510,441 inputs over 20 seeds (median 39,127);
  in Chrome a guided run found it after 38,164 inputs in 5.0 s of Watch mode; the fixed build took
  10 M inputs in 9.0 s of fuzzing time; blind reached 2 paths in 7.8 M inputs. No console errors or
  CSP/Trusted Types violations; no horizontal scroll at 390 px.
- **Tests:** 8 Rust (the bug is real, the fix holds over 2 M random and mutated inputs, guidance
  finds it for all 20 seeds, blind never passes the header, determinism) and 13 TypeScript against
  the built module (panic location matches the source line, minimisation, strings both ways).
- 26.3 kB of WebAssembly, 20.5 kB island (its own 21 kB budget); runs only after a click and only
  while on screen, in ≤ 10 ms main-thread slices; touches nothing but the embedded parser.
- Built from a drop-in prepared outside this repo, then ported to the house conventions: shared
  `h`/`t`/`assetUrl`, rendering inside the instrument frame, strings in `fuzzer.ftl`, tokens and
  components in CSS, tests against the built module instead of a checked-in binary.
