# Contracts

The interfaces between the parts of `mintaka-web`. If code and this file disagree, one of them is
a bug — fix it or raise it, don't silently diverge. Product intent lives in
the landing-page concept and build spec, kept outside this repository (referred to as **the spec**, with its § numbers).

## 1. Ownership

| Area | Files | Owner |
|---|---|---|
| Workspace, contracts, shared TS, tokens/components CSS, boot loader | `Cargo.toml`, `docs/`, `web/shared/`, `web/main.ts`, `web/styles/{tokens,base,components,page,islands}.css` | main |
| Site generator and build pipeline | `crates/site/`, `xtask/`, `content/en/page.ftl`, `web/styles/sections/` | main |
| Hero engine, constellation nav | `web/hero/`, `web/nav/` | main |
| S4 routing race (Rust → WebAssembly) | `crates/routing/`, `web/instruments/routing/`, `assets/data/dushanbe.*`, `routing.css`, `routing.ftl` | main |
| S5 fuzzer (Rust → WebAssembly) | `crates/fuzzer/`, `web/instruments/fuzzer/`, `fuzzer.css`, `fuzzer.ftl` | main |
| S6 overload lab (Rust → WebAssembly) | `crates/loadsim/`, `web/instruments/loadsim/`, `loadsim.css`, `loadsim.ftl` | worker C |
| Console challenge (CTF) | `crates/server/src/ctf/`, `crates/server/src/routes/ctf.rs`, `crates/proof/src/ctf.rs`, `web/console/ctf.ts`, `ctf.ftl` | main |
| Server | `crates/server/`, `crates/proof/src/{proof,telemetry,contact}.rs` (+ re-exports in `lib.rs`), `deploy/`, `bench/` | worker A |
| Proof console, terminal, console greeting, S2 physics, S5 armour | `web/hud/`, `web/terminal/`, `web/console/`, `web/instruments/{physics,armour}.ts`, `web/instruments/tz/`, matching `web/styles/islands/*.css` and `content/en/islands/*.ftl` | worker B |
| S6 follow one request, S6 live load, S7 laws you can feel, S13 contact | `web/instruments/{trace,live,contact,pow.worker}.ts`, `web/instruments/laws/`, matching CSS and FTL | worker C |
| Real-user measurements (§14), offline mode (§15) | `crates/proof/src/rum.rs`, `crates/server/src/{rum/,routes/rum.rs}`, `crates/site/src/sw.rs`, `web/rum/`, `web/sw/`, `web/offline/`, `web/shared/rum.ts`, `web/hud/visitors.ts`, `content/en/islands/offline.ftl` | main |
| Telegram bot (§19): notifications, links, the relay, its memory | `crates/server/src/telegram/`, `crates/server/src/secret.rs`, `bot-*` and `pv-telegram*` in `content/en/page.ftl`, `contact-bot*` in `contact.ftl` | main |
| A/B test (§17): plan and statistics, variant pages, serving, counting, results | `crates/proof/src/experiment.rs`, `crates/server/src/rum/experiment.rs`, `[experiment]` in `content/site.toml` (with `config::Experiment` in `crates/site/src/config.rs`), `web/hud/experiment.ts`, `hero-title-*` and `pv-ab*` in `content/en/page.ftl`, `hud-ab-*` in `hud.ftl` | main |

Only edit files you own. If you need a change in someone else's file (a new bus event, a token,
a DOM hook), make the smallest possible change and list it in your final report.

## 2. Commands

```sh
cargo xtask build            # TS (esbuild) + CSS (Lightning CSS) + pages → dist/  (main)
cargo run -p server -- --dist dist          # serve dist/ on http://127.0.0.1:8080
cargo run -p server -- --dist dist --tls-self-signed --http3   # + https:// and HTTP/3 on 127.0.0.1:8443 (TCP + UDP)
cargo test -p server -p proof               # worker A — never build the whole workspace
npx tsc --noEmit -p .                       # type-check all TypeScript
node --test "web/**/*.test.ts"              # unit tests (Node strips types natively)
```

TypeScript imports use explicit `.ts` extensions (`import { h } from '../shared/dom.ts'`), so the
same files run in Node tests and in esbuild. Only erasable syntax (no `enum`, `namespace`,
parameter properties) — `erasableSyntaxOnly` is on.

## 3. dist/ and the manifest

The generator writes `dist/manifest.json` (Rust types: `crates/proof/src/manifest.rs`). The server
loads every listed route **into memory at startup**, including precompressed siblings
(`file.br`, `file.gz`) named in `encodings`, and never touches the file system per request.
`manifest.json` itself is not served.

While an A/B test runs (§17), `manifest.experiment` describes it, and each page of the test lists
its variants after the control (whose file is `file`) in `variants`:
`[{ "id": "hero-title-hardware", "file": "en/index.hero-title-hardware.html", "encodings": ["br", "gzip"] }, …]`,
in `experiment.variants` order, each with its own precompressed siblings. `MANIFEST_VERSION` is 2
since then: a server that ignored `variants` would serve only the control and refuse every beacon
that names a variant.

`manifest.bot` is what the Telegram bot writes to clients (§19): `lang -> (message id -> text)` for
every id in `proof::BOT_MESSAGES`, rendered from the page strings, with `{brief}` left for the server.

```
dist/
  manifest.json
  en/index.html  en/index.html.br  en/index.html.gz     → /en/
  en/index.hero-title-hardware.html (+ .br/.gz) …      → /en/ as well, while an A/B test runs (§17)
  en/404.html                                          → not_found["en"], status 404
  en/sent/index.html                                   → /en/sent/   (contact, no-JS result)
  en/lab/index.html  en/lab/<slug>/index.html (+ .br/.gz) → /en/lab/, /en/lab/<slug>/   (§9.2)
  en/lab/feed.xml                                      → /en/lab/feed.xml   (Atom, `short`)
  assets/main.3F2A1B9C.js  assets/hero.4D5E6F70.js …    → /assets/…  (immutable)
  assets/fonts/space-grotesk-latin.9A8B7C6D.woff2 …
  favicon.ico favicon.svg apple-touch-icon.png icon-192.png icon-512.png og-image.png
  manifest.webmanifest robots.txt humans.txt sitemap.xml .well-known/security.txt
  sw.js sw.js.br sw.js.gz                               → /sw.js (service worker, §15; policy `document`)
  text/en/index.ansi text/en/pillars.ansi text/en/contact.ansi   → curl mode
```

### 3.1 A URL prefix (`manifest.base`)

The site can live under a path of someone else's host, e.g. `https://dchr.koinotinav.com/mintaka/`:
`content/site.toml` `base = "/mintaka"` (or `MINTAKA_BASE` for one build; empty = the root).
`base` is empty or `/segment[/segment…]` without a trailing `/`.

- **Route paths in the manifest never carry it** (`/en/`, `/assets/x.js`). Everything the
  generator *writes as a link* does: `href`/`src`, `@font-face` URLs, `#mintaka-assets`, the web
  manifest (`start_url`, `scope`, icons), sitemap, robots, security.txt, the Atom feed, curl-mode
  text and the service worker's file list — through `SiteConfig::url`/`abs` and `Dist::link`.
- **The server strips it** before routing (`conn::respond`): `{base}/x` is routed as `/x`; the bare
  `{base}` gets `301` to `{base}/`; any path outside it gets a plain `404` (another application
  may share the host). Every `Location` a handler sends (`/en/sent/`, the slash redirect) gets the
  prefix back in `routes::finish`; 404 suggestions are written with it.
- **Islands** build URLs with `at('/path')` from `web/shared/base.ts` (`BASE` is the esbuild
  constant `MINTAKA_BASE`, §11). Beacons name their page by its URL path, prefix and all; the
  server takes the prefix off before matching it (`/_rum` counts by route path).
  The service worker registers at `{base}/sw.js` with scope `{base}/`, and treats only paths under
  the prefix as its own.

## 4. HTTP

### 4.1 Routing

| Request | Response |
|---|---|
| `GET /` | The page for the language negotiated from `Accept-Language` among `manifest.langs` (default `manifest.default_lang`), served **directly — no redirect** (a redirect costs a round trip, and this page is about round trips). Headers add `Vary: Accept-Language` and `Content-Language`. The HTML carries `<link rel="canonical" href="/{lang}/">`. |
| `GET /{lang}` | `301` → `/{lang}/` |
| Any path in `manifest.routes` | That file (see 4.3–4.4). `HEAD` works everywhere `GET` does. |
| `/_proof`, `/_telemetry`, `/_ping` | §5–§7 |
| `POST /_rum`, `GET\|HEAD /_rum` | §14 |
| `GET /api/pow`, `POST /api/contact` | §8 |
| curl / Wget / HTTPie User-Agent | §4.6 |
| Anything else | `404` with `not_found[lang]` (lang from the path prefix, else negotiated). The page contains the literal marker `<!--mintaka:suggest-->`; replace it with `<a href="{route}">{route}</a>` for the closest page route by edit distance, or with nothing. Never echo the requested path. |

A page of the A/B test (and `/` in front of it) answers each `GET` and `HEAD` with one of its
variants, drawn uniformly at random for that request (§17).

