# Milestone 1 — hand-over notes (27 October 2026)

Spec phases 1–2 (foundation + signature), plus most of the phase-3 instruments that need no
WebAssembly. Everything below was built, run and checked in Chrome 155 on Linux against the real
Rust server; the numbers quoted were measured in this repository.

## What works

| Area | State |
|---|---|
| Rust site generator (`crates/site`) | All of S1–S14 in Maud templates; Fluent strings; Lightning CSS; `@font-face` with metric-matched fallbacks; a build-time glyph-coverage check that reads the shipped WOFF2 files' `cmap`; precompression; manifest; budgets; curl-mode ANSI art rasterised from the logo SVG; `security.txt`, `sitemap.xml` (hreflang), `humans.txt`, `robots.txt`, web manifest, privacy page, 404, no-JS contact result page |
| Rust server (`crates/server`) | Axum + rustls (TLS 1.3, **X25519MLKEM768 negotiated and reported**), HTTP/2, full security headers on every response, `Server-Timing` incl. kernel RTT, `/_proof`, `/_telemetry` (SSE), `/_ping`, curl mode, Accept-Language without redirects, 404 suggestions that never echo the path, contact API with proof of work, honeypot, in-memory rate limits and **age encryption at rest** (server holds only the public key) |
| Hero engine (`web/hero`) | One physics, three renderers, each its own lazy chunk. **Tier A: WebGPU compute, 120 000 particles — measured p90 frame time 5.8 ms on this laptop's Intel iGPU (budget 12 ms).** Tier B: WebGL2 transform feedback, 40 000. Tier C: Canvas 2D twin. Tier D: the real mark as poster (no JS, reduced motion, Data Saver). The simulation runs in the logo's own geometry (two grown quads + the 22-unit belt channel): requests fill the top bulb, turn Mintaka blue along the belt, fan into the bottom bulb and leave in four lanes; scroll turns them into the S2 light ray, then into the real Orion (projected from J2000 coordinates) while the logo's stars travel to their places in the sky. The tier probe judges the renderer's *own* frame time (GPU timing), as the spec says — not animation-frame gaps, which browsers throttle for unrelated reasons |
| Islands | Proof console (`P`), terminal (`~`), DevTools greeting + `mintaka.help()`, S2 physics (no IP geolocation: time zone + speed of light), S5 armour (live header + TLS scan, Trusted Types check), S6 follow one request, S6 live load (SSE), S7 Fitts / Hick–Hyman / pop-out with on-device regression, S13 contact enhancement with a hand-written SHA-256 proof-of-work worker |
| Honesty mechanics | Mono means measured; unverified claims show an UNVERIFIED badge in development and are **dropped from release builds**; `launch_ready` flag; build fails on missing strings, missing glyphs (release) and blown budgets (release) |
| Tests | 107 TypeScript unit tests; 103 Rust tests (13 generator, 47 server unit, 36 server integration incl. a TLS test proving X25519MLKEM768, 7 shared types) — all green; clippy clean |

Measured (release build): first screen 11.0 kB compressed (budget 14 kB, one round trip); first
full view 64 kB (budget 100 kB); hero 5.4 kB core + one renderer chunk of 14–16 kB (25 kB); HUD
5.7 kB (6 kB). In the browser the page is **cross-origin isolated** and runs with **zero CSP or
Trusted Types violations**; an in-page WCAG check found no contrast, naming, labelling, heading
or duplicate-id problems.

Server (release, this laptop, load generator on the same machine, see `bench/README.md`): a fixed
50 000 requests/s for 10 s, 100 % success; server time per request p50 3 µs, p99 26 µs.

## Corrections to the spec (fact-checked while building)

0. **The server cannot force post-quantum key exchange.** rustls picks among shared groups in the
   *client's* preference order; Chrome, Firefox, Safari and curl/OpenSSL 3.5+ put X25519MLKEM768
   first, so they get it. A classical-only client gets X25519 and the page says so honestly.

1. **rustls 0.23.46 already prefers X25519MLKEM768** — `prefer-post-quantum` is a *default*
   feature now. The rule is "don't disable default features", not "enable it".
2. **WebGPU is available on Linux** in Chrome 155 (this machine: Intel Gen-12 adapter). The spec's
   November 2025 table is out of date; re-check the matrix at launch.
3. **Space Grotesk's Latin subset lacks →, ≤, ≈, λ, ₂, ✓, ✗.** Those glyphs come from Onest's math
   subset (lazy) or are drawn as SVG. A single "ṭ" pulled a 19 kB Latin-ext font; the copy now uses
   "al-mintaqa".
4. **The root URL serves the page directly** with `Vary: Accept-Language` instead of the spec's
   302 — a redirect costs a full round trip on a page about round trips.
5. **S2 uses no IP database.** RTT gives a hard upper bound on distance (light in fibre covers
   204.2 km/ms); the visitor's *time zone* gives a city for the physics floor — computed in the
   browser, nothing sent. If the estimate is physically impossible, the page says so.
6. **Single-key shortcuts (`P`, `~`) can be switched off** in the footer — WCAG 2.2 SC 2.1.4 needs
   that; the spec didn't mention it.
7. **Orion's star coordinates** shown on the page are real (J2000, ICRS), and the constellation is
   projected from them; the brand board's full-Orion figure is the same projection.

## Placeholders and decisions waiting for you

- Domain (`mintaka.tj`) and every contact address in `content/site.toml` → then `launch_ready = true`.
- The 14 practice claims in `content/site.toml` — mark each `verified = true` only when it is true
  today (release builds hide the rest).
- Contact: reply-time promise ("one working day"), whether to show budget ranges (not shown yet),
  the production age recipient key and Telegram bot.
- Team section: real names, photos with consent, and the team's own words for the story.
- Russian and Tajik copy, written natively (`content/ru/`, `content/tg/`), then add them to `langs`.
- Hosting region (sets `--location`, which the physics instrument relies on).

## Next milestone

- WebAssembly instruments (needs `sudo pacman -S rust-wasm binaryen wasm-bindgen`): S4 routing race
  on Dushanbe's roads (OpenStreetMap, Contraction Hierarchies), S5 payload analyser, S6 overload lab.
- ACME certificates inside the binary, HTTP/3, the console challenge (CTF), Lab posts + RSS,
  self-hosted Web Vitals (`/_rum`), offline mode, the published benchmark on production hardware.
- Open question to chase: Chrome requests Latin-ext and Cyrillic font files at first style pass
  even when no rendered text needs them (seen with cache warm); trace with a font-loading log.
