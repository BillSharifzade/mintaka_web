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
#                                 https://127.0.0.1:8762/ with post-quantum TLS, and --http3
#                                 for HTTP/3 on UDP 8762 as well)

cargo xtask dev                  # build, serve on :8761, rebuild + restart on every change
                                 # (MINTAKA_LISTEN=127.0.0.1:9000 and MINTAKA_LOCATION=… override)
cargo xtask build --release      # drops unverified claims; fails on budgets, missing glyphs or strings
cargo xtask graph [--fetch]      # rebuild assets/data/dushanbe.bin from OpenStreetMap (Overpass API)
```

Check it from a terminal: `curl http://127.0.0.1:8761/` (curl mode), `curl -sk https://127.0.0.1:8762/_proof` (look for `"group": "X25519MLKEM768"`), `curl --http3-only -sk https://127.0.0.1:8762/_proof` (with `--http3`: `"alpn": "h3"`, `"version": "HTTP/3.0"`; needs a curl built with HTTP/3), `curl http://127.0.0.1:8761/_rum` (real visitors' Web Vitals, aggregated).

Tests: `cargo test --workspace` · `npx tsc --noEmit -p .` · `node --test "web/**/*.test.ts"` (after one `cargo xtask build`: the fuzzer tests load the built module).

## How it fits together

```
content/ (Fluent strings, site.toml) ─┐
web/styles/ (CSS)  ── Lightning CSS ──┤
web/**/*.ts  ── esbuild (xtask) ──────┼─► crates/site (Rust generator) ─► dist/ + manifest.json
assets/ (fonts, brand kit files) ─────┘                                        │
                                                                               ▼
                              crates/server (Axum + rustls, TLS 1.3, X25519MLKEM768;
                              HTTP/3 via quinn + h3; certificates via ACME)
                              loads every route into memory at startup; per request it only
                              picks an encoding and adds headers, Server-Timing and /_proof facts
```

| Path | What |
|---|---|
| `crates/site` | Generator: Maud templates (`src/pages/`), Fluent strings, Lightning CSS, font `@font-face` + metric-matched fallbacks + a WOFF2 glyph-coverage check, curl-mode ANSI art rasterised from the logo SVG, `security.txt`/`sitemap.xml`/`humans.txt`, the Lab (Markdown → HTML with MathML and highlighted code, no JavaScript, Atom feed), the service worker's file list, precompression, budgets |
| `crates/server` | The server: headers, Server-Timing (incl. kernel RTT), `/_proof`, `/_telemetry` (SSE), `/_ping`, `/_rum` (real-user Web Vitals and event counts, in memory), curl mode, 404 suggestions, contact API with proof of work and age encryption, the console challenge API (`/api/ctf/*`), HTTP/3 (quinn + h3), Let's Encrypt certificates from inside the binary (TLS-ALPN-01) |
| `crates/proof` | Types shared by both (manifest, `/_proof`, telemetry, contact, `/_rum`, console challenge, the A/B test and its statistics) |
| `crates/fuzzer` | S5: a coverage-guided fuzzer attacking a small parser of ours (v1 with one bug, v2 fixed) → WebAssembly without bindgen |
| `crates/routing` | S4: road graph, Dijkstra, A*, our own Contraction Hierarchies (build-time `prep` feature turns an OpenStreetMap extract into `assets/data/dushanbe.bin`) → WebAssembly |
| `crates/loadsim` | S6: discrete-event queueing simulator (naive vs. load-shedding + autoscaling) → WebAssembly |
| `xtask` | `cargo xtask build / dev` |
| `bench` | `bench/run.sh`: the published benchmark, one command on the machine being measured |
| `web/hero` | The hourglass engine: WebGL2 transform-feedback particle simulation in logo space (tier B), Canvas 2D twin (tier C), static poster (tier D) |
| `web/hud`, `web/terminal`, `web/console` | Proof console (`P`), terminal (`~`), DevTools greeting + `mintaka.help()`, and the console challenge (`mintaka.ctf()`: four puzzles, then an application form) |
| `web/instruments` | S2 physics, S4 routing race, S5 fuzzer + armour, S6 follow-one-request + live load + overload lab, S7 laws you can feel, S13 contact (+ proof-of-work worker) |
| `web/nav` | Constellation navigation ("you are here") |
| `web/rum`, `web/sw`, `web/offline` | Real-user Web Vitals beacon (no library); the service worker (offline mode) and its registration |
| `web/shared` | DOM builder (Trusted Types–safe), strings, `/_proof` client, timing, bus, formatting |
| `content/` | `site.toml` (published facts, claims to verify, the team), `en/page.ftl`, `en/islands/*.ftl` (and `islands/lazy/` bundles), `en/lab/*.md` (Lab posts) |
| `assets/team` | Team portraits (AVIF + WebP, 224 px), published with each person's consent |