Methods other than GET/HEAD on static routes → `405` with `Allow: GET, HEAD`.
Request bodies are only accepted on `POST /api/contact` (limit 32 KiB) and `POST /_rum` (2 KiB).

### 4.2 Headers on every response

```http
Content-Security-Policy: default-src 'none'; script-src 'self' 'wasm-unsafe-eval'; style-src 'self' {style hashes}; img-src 'self'; font-src 'self'; connect-src 'self'; worker-src 'self'; manifest-src 'self'; form-action 'self'; base-uri 'none'; frame-ancestors 'none'; require-trusted-types-for 'script'; trusted-types mintaka[; upgrade-insecure-requests]
Cross-Origin-Opener-Policy: same-origin
Cross-Origin-Embedder-Policy: require-corp
Cross-Origin-Resource-Policy: same-origin
Permissions-Policy: camera=(), microphone=(), geolocation=(), payment=(), usb=(), browsing-topics=()
Referrer-Policy: no-referrer
X-Content-Type-Options: nosniff
Server-Timing: …            (4.5)
Strict-Transport-Security: max-age=63072000; includeSubDomains; preload      (TLS only)
Alt-Svc: h3=":443"; ma=86400                                                 (TLS only, with --http3)
```

- `{style hashes}` is every `manifest.csp.style_src` entry, each in single quotes. Same for
  `script_src` (empty today) appended to `script-src`.
- `upgrade-insecure-requests` and HSTS are sent **only over TLS** (on plain-HTTP localhost they
  would break development).
- `Alt-Svc` goes on every TLS response (HTTP/1.1, HTTP/2 and HTTP/3 itself) only when the server
  runs with `--http3`; the port is `--http3-advertise-port`, else the UDP port actually bound. It is
  not a security header, but `/_proof` lists it in `headers` whenever it is sent (§5).
- The full set goes on every response, not only HTML: worker scripts take their CSP from their
  own response, and COEP/COOP are harmless elsewhere.
- No `Set-Cookie`, ever. No `Server` header with versions (the software string lives in `/_proof`).

### 4.3 Cache policy (`Route.cache`)

| Policy | Cache-Control | Validator |
|---|---|---|
| `document` | `no-cache` | strong `ETag` (hash of the identity bytes), honour `If-None-Match` → `304` |
| `immutable` | `public, max-age=31536000, immutable` | ETag |
| `short` | `public, max-age=3600` | ETag |

`/_proof`, `/_telemetry`, `/_ping`, `/_rum`, `/api/*`: `Cache-Control: no-store`.

A page served as one of several A/B test variants (§17) sends `Cache-Control: private, no-cache`:
the browser may keep the variant it got and revalidate it, but no shared cache may keep one variant
and hand it to everybody. Each variant has its own ETags (per coding), and the variant is drawn
**before** `If-None-Match` is compared, so a `304` comes only when the draw lands on the variant the
browser already has — a cached copy never makes a variant stick.

`/sw.js` has a fixed, unhashed URL and the `document` policy: `no-cache` + ETag is exactly what a
service worker script needs (revalidated on every use, `304` when unchanged). The policy names
how a route is revalidated, not what it contains, so no new policy was added.

### 4.4 Compression

Pick the precompressed variant from `Accept-Encoding` (br > gzip > identity, honouring `q=0`),
send `Content-Encoding` and `Vary: Accept-Encoding` for every route that has any `encodings`.
No compression at request time.

### 4.5 Server-Timing

Durations in milliseconds with up to three decimals. Every response:

| Metric | Meaning |
|---|---|
| `app;dur=0.021` | Server time for this request: from the moment the request head was parsed to the response being ready. |
| `route;dur=0.002` | Looking up the route. |
| `render;dur=0.003` | Building the response (choosing the encoding, headers, body handle). |

When available, also:

| Metric | Meaning |
|---|---|
| `tls;dur=1.83;desc="TLS handshake"` | Server-side handshake duration. Only on the **first** request of a TLS connection (TCP or QUIC). |
| `krtt;dur=48.21;desc="kernel RTT"` | `TCP_INFO.tcpi_rtt` of this connection (Linux). Every response over TCP; never over HTTP/3. |
| `qrtt;dur=0.852;desc="QUIC RTT"` | HTTP/3 only: QUIC's own smoothed RTT for this connection (quinn's RFC 9002 estimator, from acknowledgements of our packets), once it rests on a sample. Every response over HTTP/3. A different measurement from `krtt`, so a different name. |
| `edge;desc="TLS at the hosting edge"` | No duration: the request came through the `--trusted-proxy` (§18), so the visitor's TCP and TLS ended there. Then there is never a `krtt` (it would be the proxy hop's) nor a `tls`. |

Islands that need the network round trip (S2 physics, S6 trace, the terminal) take `krtt`, else
`qrtt`, else measure it themselves from Navigation/Resource Timing.

### 4.6 curl mode

If the `User-Agent` starts with `curl/`, `Wget/`, `HTTPie/` or `xh/`, then for `/`, `/{lang}/`,
`/pillars`, `/contact` (and `/{lang}/pillars`, `/{lang}/contact`) respond
`200 text/plain; charset=utf-8` with the file from `manifest.text[lang][path]` (`/{lang}/x` uses
key `/x`; `/` and `/{lang}/` use key `/`), after replacing these tokens:

| Token | Example value |
|---|---|
| `{{server_time}}` | `21 µs` (this request so far; µs below 1 ms, else ms with 1 decimal) |
| `{{tls}}` | `TLS 1.3 · X25519MLKEM768 (post-quantum hybrid)` or `plain HTTP (development)` |
| `{{http}}` | `HTTP/3` / `HTTP/2` / `HTTP/1.1` |
| `{{build}}` | `2026-10-27.3f2a1b9c` |

`/proof` for those agents is generated by the server: the `/_proof` facts as aligned, coloured
text lines (24-bit ANSI: Starlight `38;2;244;245;247`, Mist `38;2;201;206;216`, Mintaka blue
`38;2;61;123;255`, rust `38;2;226;85;45`, success `38;2;47;191;113`).
Browsers asking for these paths get the normal 404.

## 5. `GET /_proof`

`200 application/json`, `Cache-Control: no-store`. TypeScript mirror: `web/shared/proof.ts`.

```json
{
  "server": {
    "region": "dev",
    "location": { "city": "Dushanbe", "country": "TJ", "lat": 38.5598, "lon": 68.787 },
    "build": "2026-10-27.3f2a1b9c",
    "software": "mintaka-server 0.1.0 · rustc 1.99.1 · axum 0.8 · rustls 0.23",
    "time_us": 41,
    "uptime_s": 3600,
    "cores": 16
  },
  "tls": {
    "version": "TLSv1.3",
    "cipher": "TLS13_AES_128_GCM_SHA256",
    "group": "X25519MLKEM768",
    "post_quantum": true,
    "alpn": "h2",
    "handshake_us": 1830
  },
  "http": { "version": "HTTP/2.0" },
  "connection": { "id": 812, "requests": 3, "kernel_rtt_us": 48210, "kernel_rttvar_us": 1200, "quic_rtt_us": null },
  "privacy": { "cookies_set": 0, "third_party_origins": 0, "ip_stored": false, "ip_logged": false },
  "headers": [
    "content-security-policy", "cross-origin-opener-policy", "cross-origin-embedder-policy",
    "cross-origin-resource-policy", "permissions-policy", "referrer-policy",
    "x-content-type-options", "strict-transport-security"
  ]
}
```

- `server.location` is configuration (where the machine is), `null` if not configured.
- `tls` is `null` on plain HTTP. `post_quantum` is true for any hybrid/ML-KEM group (false when
  `group` is `null`).
- `tls.cipher` and `tls.group` are `null` when the TLS stack did not report them. Over TCP rustls
  always does. Over HTTP/3 quinn 0.11 exposes only the ALPN protocol and SNI, so the server reads
  both from **its own ServerHello** for that connection (§16.2); `null` only if that failed —
  never filled in from configuration or from what the client offered.
- Over HTTP/3: `http.version` is `HTTP/3.0`, `tls.alpn` is `h3`, `tls.version` is `TLSv1.3` (QUIC
  allows nothing else), `connection.kernel_rtt_us`/`kernel_rttvar_us` are `null` (no TCP socket),
  and `connection.quic_rtt_us` is QUIC's own RTT estimate (as `qrtt`, §4.5). Over TCP
  `quic_rtt_us` is `null`.
- `connection.*` describes the connection this request arrived on. `kernel_rtt_us` is `null` when
  `TCP_INFO` is unavailable. `id` is a process-local counter, never derived from the client.
- `headers` lists only headers actually sent on this response (so no HSTS over plain HTTP), in the
  order above, then `alt-svc` when it is sent.
- `edge` is `{ "proto": "https" }` when the request came through the `--trusted-proxy` (§18):
  `proto` is the proxy's `X-Forwarded-Proto` (`https` or `http`, else `null`). Then `tls` is
  `null` (this server did no TLS), `http.version` and `connection` describe the proxy's hop, and
  `kernel_rtt_us` is `null`. Islands say "HTTPS at the hosting edge" rather than "plain HTTP" and
  never present the hop's facts as the visitor's. `edge` is `null` for direct connections.

