# Milestone 3 — team, challenge, Lab, real-user metrics, offline, certificates, HTTP/3 (29 October 2026)

Spec phase 4 ("delight and launch") minus what needs people or hardware we don't have yet: the
Russian and Tajik copy, and a benchmark on the production machine.

## Team (S12)

Two cards from `[[team]]` in `content/site.toml`: Sharifzoda Bilol (founder) and Amirhamza
Sayfiddinzoda (co-founder) — portrait, name, role, one line, GitHub and personal site. Portraits are
square head-and-shoulders crops of the photos each person publishes on their own site (AVIF 6–7 kB,
WebP fallback 8–10 kB, `assets/team/`). The one-line descriptions are drafts for their owners to
rewrite (`team-<id>-line` in `content/en/page.ftl`). Both are listed in `humans.txt`.

## Console challenge (spec §7.5)

`mintaka.ctf()` in DevTools starts four puzzles, one per pillar star (CONTRACTS §8.3):

| Stage | Star | Puzzle | The skill |
|---|---|---|---|
| 1 | Rigel | cheapest route across a 100 × 100 grid of digit costs | Dijkstra (a right/down sweep gives a different number on every grid) |
| 2 | Saiph | two messages under one reused one-time pad | crib dragging; the crib is the view-source greeting |
| 3 | Betelgeuse | 280 raw bytes of `struct tcp_info` | reading a kernel struct: cwnd × MSS ÷ RTT |
| 4 | Meissa | 40 simulated pointing trials | a least-squares fit of Fitts's law |

- **The public repo shows no answers.** Runs are stateless MAC'd tokens; puzzles and answers are
  derived from a 32-byte secret in `{data-dir}/ctf.key`. Answers are rate-limited per network.
- **Solving all four opens an application form** in `#careers`. It goes through the contact API
  and arrives as "New application (console challenge solved)".
