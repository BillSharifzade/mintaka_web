# mintaka-web

The landing page for **Mintaka** — a Rust-first software engineering studio from Dushanbe.
Product intent: the landing-page concept and build spec (kept outside this repository; § numbers in the code refer to it).
Interfaces between the parts: [`docs/CONTRACTS.md`](docs/CONTRACTS.md).

> **The page is the proof.** Every claim on it is either measured live, in front of the visitor,
> or something they can try with their own hands.

## Run it

Requirements: Rust 1.90+ (built with 1.99) with the `wasm32-unknown-unknown` target,
`wasm-bindgen` 0.2.128 and `wasm-opt` (Arch: `pacman -S rust-wasm wasm-bindgen binaryen`), Node 22+
with npm (for esbuild and TypeScript only — the page ships no third-party JavaScript), a C compiler
(for `aws-lc-rs`).

```sh
npm install                      # esbuild + TypeScript (dev tools only)
cargo xtask build                # TypeScript → CSS → pages → dist/, with a budget report
cargo run -p server -- --dist dist --listen 127.0.0.1:8761 --location "Dushanbe,TJ,38.5598,68.7870"
# → http://127.0.0.1:8761/      (add --tls-self-signed --tls-listen 127.0.0.1:8762 for
#                                 https://127.0.0.1:8762/ with post-quantum TLS)

cargo xtask dev                  # build, serve on :8761, rebuild + restart on every change
                                 # (MINTAKA_LISTEN=127.0.0.1:9000 and MINTAKA_LOCATION=… override)
cargo xtask build --release      # drops unverified claims; fails on budgets, missing glyphs or strings
cargo xtask graph [--fetch]      # rebuild assets/data/dushanbe.bin from OpenStreetMap (Overpass API)
```

Check it from a terminal: `curl http://127.0.0.1:8761/` (curl mode), `curl -sk https://127.0.0.1:8762/_proof` (look for `"group": "X25519MLKEM768"`).

Tests: `cargo test --workspace` · `npx tsc --noEmit -p .` · `node --test "web/**/*.test.ts"` (after one `cargo xtask build`: the fuzzer tests load the built module).

## How it fits together

```
content/ (Fluent strings, site.toml) ─┐
web/styles/ (CSS)  ── Lightning CSS ──┤
web/**/*.ts  ── esbuild (xtask) ──────┼─► crates/site (Rust generator) ─► dist/ + manifest.json
assets/ (fonts, brand kit files) ─────┘                                        │
                                                                               ▼
                              crates/server (Axum + rustls, TLS 1.3, X25519MLKEM768)
                              loads every route into memory at startup; per request it only
                              picks an encoding and adds headers, Server-Timing and /_proof facts
```

| Path | What |
|---|---|
| `crates/site` | Generator: Maud templates (`src/pages/`), Fluent strings, Lightning CSS, font `@font-face` + metric-matched fallbacks + a WOFF2 glyph-coverage check, curl-mode ANSI art rasterised from the logo SVG, `security.txt`/`sitemap.xml`/`humans.txt`, precompression, budgets |
| `crates/server` | The server: headers, Server-Timing (incl. kernel RTT), `/_proof`, `/_telemetry` (SSE), `/_ping`, curl mode, 404 suggestions, contact API with proof of work and age encryption |
| `crates/proof` | Types shared by both (manifest, `/_proof`, telemetry, contact) |
| `crates/fuzzer` | S5: a coverage-guided fuzzer attacking a small parser of ours (v1 with one bug, v2 fixed) → WebAssembly without bindgen |
| `crates/routing` | S4: road graph, Dijkstra, A*, our own Contraction Hierarchies (build-time `prep` feature turns an OpenStreetMap extract into `assets/data/dushanbe.bin`) → WebAssembly |
| `crates/loadsim` | S6: discrete-event queueing simulator (naive vs. load-shedding + autoscaling) → WebAssembly |
| `xtask` | `cargo xtask build / dev` |
| `web/hero` | The hourglass engine: WebGL2 transform-feedback particle simulation in logo space (tier B), Canvas 2D twin (tier C), static poster (tier D) |
| `web/hud`, `web/terminal`, `web/console` | Proof console (`P`), terminal (`~`), DevTools greeting + `mintaka.help()` |
| `web/instruments` | S2 physics, S4 routing race, S5 fuzzer + armour, S6 follow-one-request + live load + overload lab, S7 laws you can feel, S13 contact (+ proof-of-work worker) |
| `web/nav` | Constellation navigation ("you are here") |
| `web/shared` | DOM builder (Trusted Types–safe), strings, `/_proof` client, timing, bus, formatting |
| `content/` | `site.toml` (published facts + claims to verify), `en/page.ftl`, `en/islands/*.ftl` |

## House rules (enforced in code where possible)

- **Mono means measured.** A number in IBM Plex Mono was measured on this page view or computed from a cited source. Never an example.
- **Claims must be true on launch day.** Practices listed in `content/site.toml` are shown with an *UNVERIFIED* badge in development and **dropped from release builds** until marked `verified = true`.
- **Zero third-party requests, zero cookies.** CSP `default-src 'none'`, Trusted Types enforced, COOP + COEP (the page is cross-origin isolated).
- **Budgets** (spec §10.1) are checked on every build: first screen ≤ 14 kB compressed (one round trip), first full view ≤ 100 kB, hero ≤ 25 kB, HUD ≤ 6 kB, each instrument ≤ 15 kB.
- **Languages** are written natively by the team; the build fails on any missing string in an enabled language.

## Status

[`docs/MILESTONE-1.md`](docs/MILESTONE-1.md): what is done, what was measured, where the build
corrected the spec, what is still a placeholder, and what comes next.