## 6. `GET /_telemetry` (Server-Sent Events)

`text/event-stream`, `no-store`. First `event: hello` with
`{"build": "…", "cores": 16, "region": "dev"}`, then `event: tick` once per second:

```json
{ "t": 1793102400, "rps": 12, "p50_us": 21, "p99_us": 180, "p999_us": 410,
  "cpu_pct": 0.4, "rss_bytes": 14680064, "conns": 3, "req_total": 12345 }
```

- `rps`: requests completed during the last full second. Percentiles: over the last 10 s, from
  HDR histograms, `null` if no requests. Long-lived responses (`/_telemetry` itself) are excluded.
- `cpu_pct`: process CPU time over the last second ÷ wall time × 100 (can exceed 100 on many cores).
- Aggregates only: no IPs, paths, user agents. A keep-alive comment (`: ping`) every 15 s.
- At most 64 concurrent streams per process; beyond that, `503`.

## 7. `GET|HEAD /_ping`

`204 No Content`, `no-store`, with Server-Timing. Used by the terminal's `ping`.

## 8. Contact

### 8.1 `GET /api/pow`

```json
{ "challenge": "<base64url, no padding, 41 bytes>", "difficulty": 18, "expires_at": 1793103000 }
```

`challenge` bytes = `nonce[16] ‖ issued_at[8, big-endian u64 unix seconds] ‖ difficulty[1] ‖ mac[16]`,
`mac` = first 16 bytes of HMAC-SHA256(server secret, nonce ‖ issued_at ‖ difficulty). The secret
is random per process start.

**Solution:** the smallest-effort `counter: u64` such that
`SHA-256(challenge_bytes ‖ counter[8, little-endian])` has at least `difficulty` leading zero
bits. The message is 49 bytes — a single SHA-256 block — so a tight JS loop is fast; target ≈ 0.5 s
on a mid-range phone. Valid for 10 minutes, single use (the server remembers spent challenges
until they expire).

### 8.2 `POST /api/contact`

Accepts `application/json` (enhanced form) or `application/x-www-form-urlencoded` (no JS).

| Field | Rule |
|---|---|
| `kind` | Zero or more of `new-product`, `performance`, `security-audit`, `design`, `ai`, `not-sure` (form: repeated key; JSON: array) |
| `message` | Optional, ≤ 5000 characters |
| `timeline` | Optional, one of `asap`, `1-3-months`, `later`, `exploring` |
| `budget` | Optional, one of `undecided`, `lt-10k`, `10-50k`, `50-150k`, `gt-150k` (labels are page copy; ranges still to be set by Bill) |
| `name` | Required, 1–120 characters after trimming |
| `email` | Optional; if present a syntactically valid address ≤ 254 chars |
| `telegram` | Optional; if present `@?[A-Za-z][A-Za-z0-9_]{4,31}` (stored without `@`) |
| `company` | Optional, ≤ 120 characters |
| `lang` | `en` / `ru` / `tg` (defaults to the default language) |
| `website` | Honeypot. Must be empty. If filled: answer exactly like a success, store nothing. |
| `pow_challenge`, `pow_counter` | Present on the JS path. Absent on the no-JS path (stricter rate limit). |
| `ctf_pass` | Optional: the run token of a finished console challenge (§8.3), sent by the application form. A valid one stores the brief with `ctf_solved: true`; an invalid one is ignored, never refused. |

At least one of `email` / `telegram` is required. Validation lives in Rust newtypes
(`Name`, `Email`, `TelegramHandle`, …) — parse, don't validate.

**Error codes** (kebab-case, unique across fields): `name-required`, `name-too-long`,
`email-invalid`, `telegram-invalid`, `contact-required`, `message-too-long`, `company-too-long`,
`choice-invalid`, `pow-invalid`, `pow-expired`, `rate-limited`, `too-large`, `server-error`.

**Responses — JSON request:** `200 {"ok":true}` — with `"bot_link": "https://t.me/<bot>?start=<code>"`
while the server runs the Telegram bot (§19) · `422 {"ok":false,"errors":[{"field":"email","code":"email-invalid"}]}` ·
`429 {"ok":false,"errors":[{"field":"form","code":"rate-limited"}]}` · `400` for malformed bodies.

**Responses — form request:** `303 See Other` to `/{lang}/sent/` on success, or to
`/{lang}/sent/#e-{code}` for the first error (the static page shows the message for `:target`).

**Rate limits** (in memory only, keyed by IPv4 /24 or IPv6 /48 prefix, never written to disk or
logs): 10 per hour with a valid proof of work, 3 per hour without.

**Storage:** each accepted message is one file in `{data-dir}/contact/`, named
`{unix-ms}-{random}.json.age`, encrypted with `age` to the configured X25519 recipient: mode `0640`
in a `0750` directory, so a deploy account in the server's group can fetch the ciphertext. The
server holds only the public key; without a configured recipient it creates a development identity in
`{data-dir}/dev-identity.txt` and logs a loud warning. **Notification** (optional, the Telegram
bot of §19): "New brief · kinds · timeline · id" ("New application (console challenge solved) · …"
with a valid pass) in the team chat — never the message body.

### 8.3 The console challenge: `POST /api/ctf/start|puzzle|flag`