- **Verified:** a Rust integration test plays all four stages over HTTP with solvers that see only
  the puzzle data, then applies with the pass; the same in Chrome with JavaScript solvers (JS and
  Rust agree on Meissa's rounding). The `tcp_info` layout is checked against `libc::tcp_info`.
- The puzzle text is a lazy string bundle (`content/en/islands/lazy/ctf.ftl`): fetched on the first
  challenge command instead of riding inline on every page (−1.0 kB brotli on the home page).

## The Lab (S11)

Four posts in `content/en/lab/`, rendered at build time (pulldown-cmark, MathML via
pulldown-latex, our own highlighter), **no JavaScript on Lab pages**, an Atom feed, sitemap
entries, and the home page's S11 list linking to them. Code excerpts are read from the repository
at build time and link to the exact lines on GitHub; a moved anchor fails the build.

- Every number in the posts was measured on 2026-10-29 on this laptop (machine, commands and
  caveats in each post) or cited: FIPS 203 and RFC 10024 (checked against the published RFC).
- Release budgets: each Lab page's first screen 8.6–8.9 kB brotli (≤ 14 kB), whole pages
  10.0–14.6 kB (≤ 32 kB), feed 17.6 kB.

## Real-user metrics and offline mode (spec §12, §7.9)

- **`/_rum`:** TTFB, FCP, LCP, CLS and INP from PerformanceObserver plus the §12 event counts, one
  `sendBeacon` per page view. Aggregated in memory (since start and the last 24 h, p75 only from 20
  samples), no IP, user agent or cookie, nothing on disk; Global Privacy Control and Do Not Track are
  honoured on both ends. The proof console shows the totals and says when there are too few.
- **Offline:** a service worker (`/sw.js`) caches the shell and the assets a visit used. While the
  browser is online it leaves page loads alone, because answering them from the worker would
  replace the Navigation Timing that S2, S6 and `/_rum` read. With no connection, pages come from
  the cache and live instruments show offline states.
- **Verified in Chrome:** the worker registers under Trusted Types and controls the page, which
  stays cross-origin isolated; beacons arrive; the proof console row renders. **Not verified:** an
  actual offline page load — the automated browser can't switch the network off. Check with DevTools
  → Network → Offline (or Application → Service workers → Offline).

## Certificates and HTTP/3

- **Let's Encrypt from inside the binary** (`--acme-domain`, CONTRACTS §16.3): instant-acme plus
  our own resolver, cache (`{data-dir}/acme`, files 0600, written atomically) and renewal (30 days
  before expiry, or a third of the lifetime for short certificates), hot-swapped without a restart
  for TCP and QUIC at once. Validation is TLS-ALPN-01 on port 443 through the existing ClientHello
  hook, so nothing listens on port 80 for it.
- **Tested** against a small RFC 8555 CA inside the test process: account, orders, four real
  TLS-ALPN-01 validations of the listener, renewal, restart from the cache. **Not run:** an issuance
  from the real Let's Encrypt, which needs the domain pointing at the server. README → "Certificates
  and HTTP/3 in production" has the staging command, the CAA record and the firewall ports.
- **HTTP/3** (`--http3`, CONTRACTS §16.2): quinn + h3 on UDP, through the same router and
  `routes::finish` as HTTP/1.1 and HTTP/2, announced with `Alt-Svc`. quinn doesn't expose the
  negotiated cipher or key-exchange group, so the server reads both from the ServerHello it wrote
  itself; `/_proof` reports `null` rather than guess if that ever fails. QUIC's own RTT estimate is
  reported separately (`quic_rtt_us`, Server-Timing `qrtt`), never as the kernel's.
- **Measured here** with curl 8.23 over loopback: `/_proof` over HTTP/3 reports `HTTP/3.0`, `h3`,
  `TLS13_AES_256_GCM_SHA384`, `X25519MLKEM768` — the same suite and group curl reports itself.
  **Not verified:** Chrome over HTTP/3 (it won't use QUIC with a self-signed certificate) and real
  networks with loss.
- S2 physics, S6 trace and the terminal read the kernel RTT (`krtt`), which HTTP/3 doesn't have;
  over HTTP/3 they fall back (physics to its own ping). Using `qrtt` there is a follow-up.

## Benchmark

`bench/run.sh --label <machine>` is the published benchmark as one command: release server and
site, fixed-rate oha with latency correction over HTTP/1.1 and over TLS 1.3 + HTTP/2, the server's
own `/_telemetry` alongside, and a Markdown report with the machine's facts in `bench/results/`.

First run: [`bench/results/2026-10-29-dev-laptop.md`](../bench/results/2026-10-29-dev-laptop.md)
(this laptop, load generator on the same machine, the real 30.2 kB home page):

| Protocol, rate | Client p50 | Client p99 | Server `app` p50 | Server `app` p99 |
|---|---:|---:|---:|---:|
| HTTP/1.1, 20,000/s | 0.163 ms | 0.386 ms | 4 µs | 23–32 µs |
| HTTP/1.1, 50,000/s | 0.269 ms | 0.676 ms | 4 µs | 15–20 µs |
| HTTP/2 + TLS 1.3, 20,000/s | 0.464 ms | 1.321 ms | 4–8 µs | 15–20 µs |
| HTTP/2 + TLS 1.3, 50,000/s | 1.056 ms | 34.0 ms | 7–8 µs | 21–28 µs |

At 50,000/s over TLS the server used 4–5 cores and oha encrypted the other side on the same 16
threads; the server's own p99 stayed under 30 µs, so the 34 ms client tail is on the client side of
the loopback, not in the request path. The run on production hardware is still to do: the same
command on the server, with nothing else busy.

## Tests and budgets

201 Rust tests and 150 TypeScript tests, all green; clippy shows only the deliberate
`collapsible_if`. Release build: home page first screen 11.2 kB and whole page 30.2 kB brotli
(budgets 14 and 32), Lab pages 8.6–8.9 kB first screen and 10.0–14.5 kB whole, first full view
79.4 kB (budget 100 — now counting the shared chunks the first-view modules import, which it
used to leave out).

## Found along the way

- **CONTRACTS §12 was wrong about λ and ₂:** Onest's math subset has no Greek letters or subscript
  digits, so they render in a system font. Corrected. The S6 overload lab still prints `L = λW`
  from TypeScript (`web/instruments/loadsim/index.ts`), which the build's glyph check can't see, so
  that λ is drawn by a system font.
- **A privacy comment overstated retention:** `routes/proof.rs` said rate limiters keep a network
  prefix "for at most an hour"; they keep it until idle ones are pruned at 10,000 networks or the
  server restarts. The comment now says so (`ip_stored: false` was and is true).
- **The first-view budget undercounts:** it adds the entry chunks but not the shared `chunk.*` files
  they import.

## Waiting on you

- Read and rewrite the two team lines; swap the portraits if you prefer others.
- Read the four Lab posts before launch: they speak for the company.
- The items from MILESTONE-1 still open: domain and contacts, the 14 practice claims, the
  reply-time promise and budget ranges, the production age recipient and Telegram bot, the hosting
  region, the Russian and Tajik copy, and a license for the repository.