## Certificates and HTTP/3 in production

The server gets its certificate from Let's Encrypt itself and renews it about 30 days before
expiry, without a restart (docs/CONTRACTS.md §14). Validation is **TLS-ALPN-01** on the TLS
listener: Let's Encrypt connects to port 443 and the server answers with a challenge certificate.
Nothing on port 80 is needed for it (`--redirect-http` there is only the HTTP → HTTPS redirect).

1. **DNS:** A (and AAAA, if any) records for every name point at this server. An AAAA record
   that does not reach it can fail validation: Let's Encrypt connects over IPv6 when one exists.
2. **CAA:** allow only Let's Encrypt to issue for the domain:

   ```dns
   mintaka.tj.  CAA 0 issue "letsencrypt.org"
   mintaka.tj.  CAA 0 issuewild ";"
   ```

   Optional, once the account exists (RFC 8657): pin it to this server's account and method. The
   account URL is in the log (`acme: created ACME account https://…/acme/acct/…`, then
   `using ACME account …` on later starts); the production and staging accounts differ.

   ```dns
   mintaka.tj.  CAA 0 issue "letsencrypt.org; accounturi=https://acme-v02.api.letsencrypt.org/acme/acct/<id>; validationmethods=tls-alpn-01"
   ```

   CAA applies to subdomains too (`www.mintaka.tj` inherits it unless it has its own records).