Four puzzles played from DevTools (spec §7.5), one per pillar star in page order: Rigel
(efficiency: cheapest route across a digit grid), Saiph (security: two messages under one
one-time pad; the crib is `proof::CTF_CRIB`, printed in every page's view-source greeting),
Betelgeuse (highload: what one connection's congestion window allows, from raw `struct tcp_info`
bytes), Meissa (design: Fitts's-law throughput from simulated pointing trials). The fourth answer
yields a pass for the application form.

- **Nothing is stored per player.** A run is `nonce[16] ‖ stage[1] ‖ tag[16]` (tag = first 16
  bytes of HMAC-SHA256(secret, `"mintaka-ctf/run/v1"` ‖ nonce ‖ stage)), sent as `r1.` +
  base64url. Each stage's puzzle and answer are derived from HMAC(secret, `"mintaka-ctf/puzzle/v1"`
  ‖ nonce ‖ stage ‖ attempt), so a run replays identically and the public source shows how
  puzzles are made, never an answer. The secret is `{data-dir}/ctf.key` (32 random bytes, mode
  `0600`, created on first start); deleting it invalidates every run.
- **Requests:** `start` takes no body; `puzzle` and `flag` take `{"run": "…"}` / `{"run": "…",
  "flag": "…"}` (≤ 1 KiB, `413 {"error":"too-large"}` beyond).
- **Answer** (`200`, `no-store`): `{"correct": true|false (flag only), "run": "r1.…", "stage": 1–5,
  "puzzle": {…} | null}`. A right flag returns the next stage's token; a wrong one returns the same.
  Stage 5 means solved: `puzzle` is `null` and `run` is the pass. Puzzles are tagged by `star`:
  `{"star":"rigel","grid":"…"}`, `{"star":"saiph","a":"<hex>","b":"<hex>"}`,
  `{"star":"betelgeuse","tcp_info":"<hex, 280 bytes>"}`, `{"star":"meissa","trials":[[d,w,ms],…]}`.
  The words that explain them are page copy (`ctf.ftl`). Rust types: `crates/proof/src/ctf.rs`.
- **Answers are compared by content, not form:** digit grouping and a `mintaka{…}` wrapper are
  fine; Saiph takes the last word of what is sent; Meissa compares at one decimal.
- **Errors:** `400 {"error":"bad-run"}` (forged, damaged or from another key — the client starts
  over), `400 {"error":"bad-request"}`, `405` with `Allow: POST`,
  `429 {"error":"rate-limited"}`.
- **Rate limits** per IPv4 /24 or IPv6 /48, in memory: `start` 20/h, `puzzle` 240/h, `flag` 60/h.
- **Client:** `web/console/ctf.ts`, loaded on the first `mintaka.ctf()`, `.puzzle()`, `.flag()`,
  `.hint()`, `.restart()` or `.apply()`. It keeps the run token in `localStorage["mintaka-ctf"]`
  (only once the visitor starts) and builds the application form into `#careers` (no proof of
  work: it rides the 3-per-hour limit).

## 9. The page (DOM contract)

```html
<html lang="en" data-shortcuts-off? data-variant? data-converts?>
<body>
  <a class="skip-link" href="#main">…</a>
  <header class="site-header">… <a class="btn btn--primary" href="#contact">Start a project</a></header>
  <nav class="orion-nav" aria-label="…">… real links to section ids …</nav>
  <canvas id="hero-canvas" aria-hidden="true"></canvas>
  <main id="main">
    <section id="hero">   … <a href="#physics" data-action="proof">See the proof ↓</a></section>
    <section id="physics"> <div class="instrument" data-island="physics">…</div></section>   S2
    <section id="pillars"> …</section>                                                       S3
    <section id="efficiency" data-star="rigel">…</section>                                  S4
    <section id="security" data-star="saiph"> … <div class="instrument" data-island="armour">…</div></section>
    <section id="highload" data-star="betelgeuse"> … data-island="trace" … data-island="live" …</section>
    <section id="design" data-star="meissa"> … <div class="instrument" data-island="laws">…</div></section>
    <section id="process"> … </section>   S8   ·  #qualities S9 · #proof-wall S10 · #lab S11
    <section id="team" data-star="bellatrix"> <ul class="team-grid">…</ul> …
             <div id="careers">…</div></section>                                            S12
    <section id="contact"> <form id="contact-form" class="contact" data-island="contact"
             method="post" action="/api/contact"> … </form></section>                       S13
  </main>
  <footer class="site-footer"> … <p id="footer-stats" aria-live="off">…</p>
    <button data-action="terminal">…</button> <button data-action="shortcuts" aria-pressed="true">…</button>
  </footer>
  <script type="application/json" id="mintaka-i18n">{ "hud-server-time": "Server time", … }</script>
  <script type="application/json" id="mintaka-assets">{ "pow.worker": "/assets/pow.worker.AB12CD34.js" }</script>
  <script type="module" src="/assets/main.3F2A1B9C.js"></script>
</body>
```

- **Instrument containers** use the frame from `web/styles/components.css`
  (`.instrument` › `.instrument__bar` / `.instrument__body` / `.instrument__foot`). The page renders
  the frame and a static fallback inside `.instrument__body`; the island replaces the body's
  children (`render(body, …)`) and may update `.instrument__status`. Never remove the frame. The
  frame carries `tabindex="0"` until its island has mounted and focus has left it (`web/main.ts`),
  so Tab can reach an instrument before it has scrolled near (docs/ACCESSIBILITY.md).
- **The contact form** is complete, working HTML without JS (labels, `name`s matching §8.2,
  `required`, `type="email"`, honeypot field `website` inside a visually hidden wrapper with
  `tabindex="-1"` and `autocomplete="off"`). The island enhances it; it never replaces it.
- `data-action="proof" | "terminal" | "shortcuts"` clicks are handled in `web/main.ts`.
- **A page rendered as an A/B test variant** (§17) says so on its root element:
  `<html lang="en" dir="ltr" data-variant="hero-title-hardware" data-converts="cta_start_project contact_submit">`
  — the variant's id and the event keys its beacons report as conversions. Read by `web/rum/` and
  the proof console; pages outside the test have neither attribute.
- **The team grid** comes from `[[team]]` in `content/site.toml` (id, GitHub name, site); name,
  role, line and an optional alt text are strings (`team-<id>-name|role|line|photo`), the
  portrait is `assets/team/<id>.avif` + `.webp` (square, 224 px). `#careers` is where the console
  challenge's application form appears (§8.3).

### 9.1 Contact form markup (main writes it, worker C enhances it)

```html
<form id="contact-form" class="contact" data-island="contact" method="post" action="/api/contact" novalidate?>
  <input type="hidden" name="lang" value="en">
  <fieldset class="contact__group" data-field="kind">
    <legend class="field__label">What are you building?</legend>
    <label class="chip"><input class="visually-hidden" type="checkbox" name="kind" value="new-product"> New product</label>
    … performance · security-audit · design · ai · not-sure
  </fieldset>
  <div class="field" data-field="message">
    <label class="field__label" for="f-message">Tell us more</label>
    <p class="field__hint" id="h-message">…</p>
    <textarea class="input" id="f-message" name="message" maxlength="5000" aria-describedby="h-message e-message"></textarea>
    <p class="field__error" id="e-message" hidden></p>
  </div>
  <fieldset class="contact__group" data-field="timeline"> … radio chips name="timeline" … </fieldset>
  <fieldset class="contact__group" data-field="budget"> … radio chips name="budget" … </fieldset>
  <div class="field" data-field="name">     … <input class="input" id="f-name" name="name" required maxlength="120" autocomplete="name"> … <p class="field__error" id="e-name" hidden></p></div>
  <div class="field" data-field="email">    … <input class="input" id="f-email" name="email" type="email" autocomplete="email"> … id="e-email"</div>
  <div class="field" data-field="telegram"> … <input class="input" id="f-telegram" name="telegram" autocomplete="off" spellcheck="false"> … id="e-telegram"</div>
  <div class="field" data-field="company">  … <input class="input" id="f-company" name="company" maxlength="120" autocomplete="organization"> … id="e-company"</div>
  <p class="field__error" id="e-contact" hidden></p>          <!-- "Add an email or a Telegram handle" -->
  <div class="contact__trap" aria-hidden="true"><label>Website <input name="website" tabindex="-1" autocomplete="off"></label></div>
  <p class="contact__status" id="contact-status" role="status" aria-live="polite"></p>
  <button class="btn btn--primary" type="submit">Send</button>
</form>
```

Error codes map to the `e-*` element by their prefix (`email-invalid` → `#e-email`,
`contact-required` → `#e-contact`, `rate-limited`/`pow-*`/`server-error`/`too-large` → the status
line). Messages come from `contact.ftl` (`contact-error-email-invalid`, …), not from the server.

### 9.2 Lab pages (spec §S11)

`/{lang}/lab/` and `/{lang}/lab/{slug}/` share the header and footer but ship **no JavaScript**:
no module script, no `#mintaka-i18n` / `#mintaka-assets`, no `data-action` buttons in the footer,
and the footer's live cell says why. The build fails on any `<script>` in a Lab page. They inline
`web/styles/lab.css` (tokens, base, components, `sections/layout.css`, `sections/lab.css`) instead
of `page.css`; its hash is a second entry in `manifest.csp.style_src`. In `pages/mod.rs` this is
`Page::long_form` (`LongForm`); every other page passes `None`.

- Posts are `content/{lang}/lab/*.md` with TOML front matter; the format, the Markdown extras
  (`include=` code excerpts, `repo:` links, MathML, table captions) and the checks are documented
  in `crates/site/src/lab.rs`. A post's slug is the same in every language that has it.
- `/{lang}/lab/feed.xml` (Atom) is linked with `<link rel="alternate" type="application/atom+xml">`
  from every page of a language that has posts; its dates and ids depend only on the posts and
  `origin`, never on the build time.
- Budgets: per Lab page, first screen (up to the post body) ≤ 14 kB and whole page ≤ 32 kB brotli;
  the feed ≤ 64 kB brotli.

## 10. Islands

- A module per island exports `mount(root: HTMLElement): void` (instruments, contact) or the
  functions `web/main.ts` calls (`hud`: `mount()`, `open()`; `terminal`: `open()`;
  `console`: `greet()`; `hero`: `start(canvas)`; `nav`: `start()`). `open()` must be idempotent
  and must work before or after `mount()`.
- `web/main.ts` loads instrument islands when their container comes within one screen of the
  viewport, or as soon as keyboard focus enters it; HUD/console/nav, the real-user collector (`web/rum/`, §14) and offline mode
  (`web/offline/`, §15) on idle; the terminal on first use.
- **Offline.** Anything that asks our server checks `offline()` (`web/shared/env.ts`) when it
  fails and then says so in words — "You're offline, …" — instead of showing the browser's error,
  and comes back by itself or on the next try once `online` fires (`whenOnline()`). A page that
  came from the service worker's copy is `offlineCopy()` (`web/shared/timing.ts`): its Navigation
  Timing and Server-Timing describe the cache, not our server, so nothing may present them as a
  measurement of this view. An island whose code or WebAssembly was never stored shows
  `unavailable()` from `web/offline/index.ts` and loads once the connection is back.
- Workers and data files: build them as their own esbuild entries; get the hashed URL with
  `assetUrl('pow.worker')` and start them with `worker()` from `web/shared/tt.ts`.
- Cross-island signals go through `web/shared/bus.ts` (`tier`, `sky`, `proof:open`, …). Add an
  event there if you need one (and report it).

### 10.1 Rules for every island

1. **Trusted Types and CSP are enforced.** No `innerHTML`/`outerHTML`/`insertAdjacentHTML`/
   `document.write`/`eval`/`new Function`/string timers. Build DOM with `h()`/`s()`
   (`web/shared/dom.ts`). No inline `style` attributes; dynamic values via
   `el.style.setProperty('--x', …)` and CSS custom properties.
2. **Zero third-party code at runtime.** No npm runtime dependencies. Dev dependencies only.
3. **Mono means measured.** Every number shown in `.num` (IBM Plex Mono) is measured live on this
   page view or cited with its source on screen. No example, invented or "typical" numbers. If a
   measurement is unavailable, say so plainly ("not available in this browser").
4. **Honest labels.** Simulations say "simulation" on screen. Development mode (plain HTTP, no
   TLS) is described as such, not hidden.
5. **Accessibility (WCAG 2.2 AA).** Everything works with the keyboard; visible focus (the global
   `:focus-visible` ring); results announced through `aria-live="polite"` regions; canvases are
   `aria-hidden` and mirrored as text or a table; touch targets ≥ 44 px; colour is never the only
   signal; honour `prefers-reduced-motion` (`reducedMotion()` in `web/shared/env.ts`).
6. **Performance.** Do nothing until mounted; pause animation when off screen
   (IntersectionObserver) or when the tab is hidden; no layout thrash in rAF loops; no polling.
   Budgets (minified, before compression): HUD ≤ 6.2 kB, terminal ≤ 8 kB, each instrument ≤ 15 kB
   (data tables excluded, loaded lazily).
7. **Strings** come from `content/en/islands/<island>.ftl` and are read with `t('<island>-key')`.
   Message ids are prefixed with the island name. Variables (`{ $ms }`) and at most **one selector
   per message** on a variable, for plurals: `x-requests = { $n -> [one] { $n } request *[other] { $n } requests }`.
   The generator exports each variant as `id[key]` (plus the default as `id`); read counts with
   `tp('x-requests', n)`, which picks by `Intl.PluralRules` for the page language.
   English is the source; Russian and Tajik were drafted by Claude at the founder's request and
   are reviewed by native speakers (docs/TRANSLATION.md). Long text read only after an explicit action (the console challenge) goes in
   `content/{lang}/islands/lazy/<name>.ftl`: it is not inlined but exported as
   `/assets/strings/<name>.<lang>.<hash>.json` (named `strings/<name>.<lang>`) and fetched with
   `await loadStrings('<name>')`. Keep the line that reports a failed fetch inline.
8. **Copy voice.** Plain verbs, sentence case, specific, no hype, no exclamation marks. Name things
   by what the visitor sees and controls. Errors say what happened and what to do.
9. **Styles** go in `web/styles/islands/<island>.css`, inside `@layer islands { … }`, with
   prefixed class names. They are bundled into one `islands.css`, fetched by `web/main.ts` in
   parallel with the first island's code — so anything that must look right **without JavaScript**
   (the contact form, instrument frames, fallbacks) belongs in the page CSS
   (`web/styles/page.css` and its imports), which is inlined into every page. Use tokens (`var(--surface)`, `var(--s-4)`, `var(--font-mono)`) — no raw
   hex values except inside canvas drawing code, where you read tokens with
   `getComputedStyle(document.documentElement).getPropertyValue('--mintaka')`.
10. **Colour rarity.** Rust (`--betelgeuse`) appears only on the primary call to action and the
    Betelgeuse star. Blue (`--mintaka`) marks links, focus, live/measured state. Danger red only for
    real errors. Everything else is Starlight / Mist / Steel / Graphite on Void and Ink.

### 10.2 Counting events: `track()`

`web/shared/rum.ts` exports one function for the event counts of spec §12:

```ts
import { track } from '../shared/rum.ts';
track('proof_open');                       // events without a value
track('ctf_stage', 2);                     // 1 | 2 | 3 | 4
track('instrument_start', 'security');     // 'physics' | 'efficiency' | 'security' | 'highload' | 'design'
```

- Typed by `RumEvents`: a name without a value takes no second argument; one with a value
  requires it, from its closed set. Every key it can produce is in `EVENTS`
  (`crates/proof/src/rum.rs`); add a new event to both, and to `EVENT_KEYS` in `web/rum/vitals.ts`.
- It only adds one to an in-memory counter (at most 20 per key per beacon). It never sends,
  throws or waits, so it is safe at any time: before the collector has loaded, after the page's
  beacon has gone (the next hide sends the new counts), or when nothing will ever be sent
  (Global Privacy Control, Do Not Track).
- Count what the visitor did, once: `instrument_start` when they start a run, `instrument_complete`
  when that run shows its result; never on load, scroll or automatic refreshes. The value says
  which pillar, nothing about what was measured (S7's measurements never leave the browser).
- Already wired: `cta_start_project` and `lang_switch` (clicks, `web/main.ts`), `proof_open`
  (each opening of the panel), `terminal_open` (each opening), `contact_submit` (the bus event
  `contact:sent`, i.e. accepted by the server), `render_tier` (the bus `tier` at the first beacon,
  sent by the collector itself), and `instrument_start`/`instrument_complete` for S2 physics
  (measure), S4 routing (a run the visitor asked for), S5 fuzzer (a chapter), S6 follow one
  request (send another) and S7 laws (an experiment). The armour scan, live load and the overload
  lab run by themselves and are not counted. `ctf_stage:n` counts stage n of the console challenge solved (`web/console/ctf.ts`, §8.3).

## 11. Build-time flags

`MINTAKA_BASE` (esbuild `--define`) is the site's URL prefix as a string literal (`"/mintaka"`
or `""`, §3.1); read it only through `web/shared/base.ts`, which falls back to `""` in unit tests.

`MINTAKA_DEV` (esbuild `--define`) is `true` in `cargo xtask build`/`dev` and `false` with
`--release`, where dead code behind it is removed. Development-only hooks live behind it — e.g.
`window.__hero.advance(seconds)` runs the hero simulation synchronously for reviews in throttled
or automated browsers.

## 12. Fonts

`--font-sans` = Space Grotesk (variable, Latin) → Onest (variable, Cyrillic incl. Tajik letters) →
metric-matched fallback. `--font-mono` = IBM Plex Mono 400/500 (Latin + Cyrillic). Space Grotesk's
Latin subset lacks `→ ≤ ≈`; those fall through to Onest's math subset (loaded only when such a
glyph is on screen) — avoid them above the fold. That subset adds arrows and `− ≈ ≠ ≤ ≥`, nothing
else (its cmap, checked 2026-10-29): **no Greek letters** (`λ`, `μ`), no `∑ ∈ ∞ ⋅` and no subscript
digits (`₂`). Those render in a system font, and the release build's glyph check fails on them in
generated pages and FTL strings (not in TypeScript literals). Lab formulas (MathML) use the same
fonts, so they keep to that set.

