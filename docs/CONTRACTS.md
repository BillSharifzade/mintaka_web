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
| S6 overload lab (Rust → WebAssembly) | `crates/loadsim/`, `web/instruments/loadsim/`, `loadsim.css`, `loadsim.ftl` | worker C |
| S5 payload analyser (Rust → WebAssembly) | `crates/shield/`, `web/instruments/shield/` — **on hold, awaiting the owner's go-ahead** | — |
| Server | `crates/server/`, `crates/proof/src/{proof,telemetry,contact}.rs` (+ re-exports in `lib.rs`), `deploy/`, `bench/` | worker A |
| Proof console, terminal, console greeting, S2 physics, S5 armour | `web/hud/`, `web/terminal/`, `web/console/`, `web/instruments/{physics,armour}.ts`, `web/instruments/tz/`, matching `web/styles/islands/*.css` and `content/en/islands/*.ftl` | worker B |
| S6 follow one request, S6 live load, S7 laws you can feel, S13 contact | `web/instruments/{trace,live,contact,pow.worker}.ts`, `web/instruments/laws/`, matching CSS and FTL | worker C |

Only edit files you own. If you need a change in someone else's file (a new bus event, a token,
a DOM hook), make the smallest possible change and list it in your final report.

## 2. Commands

```sh
cargo xtask build            # TS (esbuild) + CSS (Lightning CSS) + pages → dist/  (main)
cargo run -p server -- --dist dist          # serve dist/ on http://127.0.0.1:8080
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

```
dist/
  manifest.json
  en/index.html  en/index.html.br  en/index.html.gz     → /en/
  en/404.html                                          → not_found["en"], status 404
  en/sent/index.html                                   → /en/sent/   (contact, no-JS result)
  assets/main.3F2A1B9C.js  assets/hero.4D5E6F70.js …    → /assets/…  (immutable)
  assets/fonts/space-grotesk-latin.9A8B7C6D.woff2 …
  favicon.ico favicon.svg apple-touch-icon.png icon-192.png icon-512.png og-image.png
  manifest.webmanifest robots.txt humans.txt sitemap.xml .well-known/security.txt
  text/en/index.ansi text/en/pillars.ansi text/en/contact.ansi   → curl mode
```

## 4. HTTP

### 4.1 Routing

| Request | Response |
|---|---|
| `GET /` | The page for the language negotiated from `Accept-Language` among `manifest.langs` (default `manifest.default_lang`), served **directly — no redirect** (a redirect costs a round trip, and this page is about round trips). Headers add `Vary: Accept-Language` and `Content-Language`. The HTML carries `<link rel="canonical" href="/{lang}/">`. |
| `GET /{lang}` | `301` → `/{lang}/` |
| Any path in `manifest.routes` | That file (see 4.3–4.4). `HEAD` works everywhere `GET` does. |
| `/_proof`, `/_telemetry`, `/_ping` | §5–§7 |
| `GET /api/pow`, `POST /api/contact` | §8 |
| curl / Wget / HTTPie User-Agent | §4.6 |
| Anything else | `404` with `not_found[lang]` (lang from the path prefix, else negotiated). The page contains the literal marker `<!--mintaka:suggest-->`; replace it with `<a href="{route}">{route}</a>` for the closest page route by edit distance, or with nothing. Never echo the requested path. |

Methods other than GET/HEAD on static routes → `405` with `Allow: GET, HEAD`.
Request bodies are only accepted on `POST /api/contact` (limit 32 KiB).

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
```

- `{style hashes}` is every `manifest.csp.style_src` entry, each in single quotes. Same for
  `script_src` (empty today) appended to `script-src`.
- `upgrade-insecure-requests` and HSTS are sent **only over TLS** (on plain-HTTP localhost they
  would break development).
- The full set goes on every response, not only HTML: worker scripts take their CSP from their
  own response, and COEP/COOP are harmless elsewhere.
- No `Set-Cookie`, ever. No `Server` header with versions (the software string lives in `/_proof`).

### 4.3 Cache policy (`Route.cache`)

| Policy | Cache-Control | Validator |
|---|---|---|
| `document` | `no-cache` | strong `ETag` (hash of the identity bytes), honour `If-None-Match` → `304` |
| `immutable` | `public, max-age=31536000, immutable` | ETag |
| `short` | `public, max-age=3600` | ETag |

`/_proof`, `/_telemetry`, `/_ping`, `/api/*`: `Cache-Control: no-store`.

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
| `tls;dur=1.83;desc="TLS handshake"` | Server-side handshake duration. Only on the **first** request of a TLS connection. |
| `krtt;dur=48.21;desc="kernel RTT"` | `TCP_INFO.tcpi_rtt` of this connection (Linux). Every response. |

### 4.6 curl mode

If the `User-Agent` starts with `curl/`, `Wget/`, `HTTPie/` or `xh/`, then for `/`, `/{lang}/`,
`/pillars`, `/contact` (and `/{lang}/pillars`, `/{lang}/contact`) respond
`200 text/plain; charset=utf-8` with the file from `manifest.text[lang][path]` (`/{lang}/x` uses
key `/x`; `/` and `/{lang}/` use key `/`), after replacing these tokens:

| Token | Example value |
|---|---|
| `{{server_time}}` | `21 µs` (this request so far; µs below 1 ms, else ms with 1 decimal) |
| `{{tls}}` | `TLS 1.3 · X25519MLKEM768 (post-quantum hybrid)` or `plain HTTP (development)` |
| `{{http}}` | `HTTP/2` / `HTTP/1.1` |
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
  "connection": { "id": 812, "requests": 3, "kernel_rtt_us": 48210, "kernel_rttvar_us": 1200 },
  "privacy": { "cookies_set": 0, "third_party_origins": 0, "ip_stored": false, "ip_logged": false },
  "headers": [
    "content-security-policy", "cross-origin-opener-policy", "cross-origin-embedder-policy",
    "cross-origin-resource-policy", "permissions-policy", "referrer-policy",
    "x-content-type-options", "strict-transport-security"
  ]
}
```

- `server.location` is configuration (where the machine is), `null` if not configured.
- `tls` is `null` on plain HTTP. `post_quantum` is true for any hybrid/ML-KEM group.
- `connection.*` describes the connection this request arrived on. `kernel_rtt_us` is `null` when
  `TCP_INFO` is unavailable. `id` is a process-local counter, never derived from the client.
- `headers` lists only headers actually sent on this response (so no HSTS over plain HTTP).

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

At least one of `email` / `telegram` is required. Validation lives in Rust newtypes
(`Name`, `Email`, `TelegramHandle`, …) — parse, don't validate.

**Error codes** (kebab-case, unique across fields): `name-required`, `name-too-long`,
`email-invalid`, `telegram-invalid`, `contact-required`, `message-too-long`, `company-too-long`,
`choice-invalid`, `pow-invalid`, `pow-expired`, `rate-limited`, `too-large`, `server-error`.

**Responses — JSON request:** `200 {"ok":true}` · `422 {"ok":false,"errors":[{"field":"email","code":"email-invalid"}]}` ·
`429 {"ok":false,"errors":[{"field":"form","code":"rate-limited"}]}` · `400` for malformed bodies.

**Responses — form request:** `303 See Other` to `/{lang}/sent/` on success, or to
`/{lang}/sent/#e-{code}` for the first error (the static page shows the message for `:target`).

**Rate limits** (in memory only, keyed by IPv4 /24 or IPv6 /48 prefix, never written to disk or
logs): 10 per hour with a valid proof of work, 3 per hour without.

**Storage:** each accepted message is one file in `{data-dir}/contact/`, mode `0600`, named
`{unix-ms}-{random}.json.age`, encrypted with `age` to the configured X25519 recipient. The server
holds only the public key; without a configured recipient it creates a development identity in
`{data-dir}/dev-identity.txt` and logs a loud warning. **Notification** (optional, env-configured
Telegram bot): "New brief · kinds · timeline · id" — never the message body.

## 9. The page (DOM contract)

```html
<html lang="en" data-shortcuts-off?>
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
    <section id="team" data-star="bellatrix">…</section>                                    S12
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
  children (`render(body, …)`) and may update `.instrument__status`. Never remove the frame.
- **The contact form** is complete, working HTML without JS (labels, `name`s matching §8.2,
  `required`, `type="email"`, honeypot field `website` inside a visually hidden wrapper with
  `tabindex="-1"` and `autocomplete="off"`). The island enhances it; it never replaces it.
- `data-action="proof" | "terminal" | "shortcuts"` clicks are handled in `web/main.ts`.

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

## 10. Islands

- A module per island exports `mount(root: HTMLElement): void` (instruments, contact) or the
  functions `web/main.ts` calls (`hud`: `mount()`, `open()`; `terminal`: `open()`;
  `console`: `greet()`; `hero`: `start(canvas)`; `nav`: `start()`). `open()` must be idempotent
  and must work before or after `mount()`.
- `web/main.ts` loads instrument islands when their container comes within one screen of the
  viewport; HUD/console/nav on idle; the terminal on first use.
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
   Budgets (minified, before compression): HUD ≤ 6 kB, terminal ≤ 8 kB, each instrument ≤ 15 kB
   (data tables excluded, loaded lazily).
7. **Strings** come from `content/en/islands/<island>.ftl` and are read with `t('<island>-key')`.
   Message ids are prefixed with the island name. Variables (`{ $ms }`) and at most **one selector
   per message** on a variable, for plurals: `x-requests = { $n -> [one] { $n } request *[other] { $n } requests }`.
   The generator exports each variant as `id[key]` (plus the default as `id`); read counts with
   `tp('x-requests', n)`, which picks by `Intl.PluralRules` for the page language.
   English is the source; Russian and Tajik are written natively by the team later — never
   machine-translate.
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

## 11. Build-time flags

`MINTAKA_DEV` (esbuild `--define`) is `true` in `cargo xtask build`/`dev` and `false` with
`--release`, where dead code behind it is removed. Development-only hooks live behind it — e.g.
`window.__hero.advance(seconds)` runs the hero simulation synchronously for reviews in throttled
or automated browsers.

## 12. Fonts

`--font-sans` = Space Grotesk (variable, Latin) → Onest (variable, Cyrillic incl. Tajik letters) →
metric-matched fallback. `--font-mono` = IBM Plex Mono 400/500 (Latin + Cyrillic). Space Grotesk's
Latin subset lacks `→ ≤ ≈ λ ₂`; those fall through to Onest's math subset (loaded only when such a
glyph is on screen) — avoid them above the fold.

## 13. WebAssembly instruments

Three instruments are Rust compiled to WebAssembly: S4 `routing` (main), S5 `shield` (worker B),
S6 `loadsim` (worker C). Each is a crate under `crates/<name>`:

```toml
[package]
name = "shield"            # = the island name = the asset name
version.workspace = true
edition.workspace = true
publish.workspace = true

[lib]
crate-type = ["cdylib", "rlib"]   # cdylib for wasm32, rlib so `cargo test -p shield` runs natively

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
- **Budgets:** each `.wasm` ≤ 250 kB (checked by the build), each island ≤ 15 kB minified.
- CSP already allows WebAssembly (`'wasm-unsafe-eval'`); nothing else changes.