3. **Firewall:** 443/tcp (HTTPS and validation), 443/udp (HTTP/3), 80/tcp (redirect).
4. **Persistent `--data-dir`:** the account key and certificates are cached in
   `{data-dir}/acme` (0600 files). Without it, every start orders a new certificate and soon
   runs into Let's Encrypt's limit on duplicate certificates (5 per exact set of names per 7
   days, <https://letsencrypt.org/docs/rate-limits/>).
5. **First against staging** (untrusted certificates, generous rate limits). Running with
   `--acme-domain` means agreeing to the CA's subscriber agreement
   (Let's Encrypt: <https://letsencrypt.org/repository/>).

   ```sh
   mintaka-server --dist /opt/mintaka/dist --data-dir /var/lib/mintaka \
     --tls-listen '[::]:443' --redirect-http '[::]:80' --http3 \
     --acme-domain mintaka.tj --acme-domain www.mintaka.tj \
     --acme-contact mailto:ops@mintaka.tj --acme-directory staging
   ```

   (With systemd, add `--acme-directory staging` to `ExecStart` in `deploy/mintaka.service`
   instead; by hand, the process needs to bind port 443 — root or `CAP_NET_BIND_SERVICE`.)
   Watch for `acme: new certificate for mintaka.tj, www.mintaka.tj …` in the log, then check
   from another machine:

   ```sh
   openssl s_client -connect mintaka.tj:443 -servername mintaka.tj </dev/null 2>/dev/null \
     | openssl x509 -noout -issuer -enddate          # issuer: (STAGING) …
   curl -sk https://mintaka.tj/_proof               # -k: staging is not trusted
   curl -sk --http3-only https://mintaka.tj/_proof  # "alpn": "h3"
   ```

6. **Production:** drop `--acme-directory staging` (or set `production`) and restart. The cache
   is kept per directory, so production gets its own account and certificate; the staging files
   can be deleted.

`deploy/mintaka.service` (systemd) and `deploy/Dockerfile` are set up this way. In a container
that maps 443 onto 8443, add `--http3-advertise-port 443` (the Dockerfile does) so `Alt-Svc`
announces the port clients reach.

**Not verified here:** an issuance by the real Let's Encrypt. This machine has no public
domain or port 443, so the full flow was tested against a small ACME CA written into the test
suite (`crates/server/tests/acme.rs`: account, order, TLS-ALPN-01 validation of the real
listener, CSR, download, renewal, cache, restart). The first staging run above is the real test.

## Where it runs now: `https://dchr.koinotinav.com/mintaka/`

Until Mintaka has a domain of its own, the site is a path on the hosting provider's domain
(Koinot Innovations and Technologies, Dushanbe). That host already has an owner for ports 80 and
443, so the design above (our own TLS, HTTP/3, the kernel's RTT) runs on the LAN preview only;
production looks like this:

```
browser ─HTTPS─▶ provider's edge (OpenResty, wildcard *.koinotinav.com certificate)
        ─HTTP─▶ the host's Caddy :80, route /mintaka/*  ─HTTP─▶ mintaka-prod 172.20.0.1:8790
```

- The site is built for the prefix (`base = "/mintaka"` in `content/site.toml`, CONTRACTS §3.1)
  and the server strips it. `MINTAKA_BASE= cargo xtask build` builds for the root instead.
- The server runs with `--trusted-proxy 172.20.0.0/16 --client-ip-header x-mintaka-client-ip`
  (CONTRACTS §18): rate limits per visitor, `/_proof` reports `edge` instead of TLS it can't see.
- On the server: `~/mintaka-prod/` holds `bin/mintaka-prod`, `dist/`, `data/` (briefs, the
  console-challenge key, the Telegram bot's key and memory), `env` (mode 600:
  `MINTAKA_CONTACT_RECIPIENT`, the Telegram settings below) and `run.sh`;
  `~/mintaka-web/watchdog.sh` (cron, every minute) restarts it. `deploy/koinot/` moves it to a
  sandboxed systemd service under an account of its own, with its data in `/var/lib/mintaka`.
- Deploy: `cargo build --release -p server && cargo xtask build --release`, check
  `objdump -T target/release/mintaka-server | grep -o 'GLIBC_[0-9.]*' | sort -uV | tail -1` is at
  most 2.39 (Ubuntu 24.04), copy `dist/` and the binary (as `bin/mintaka-prod`, last), run
  `run.sh` (under systemd, the copy itself restarts the service).

### Contact briefs: the key and reading them

Briefs are encrypted to an [age](https://age-encryption.org) public key; the server never holds
the private half, so a stolen server can't read them.

```sh
cargo xtask keygen ~/.config/mintaka/briefs-identity.txt   # once; prints the age1… public key
rsync -a <server>:mintaka-prod/data/contact/ briefs/       # fetch the encrypted files
#   (under the systemd service: <server>:/var/lib/mintaka/contact/)
cargo xtask briefs ~/.config/mintaka/briefs-identity.txt briefs/
```

Keep the identity file in a password manager too: without it, stored briefs can't be read.

### Contact briefs in Telegram: the team chat and the client bot

With a Telegram bot configured, the server posts "New brief · …" to the team's chat, and the form's
last screen offers the client "Continue in Telegram — our bot answers right away". The bot answers
at once in the brief's language, copies what the client writes into the team chat, and sends a team
member's *reply* to one of those messages back to the client (CONTRACTS §19). Without the two
settings nothing changes: no notifications, no link, no promise.

1. **Create the bot:** in Telegram, open @BotFather, send `/newbot`, choose a name ("Mintaka") and a
   username ending in `bot`. BotFather replies with the token (`123456789:AA…`). It is a password:
   keep it out of chats, screenshots and shell history; if it leaks, `/revoke` in BotFather.
2. **The team chat:** create a group for the team (or use an existing one) and add the bot. Privacy
   mode can stay on: the bot only needs replies to its own messages.
3. **The chat's id:** send any message in the group, then — before the server runs with the token,
   which would take the update itself — ask Telegram, typing the token without saving it:

   ```sh
   read -rs TOKEN && curl -s "https://api.telegram.org/bot$TOKEN/getUpdates" | grep -o '"chat":{"id":-*[0-9]*'
   ```

   A group's id is negative, e.g. `-1001234567890`.
4. **On the server**, add both to `~/mintaka-prod/env` (mode 600) and restart with `run.sh`:

   ```sh
   MINTAKA_TELEGRAM_BOT_TOKEN=123456789:AA…
   MINTAKA_TELEGRAM_CHAT_ID=-1001234567890
   ```

5. **Check:** the log says `bot @… answers clients and relays them to the team chat`. Send a test
   brief from the site, follow the link, and reply to the copy in the team chat.

Only one server may poll a bot: a LAN preview must not run with the production token (Telegram
then answers `409 Conflict` to one of them). The bot keeps `{data-dir}/telegram.key` (its links'
key) and `{data-dir}/telegram.json` (which chat belongs to which brief, 30 days); back them up with
the rest of `data/`.

## House rules (enforced in code where possible)

- **Mono means measured.** A number in IBM Plex Mono was measured on this page view or computed from a cited source. Never an example.
- **Claims must be true on launch day.** Practices listed in `content/site.toml` are shown with an *UNVERIFIED* badge in development and **dropped from release builds** until marked `verified = true`.
- **Zero third-party requests, zero cookies.** CSP `default-src 'none'`, Trusted Types enforced, COOP + COEP (the page is cross-origin isolated).
- **Analytics are totals on our own server.** One beacon per page view to `/_rum`, aggregated in memory (no IP, user agent or cookie; nothing on disk); Global Privacy Control and Do Not Track mean nothing is sent. The totals are public in the proof console and at `/_rum`. An A/B test of the home page (`[experiment]` in `content/site.toml`, one at a time) draws a variant per page load on the server and counts totals per variant up to a sample size fixed in advance (docs/CONTRACTS.md §17).
- **Budgets** (spec §10.1) are checked on every build: first screen ≤ 14 kB compressed (one round trip), first full view ≤ 100 kB, hero ≤ 25 kB, HUD ≤ 6 kB, each instrument ≤ 15 kB.
- **Languages:** the build fails on any missing string in an enabled language. Russian and Tajik were drafted by Claude at the founder's request; native speakers review them (docs/TRANSLATION.md).

## Status

[`docs/MILESTONE-1.md`](docs/MILESTONE-1.md): what is done, what was measured, where the build
corrected the spec, what is still a placeholder, and what comes next.
[`docs/MILESTONE-2.md`](docs/MILESTONE-2.md): the WebAssembly instruments.
[`docs/MILESTONE-3.md`](docs/MILESTONE-3.md): team, console challenge, Lab, real-user metrics,
offline mode, Let's Encrypt and HTTP/3, the benchmark harness.
[`docs/MILESTONE-4.md`](docs/MILESTONE-4.md): live at dchr.koinotinav.com/mintaka behind the
provider's edge, the A/B test, the Telegram bot, the Lab review, the license, the production
benchmark, the [penetration test](docs/SECURITY-REVIEW.md) and the accessibility audit.

## License

The source code is MIT or Apache-2.0, at your option; the Lab articles are CC BY 4.0; the name,
logo, copy and team portraits are all rights reserved; fonts and map data keep their own licences.
Details in [LICENSE](LICENSE).