## 13. WebAssembly instruments

Three instruments are Rust compiled to WebAssembly: S4 `routing`, S5 `fuzzer` and S6 `loadsim`. Each is a crate under `crates/<name>`:

```toml
[package]
name = "loadsim"           # = the island name = the asset name
version.workspace = true
edition.workspace = true
publish.workspace = true

[lib]
crate-type = ["cdylib", "rlib"]   # cdylib for wasm32, rlib so `cargo test -p loadsim` runs natively

[package.metadata.mintaka]
wasm = true                # tells `cargo xtask build` to compile it for wasm32

[dependencies]
wasm-bindgen = "=0.2.128"  # MUST equal the installed wasm-bindgen CLI (Arch package 0.2.128)
```

- **Pipeline** (`cargo xtask build`): one `cargo build --target wasm32-unknown-unknown --profile wasm`
  for all such crates → `wasm-bindgen --target web` writes `web/wasm/<name>/<name>.js` (+ `.d.ts`)
  → `wasm-opt -O3`. The generator ships `<name>_bg.wasm` as a hashed asset named `<name>.wasm` in
  `#mintaka-assets`. `web/wasm/` is generated and gitignored — run `cargo xtask build` once before
  `npx tsc` so the glue and its types exist.
- **Loading** from the island (`web/instruments/<name>/index.ts`):
  `import init, { thing } from '../../wasm/<name>/<name>.js';`
  `await init({ module_or_path: assetUrl('<name>.wasm') });` — never rely on `import.meta.url`.
  Load on mount (the island itself is already lazy). If `WebAssembly` is missing or init fails,
  say so in the instrument and keep the static explanation.
- **Rust rules:** pure logic in plain modules, unit-tested natively; the `#[wasm_bindgen]` surface in
  a small `wasm.rs` behind `#[cfg(target_arch = "wasm32")]`. No `std::time` (it panics on
  wasm32-unknown-unknown): time in JS with `performance.now()` (5 µs resolution here — the page is
  cross-origin isolated). No OS randomness: take seeds from JS. Keep dependencies minimal — prefer
  returning typed arrays or small hand-written JSON over pulling in serde for the wasm build.
  Strings crossing into JS: offsets in **UTF-16 code units**, not bytes.
- **Without bindgen:** a crate whose metadata also says `bindgen = false` (S5 `fuzzer`) exports plain
  `extern "C"` functions; xtask copies the module to `web/wasm/<name>/<name>_bg.wasm` and runs
  wasm-opt, and the island compiles it itself (`WebAssembly.compileStreaming(fetch(assetUrl(…)))`)
  and reads results straight from linear memory. The fuzzer needs this: a parser panic must abort
  (`panic = "abort"`, inherited from the release profile) so the host catches the trap as a
  `WebAssembly.RuntimeError` and reads the panic message the hook saved in atomics. Its TS tests
  load the built module from `web/wasm/fuzzer/`, so run `cargo xtask build` before `node --test`.
- **Budgets:** each `.wasm` ≤ 250 kB (checked by the build), each island ≤ 15 kB minified — except
  `fuzzer` at 21.5 kB (three chapters, the coverage sky, a crash minimiser, a hand-input panel and
  the code that fetches its run-time text, a lazy string bundle; raised from 21 kB on 2026-10-30).
- CSP already allows WebAssembly (`'wasm-unsafe-eval'`); nothing else changes.

## 14. Real-user measurements: `/_rum`

Spec §12: self-hosted, cookie-less, aggregate-only. Rust types: `crates/proof/src/rum.rs`;
TypeScript mirror: `web/shared/rum.ts`.

### 14.1 `POST /_rum` (the beacon)

`web/rum/index.ts` (loaded on idle) measures the page view with PerformanceObserver and
Navigation Timing, no library, and sends `navigator.sendBeacon('/_rum', JSON.stringify(beacon))`
(so `text/plain;charset=UTF-8`) the first time the page is hidden (`visibilitychange` → hidden,
or `pagehide`). That first beacon is the page view (`view: true`) and the only one with metrics;
if the visitor comes back and does more, each later hide sends only the counts since the last.

```json
{ "route": "/en/", "view": true,
  "metrics": { "ttfb": 92.4, "fcp": 640.2, "lcp": 812.6, "cls": 0.0012, "inp": 48 },
  "events": { "proof_open": 1, "instrument_start:highload": 2, "render_tier:B": 1 } }
{ "route": "/en/", "view": true, "variant": "hero-title-hardware",
  "events": { "cta_start_project": 1, "render_tier:B": 1 }, "converted": ["cta_start_project"] }
```

| Field | Rule |
|---|---|
| `route` | The page's canonical path, which is a manifest page route (`/en/`, `/en/privacy/`, `/en/sent/`), or `/`, or `404` for the not-found page. Anything else → `400`. Never `location.pathname`. |
| `view` | `true` on the first beacon of a page view. |
| `metrics` | Optional each, only with `view: true`. Milliseconds from navigation start minus `activationStart`, 0–60 000: `ttfb` (`responseStart`), `fcp` and `lcp` (only while the page was visible from the start), `inp` (the slowest interaction, ignoring one per 50; absent without interactions). `cls`: largest session window, 0–10. Times rounded to 0.1 ms, CLS to 0.0001. |
| `events` | `EVENTS` key → count 1–20. Keys: `cta_start_project`, `proof_open`, `instrument_start:{physics,efficiency,security,highload,design}`, `instrument_complete:{…}`, `terminal_open`, `ctf_stage:{1,2,3,4}`, `contact_submit`, `lang_switch`, `render_tier:{A,B,C,D}` (§10.2). |
| `variant` | Only while an A/B test runs (§17), only for a page of the test, and only from a page view our server sent (`data-variant`; never from an offline copy): the variant's id, on every beacon of that page view. The one with `view: true` is the exposure. An id the test doesn't have, or a page outside it → `400`. |
| `converted` | The test's metrics (`data-converts`) this page view reached for the first time: each at most once per page view, each also counted in `events` of the same beacon, and only with `variant`. Anything else → `400`. |

Unknown fields anywhere → `400`. A beacon with neither `view` nor an event → `400`.

Not sent at all from a page that is an offline copy (§15) — its timings are the cache's — except
for event counts; and nothing at all when the browser sends Global Privacy Control
(`navigator.globalPrivacyControl`) or Do Not Track (`navigator.doNotTrack === '1'`): the collector
doesn't start. The server also drops any beacon that arrives with `Sec-GPC: 1` or `DNT: 1`.

**Answers** (all `Cache-Control: no-store`, bodies fixed text, nothing echoed), in check order:

| Condition | Status |
|---|---|
| `Sec-GPC: 1` or `DNT: 1` | `204`, nothing counted |
| `Sec-Fetch-Site` present and not `same-origin` | `403` |
| Content type other than `text/plain` / `application/json` | `415` |
| Body over 2 KiB (declared or streamed) | `413` |
| Over the rate limit | `429`, `Retry-After: 60` |
| Not the shape above | `400` |
| Accepted | `204` |
| Other methods | `405`, `Allow: GET, HEAD, POST` |

**Rate limit:** per IPv4 /24 or IPv6 /48, a burst of 60 beacons refilled at 60 per hour, in memory
only (the same network key as §8.2; the address itself is never kept).

**Aggregation:** in memory only. Per window: page views, views per route, one HDR histogram per
metric (two significant figures: exact below 25.6 ms / CLS 0.0256, within 1 % above), and a
counter per event key. Nothing per beacon is kept: no address, user agent, cookie or timestamp;
nothing is written to disk or logged beyond the access log's `POST /_rum 204 …` line.

**Windows:** *since the process started* — nothing survives a restart, so this is the honest outer
bound, and since the server restarts with every deploy it also means "this build" — and *the last
24 hours* in whole hours (the current hour and the 23 before it), so a long-running process
doesn't average over weeks of old traffic. 24 hourly slots, allocated the first time an hour has
traffic and rotated lazily (no timer).

### 14.2 `GET|HEAD /_rum` (the aggregates)

`200 application/json`, `no-store`. Recomputed only when a beacon arrived or the hour changed.

```json
{
  "since": 1793251200, "now": 1793254800, "min_n": 20,
  "start": {
    "from": 1793251200, "views": 412,
    "routes": { "/en/": 398, "/en/privacy/": 12, "404": 2 },
    "metrics": { "lcp": { "n": 398, "p75": 812.7 }, "inp": { "n": 7, "p75": null }, "cls": { "n": 400, "p75": 0.0012 },
                 "fcp": { "n": 401, "p75": 640.4 }, "ttfb": { "n": 410, "p75": 92.6 } },
    "events": { "proof_open": 31, "instrument_start:highload": 12, "render_tier:B": 280 }
  },
  "day": { "from": 1793254800, "…": "same shape" },
  "rejected": { "invalid": 3, "rate_limited": 0 },
  "experiment": {
    "id": "hero-headline", "metric": "cta_start_project", "secondary": ["contact_submit"],
    "plan": { "baseline": 0.05, "mde": 0.3, "alpha": 0.05, "power": 0.8, "sample_size": 4578 },
    "variants": [
      { "id": "hero-title", "views": 1204, "conversions": 61, "secondary": { "contact_submit": 3 } },
      { "id": "hero-title-hardware", "views": 1185, "conversions": 70, "secondary": { "contact_submit": 2 } },
      { "id": "hero-title-cycle", "views": 1210, "conversions": 52, "secondary": { "contact_submit": 1 } }
    ],
    "result": null
  }
}
```

- `since`, `now`, `from`: Unix seconds. `day.from` is never before `since`.
- `p75`: the 75th percentile, rounded **up** to the histogram's resolution (never better than
  measured); `null` when `n < min_n` (20). The proof console then says how many samples there are
  and how many it needs, in words — never a number for the percentile.
- `routes` and `events` list only keys with at least one count.
- The proof console (`web/hud/visitors.ts`) shows the `start` window: "Since {date}: N page views
  measured", then LCP, INP, CLS, FCP, TTFB.
- `experiment`: the A/B test of this build since the process started (no 24-hour window: a test is
  judged on all its data), `null` when none runs. `variants` control first; `result` stays `null`
  until every variant has `plan.sample_size` page views (§17). A beacon's page view and events are
  counted in `start` and `day` as well, whatever the test does with them.

## 15. Offline mode: `/sw.js`

Spec §7.9. `web/sw/sw.ts` (pure decisions in `web/sw/lib.ts`, tested in Node) is bundled by
xtask as a classic script, `target/web/sw.js`; the generator (`crates/site/src/sw.rs`) replaces
the one `self.__MINTAKA_SHELL__` in it with this build's file list and serves it at `/sw.js`
(policy `document`, §4.3). `web/offline/index.ts`, loaded on idle, registers it with
`navigator.serviceWorker.register(scriptURL('/sw.js'), { scope: '/', updateViaCache: 'none' })`;
the `mintaka` Trusted Types policy (`web/shared/tt.ts`) approves `/assets/*` and exactly `/sw.js`.

**The file list** (`Shell`): `build`; `pages` (path → language and the hashes the server's ETags
are made of: one per A/B test variant, control first, any of which the server may send — §17); `core` (main.js, islands.css and the hero core, each with its static imports from
esbuild's metafile); `fonts` (language → the fonts its pages preload); `renderers` (render tier
A/B/C → its chunk with imports; D needs none); `assets` (every `/assets/` file of the build — the
only assets ever stored). Any change to a shipped file changes the worker's bytes, so browsers
install the new worker.

**Caches:** one, named `mintaka-{build}`. On activate the worker deletes every other `mintaka-*`
cache, prunes entries of its own that aren't in the file list, and claims open pages.

**Precache:** on install, the core plus every page open right now (the registering one included)
and every page an earlier build had stored, each with its language's fonts; `/` is stored with
the fonts of the language it was served in (`Content-Language`). After that, each page view posts
`{ type: "warm", page, lang, tier, used }` (on `ready`, on every `tier` event, on
`controllerchange`): the worker adds the page, its fonts, the renderer of its tier and the
`/assets/` files it used, skipping what it already has and pages whose stored ETag is current —
for a page of the A/B test, that of any of its variants. The worker keeps one copy per page: the
variant its own request drew, which an offline view then shows (and doesn't count, §17).

**Strategies:**

| Request | Worker |
|---|---|
| Not `GET`, another origin, `/_*` (`/_proof`, `/_telemetry`, `/_ping`, `/_rum`), `/api/*`, `/sw.js` | Not handled: never stored, never answered from a cache |
| `/assets/*` | The stored copy, else the network (stored on the way if it is this build's) |
| A navigation while the browser is online | Not handled: the network, untouched |
| A navigation while the browser reports no connection | The stored page (query ignored), else the network's own error |
| Anything else (icons, robots.txt, …) | Not handled |

Pages are network-first in the strictest sense: online, the worker doesn't touch them, because
answering a navigation from a worker replaces its Navigation Timing (DNS, TCP and TLS read as zero,
"connection reused") and S2, S6 and `/_rum` read exactly those numbers. The stored copy of each
page is kept fresh by the warm message instead (a conditional request; `304` when current). The
cost: a connection that reports itself online but carries nothing gets the browser's error page,
not the copy.

**Stored copies** keep the server's headers — CSP, COOP, COEP, CORP, Permissions-Policy,
Referrer-Policy, X-Content-Type-Options, ETag, Content-Type — so an offline page is exactly as
isolated as an online one. Three are changed: `Content-Length` and `Content-Encoding` are dropped
(the cache keeps the decoded body), and `Server-Timing` becomes `sw;desc="offline copy"` (the
server's timings described another request; `offlineCopy()` in `web/shared/timing.ts` reads it).

**Offline states** (§10): proof console rows that ask the server (`/_proof`, the headers' `HEAD`,
`/_rum`) and the server-time row of an offline copy; the terminal's `ping` and `proof`; S2 physics;
S5 armour (scans once back online); S6 follow one request (and an offline copy reads as "from
your browser's cache"); S6 live load (its own `offline` state); S13 contact (says it can't send,
keeps the draft — in `sessionStorage` once Send was pressed offline, restored on reload, removed on
success — and says when the connection is back); S4, S5 fuzzer and S6 overload lab when their
WebAssembly or data was never stored; any island whose code was never stored.

## 16. TLS, HTTP/3 and certificates (server)

### 16.1 The TLS listener (TCP)

- rustls with aws-lc-rs, offering exactly X25519MLKEM768, X25519, secp256r1 (`tls::provider()`);
  TLS 1.3 only unless `--tls12`; ALPN `h2` then `http/1.1`; no 0-RTT; no client certificates.
- The server reads the complete ClientHello before choosing a configuration. A ClientHello that
  offers **only** `acme-tls/1` is an ACME TLS-ALPN-01 validation (RFC 8737): it gets the challenge
  certificate for its SNI name (or no certificate, if no validation is open for that name), and
  the connection is closed after the handshake — no HTTP is ever spoken on it. Any other offer,
  `acme-tls/1` among others included, is an ordinary connection.

### 16.2 HTTP/3 (`--http3`)

- QUIC (quinn 0.11) on UDP at the TLS listener's address and port, TLS 1.3 only, ALPN `h3`, the
  same certificate resolver as TCP (an ACME renewal reaches both at once), no 0-RTT.
- Every request goes through the same axum router and `routes::finish` as HTTP/1.1 and HTTP/2:
  same headers, `Server-Timing`, metrics, access log. Request bodies stream into handlers (the
  32 KiB contact limit holds, with or without `Content-Length`); response bodies stream out frame
  by frame, `/_telemetry` included.
- Limits: handshake within 10 s, first request within 10 s, each request's HEADERS within 10 s
  (as on TCP); 256 concurrent requests per connection (HTTP/2's limit); 16 client
  unidirectional streams; 30 s idle timeout; 64 KiB stream / 1 MiB connection receive windows;
  request heads up to 64 KiB (then `431`).
- What `/_proof` can know about a QUIC handshake: quinn's `HandshakeData` carries only the ALPN
  protocol and the SNI name; the negotiated cipher suite is not exposed, and the key-exchange
  group only behind quinn-proto's private `__rustls-post-quantum-test` feature. The server wraps
  quinn's rustls session (`quic::handshake`), delegating every call, and reads the cipher suite
  and the `key_share` group from the ServerHello bytes it wrote in the Initial packet space
  (skipping a HelloRetryRequest); `handshake_us` is complete ClientHello → client Finished, as on
  TCP.
- `Alt-Svc` (§4.2) announces it. `--http3-advertise-port` sets the announced port when a
  container or NAT maps the public port 443 onto another one.
- Shutdown: new QUIC connections are refused; each connection gets a GOAWAY, its accepted
  requests finish (event streams end), then the server closes it; whatever is left when the grace
  period ends is closed with it.

### 16.3 Certificates from Let's Encrypt (`--acme-domain`)

| Flag | Meaning |
|---|---|
| `--acme-domain NAME` | Repeatable. Lower-case DNS names (A-labels), no wildcards (TLS-ALPN-01 cannot validate them). Excludes `--tls-cert`/`--tls-key`/`--tls-self-signed`. |
| `--acme-contact mailto:…` | Optional account contact. |
| `--acme-directory` | `production` (default), `staging`, or an `https://` directory URL. |
| `--acme-ca-root PEM` | Trust this CA for the directory's HTTPS instead of the system roots (Pebble, a private CA). |

- **Validation:** TLS-ALPN-01 only, on the TLS listener (§16.1). The CA connects to port **443**,
  so public 443/tcp must reach the listener. Nothing on port 80 is involved.
- **Cache** `{data-dir}/acme/` (mode 0700): `account-<hash>.json` (account URL + private key, per
  directory URL) and `cert-<hash>.pem` (the CA's chain, leaf first, then our ECDSA P-256 key; per
  directory URL and set of names). Files are mode 0600, written to a temporary file, fsynced and
  renamed; a cached file found readable by others is set back to 0600. Staging and production,
  or different name sets, never share a file.
- **Startup:** a cached certificate that covers every name and has not expired is served from
  the first handshake; otherwise one is ordered at once, and until it arrives ordinary TLS
  handshakes fail (validations still work).
- **Renewal:** 30 days before expiry, or a third of the lifetime before it for certificates
  shorter than 90 days; the new certificate replaces the old one for every new connection, TCP
  and QUIC, without a restart. After a failure: retry in 5 min, doubling, at most hourly while
  nothing usable is served or expiry is within a week, else at most every 6 h. One order may take
  10 min at most.
- **Logs** (target `acme`): the CA, the names, dates, the account URL, and the CA's problem
  documents. Never keys, tokens or key authorizations.

## 17. A/B tests

Spec §12: A/B tests on the landing page itself — hero headline, button wording, order of the
pillars — with variants assigned per page load on the server (no cookie), one test at a time, and
a sample size decided in advance. Shared types and formulas: `crates/proof/src/experiment.rs`
(plan, statistics) and `crates/proof/src/rum.rs` (beacon fields, `/_rum` shape).

### 17.1 Declaring a test

`[experiment]` in `content/site.toml` (published, like every value there):

| Key | Meaning |
|---|---|
| `id` | Names the test (`hero-headline`): lower-case letters, digits, `-`. |
| `control` | The page string that varies (`hero-title`); the control shows it as it is. |
| `variants` | The strings shown in its place by the others (`hero-title-hardware`, `hero-title-cycle`). Every enabled language needs them; ids also name files. |
| `metric` | The primary metric, an `EVENTS` key: a page view **converts** when it happens at least once (`cta_start_project`: a link to `#contact` was followed). |
| `secondary` | Event keys counted per variant the same way and **never tested** (`contact_submit`, a guardrail: a headline that wins clicks but loses briefs is no win). The sample size is planned for `metric` alone; at a brief's rate a test of `contact_submit` would be badly underpowered, and every extra test raises the chance of a false positive. |
| `baseline`, `mde`, `alpha`, `power` | The planning assumptions: the control's expected conversion rate per page view, the smallest relative lift worth finding (0.3 = +30 % of the baseline), the family-wise false-positive rate (two-sided) and the chance of finding a true lift of `mde`. |
| `sample_size` | Page views per variant — the stopping point. The build computes it from the four above and **fails if this number disagrees**, so it is the one written down before the test began. |

**The plan:** each variant is compared with the control by a two-sided two-proportion z-test with
pooled variance; `alpha` is split evenly over the k − 1 comparisons (Bonferroni). Per variant,
n = ⌈(z₁₋ₐ⁄₂·√(2p̄(1−p̄)) + z_power·√(p₁(1−p₁) + p₂(1−p₂)))² ⁄ (p₂ − p₁)²⌉ with p₁ = `baseline`,
p₂ = `baseline`·(1 + `mde`), p̄ = (p₁ + p₂)/2 and a = `alpha`/(k − 1) — the equation R's
`power.prop.test` solves (tested against its documented `p1 = .50, p2 = .75, power = .90` → 76.7).
The first test: baseline 5 %, lift to 6.5 %, alpha 0.05 (0.025 per comparison), power 0.8 →
**4,578 page views per variant**, 13,734 in all. The baseline is a placeholder until two weeks of
`/_rum` give a real one (spec §2.3); changing it means a new `sample_size`, which the build asks for.

To end a test, remove the table (adopting a winner means changing the control's text); a new test
gets a new `id`.

### 17.2 Building it

The home page of every enabled language is rendered once per variant: `Strings::as_variant(control,
variant, …)` makes every read of the control's message read the variant's instead. The control is
the route's own file (`{lang}/index.html`), the others sit beside it (`{lang}/index.{variant}.html`),
listed in the route's `variants` (§3) and precompressed one by one. Variant pages carry
`<html data-variant data-converts>` (§9) and are otherwise identical: same stylesheet, so the CSP's
hashes cover them (the build checks every inline `<style>` of every page and variant against the
CSP), and each is budgeted on its own (first screen ≤ 14 kB, whole page ≤ 32 kB; the first view
counts the largest). The privacy page gains `pv-ab` while a test runs. curl mode has no variants.

### 17.3 Serving it

For a page of the test — and `/` in front of it — the server draws one variant per request,
uniformly at random from the thread's generator (`assets::Arms`), for `GET` and `HEAD`, and only then
compares `If-None-Match` with that variant's ETag (§4.3). Nothing about the request or the visitor
goes into the draw, and nothing about it is kept or logged: the access log names the route, never
the variant. A returning visitor may get another variant; that is acceptable for copy tests.
Responses are `Cache-Control: private, no-cache`.

### 17.4 Counting it

- **Exposure:** the beacon with `view: true` of a page view our server sent with a variant (§14.1:
  every beacon of that page view names it; an offline copy was never drawn, so it names none).
- **Conversion:** `converted` — the first time a page view reaches the test's metric, or one of its
  secondary ones; the collector reports each once per page view, in the first beacon that counts
  it (the page view's first, or a later one if the visitor came back and then did it).
- **Per variant**, in memory, since the process started: page views, conversions, and the same per
  secondary metric. Each variant stops counting once it has `sample_size` page views; a conversion
  reported after that isn't counted either (it can't be told from one of a page view after the
  stop). Beacons are still counted in the `/_rum` totals as usual.
- **Restarts reset it** like the rest of `/_rum`: a deploy during a test starts its count over.
- Global Privacy Control and Do Not Track: nothing is sent (§14.1), so such a page view is neither
  an exposure nor a conversion. No cookie, no storage, no identifier: per variant there are only
  totals.

### 17.5 The result

Once every variant has its `sample_size` page views, the server compares each variant after the
control with the control, once: the z statistic, the two-sided p-value and `significant` (p below
`alpha`/(k − 1)), in `/_rum` → `experiment.result`. Before that, `result` is `null` — no interim
p-values, because stopping on an early lucky streak is how A/B tests mislead. The result is final:
nothing is counted after it. The proof console's "Real visitors" row (`web/hud/visitors.ts`, which
loads `web/hud/experiment.ts` while a test runs) shows each variant's page views out of the plan,
conversions and their rate, marks the control and the variant of this page view, and shows the
verdicts only when the test is complete (it checks the page views again before showing one).

## 18. Behind a reverse proxy (`--trusted-proxy`)

For a host where another server owns ports 80/443 and ends visitors' TLS (production today: the
provider's OpenResty edge → the host's Caddy → us, plain HTTP on the docker gateway).

| Flag | Meaning |
|---|---|
| `--trusted-proxy CIDR` | Repeatable. Connections from these addresses are proxied. |
| `--client-ip-header NAME` | Where the proxy names the visitor (default `x-forwarded-for`); the **last** address in it counts — the one the proxy itself saw or appended. Production: Caddy sets `X-Mintaka-Client-Ip` from the edge's `X-Forwarded-For`, overwriting anything a client sent. |

- One proxy connection carries many visitors, so the visitor's /24 or /48 is taken **per
  request** (`ReqCtx.client_net`) and used only by the rate limiters; never logged or stored.
  An unreadable header falls back to the proxy's own network (everyone shares one bucket —
  stricter, never looser).
- Connections from anywhere else ignore the header (an untrusted client can't pick its bucket).
- Proxied requests get no kernel RTT, a Server-Timing `edge` flag (§4.5) and `/_proof.edge` (§5);
  HSTS is not sent (this server didn't serve TLS, and a host-wide policy belongs to the host).
- `/_telemetry` sends `Cache-Control: no-store, no-transform` and `X-Accel-Buffering: no`, so a
  compressing or buffering proxy passes ticks through as they happen.

## 19. The Telegram bot

The reply-time promise made true where it can be: with the bot configured, a client who sends the
form can continue in Telegram, where the bot answers at once, in the brief's language, and the
team's replies reach them in the same chat. Server side: `crates/server/src/telegram/` (`api`
client, `link` codes, `chats` memory, `relay` logic).

### 19.1 Configuration

| Variable | Meaning |
|---|---|
| `MINTAKA_TELEGRAM_BOT_TOKEN` | The bot's token (@BotFather). Environment only (a flag would show in the process list); never logged. |
| `MINTAKA_TELEGRAM_CHAT_ID` | The team chat's **numeric** id (a group's looks like `-1001234567890`): notifications, headers and the clients' messages go there. |
| `MINTAKA_TELEGRAM_API_BASE` | Tests only: where the Bot API is (default `https://api.telegram.org`); `https://`, or `http://` to this machine — the token travels in the URL. |

Both of the first two, or neither. At startup the server asks `getMe` (at most 5 s) for the bot's
username, then one task long-polls `getUpdates` (`timeout` 50 s, `allowed_updates: ["message"]`).
Errors back off from 1 s, doubling, to 5 minutes (or `retry_after` on `429`); `409` means a webhook
is set or another server polls with the same token — only one may; a refused token (`401`/`404`)
stops the bot. On shutdown the poll is dropped and the memory written. The manifest must hold every
`proof::BOT_MESSAGES` id for the default language (§3), or startup fails: rebuild `dist/`.

### 19.2 The link

- A successful **JSON** submission answers `"bot_link": "https://t.me/<bot>?start=<code>"` — only
  while the bot is reachable (`getMe` answered, and Telegram answered within the last 120 s),
  because the success screen then says the bot answers right away. Without the bot, or while it
  can't reach Telegram, the answer is `{"ok":true}` as before.
- `<code>` = `b1` + base64url(`ms[6] ‖ random[8] ‖ len[1] ‖ lang[len] ‖ tag[12]`): the brief id's
  two parts (`{ms}-{16 hex digits}`, the stored file's name), its language, and the first 12 bytes
  of HMAC-SHA256(`{data-dir}/telegram.key`, `"mintaka-bot/start/v1"` ‖ the bytes before). At most
  64 characters of `A–Z a–z 0–9 _ -` (Telegram's limit for start parameters). Nothing is stored to
  issue it. It connects a chat for **7 days** after the brief; deleting the key invalidates every
  unused link (chats already connected stay connected).
- A honeypot submission (§8.2) answers like success with a decoy of the same shape, MAC'd under
  `"mintaka-bot/decoy/v1"`, which the bot ignores like a forgery.
- The no-JS form gets no link: its `303` would have to carry the code in a URL, and the proxy in
  front of the server (§18) may log URLs.
- The client (`web/instruments/contact.ts`) shows "Continue in Telegram — our bot answers right
  away" (`contact-bot`, `contact-bot-note`) only when the answer has a link, and only a link of
  exactly that shape. The static copy (`ct-lead`, `sent-body`) promises the team's reply time, never
  an immediate answer.

### 19.3 What the bot does

| Message | What happens |
|---|---|
| `/start <code>` in a private chat, valid | The chat is connected to the brief (a chat's earlier link is replaced); the bot answers at once with `bot-linked` in the brief's language and posts a header in the team chat: "Brief {id} continues in Telegram…". Already connected to this chat: `bot-again`. Connected to another chat: `bot-taken`. |
| `/start <code>`, expired | `bot-expired` |
| `/start <code>`, forged, damaged or a decoy | Nothing |
| `/start` alone, from a chat not connected | `bot-hello` (a pointer to the site) |
| Anything else from a connected chat | `copyMessage` into the team chat, as a reply to the brief's header (no "forwarded from"); files and photos too |
| A team-chat message that replies to a brief's header, a copy, or a reply already sent on | `copyMessage` to the client, as the bot's own message. Refused (the client blocked the bot): a note in the team chat. |
| `/stop` from a connected chat | The link is forgotten; `bot-stopped`; a note under the header in the team chat |
| Everything else | Nothing: other groups and channels, bots, edited messages (not even asked for), chats not connected |

**Limits**, per chat, in memory: a connected chat 30 messages an hour (burst 30), any other private
chat 5; past that, messages are dropped. Team replies aren't limited. The team chat can stay in
Telegram's privacy mode: the bot only needs replies to its own messages.

### 19.4 Memory

`{data-dir}/telegram.json`, mode 0600, replaced atomically (temporary file, fsync, rename):
`{"version": 1, "offset": <next getUpdates offset>, "links": [{"chat", "brief", "lang", "linked",
"seen", "header", "team": [message ids]}]}` — chat ids, brief ids and languages, team-chat message
numbers (at most 200 per link) and Unix times. Never a message, a name or a username. A link is
forgotten 30 days after its last message in either direction, or at once on `/stop`; at most
10,000 links. Written after every batch of updates, so a restart neither forgets a link nor
handles an update twice. Logs name brief ids and what happened, never chat ids or text. The
privacy page (`pv-telegram`) says the same.

### 19.5 Copy

What the bot writes to clients is page copy: `bot-*` in `content/{lang}/page.ftl`, rendered per
language into `manifest.bot` (`$within` is `site.toml`'s reply time and `$site` the language's home
page, at build time; `{brief}` is filled by the server), falling back to the default language
message by message. Notes for the team (header, `/stop`, not delivered) are English in the server,
like the new-brief notification.
