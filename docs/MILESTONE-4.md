# Milestone 4 — live, tested on people's terms, attacked by us (30 October 2026)

The site went public at **https://dchr.koinotinav.com/mintaka/**, on the hosting provider's
domain until Mintaka has its own — in **Tajik, Russian and English**. This round also brought an
A/B test, an instant-reply Telegram bot, the Lab's editorial review, a license, a benchmark on the
production machine, a penetration test and an accessibility audit.

## Production

- **Where:** Koinot Innovations and Technologies, Dushanbe — a 2-vCPU VM shared with other
  applications. Browser → the provider's OpenResty edge (wildcard `*.koinotinav.com` certificate,
  HTTP/2, no HTTP/3) → the host's shared Caddy (route `/mintaka/*`) → our server on the
  docker gateway, `172.20.0.1:8790`. README → "Where it runs now" has the layout and deploy steps.
- **A URL prefix** (CONTRACTS §3.1): `base = "/mintaka"` in `content/site.toml`. Route paths stay
  root-relative; every link the generator writes carries the prefix; the server strips it before
  routing and puts it back on its redirects; islands build URLs with `at()`.
- **Edge mode** (CONTRACTS §18): `--trusted-proxy` / `--client-ip-header`. Rate limits are per
  visitor (Caddy passes the edge's `X-Forwarded-For` as `X-Mintaka-Client-Ip`; forged headers
  don't move a client between buckets — tested through the real edge). `/_proof` reports
  `edge: {proto: "https"}` instead of TLS this server never saw; Server-Timing carries `edge` and
  no kernel RTT; the proof console, armour scan and request trace say "HTTPS at the hosting edge".
- **Let's Encrypt** doesn't apply on this host: the edge already serves a GlobalSign wildcard
  certificate and port 443 never reaches our server. Our own ACME code is unchanged, still tested
  only against the in-process CA.
- **The LAN preview** (`192.168.1.156:8761`/`:8762`, root build, own TLS 1.3 + HTTP/3) keeps the
  full design running: in Chrome over HTTP/3 it reports `HTTP/3.0`, `X25519MLKEM768` and the QUIC
  RTT.
- **Incident:** adding the route reloaded the shared Caddy from its configured path, which the
  container sees as a stale copy from July (single-file bind mount, file replaced after start):
  another application's route was down for about two minutes until the routes were reloaded from
  a copy of the current file. Restarting the container fixes it: a restart binds the current
  file (docs/SECURITY-REVIEW.md, L3).

## Contact

- The founder's phone (`+992 98 544 70 72`) is the only public contact; empty details are not
  published, and security.txt and the careers link fall back to it. No placeholder emails or
  Telegram handles remain.
- Briefs are encrypted to the team's age key, made on the founder's laptop
  (`cargo xtask keygen`; `~/.config/mintaka/briefs-identity.txt`). `cargo xtask briefs` reads
  them. The files are 0640 in a 0750 directory (ciphertext), so the deploy account can still
  fetch them once the server runs under an account of its own.
- **Telegram bot** (CONTRACTS §19): once configured, the success screen offers "Continue in
  Telegram"; the bot answers at once in the brief's language and relays messages between the client
  and the team chat in both directions. Links are signed and expire after 7 days; the chat
  mapping (no message text) is kept 30 quiet days. The team's own reply-time promise stays "one
  working day" — the instant answer is the bot's. The bot is @mintaka_reply_bot; it starts once it
  is in the team's group, whose id completes its settings (README → "Contact briefs in Telegram").
- No budget question: the founder chose to keep the form short.

## A/B test (CONTRACTS §17)

The hero headline against the spec's two alternatives, a variant drawn per page load on the
server (no cookie), primary metric "followed a link to the contact form". Planned in advance:
baseline 5 %, +30 % relative lift, α 0.05 over two comparisons, power 0.8 → **4,578 page views per
variant**; the build refuses a `sample_size` that doesn't match. Results appear in the proof console
only once every variant reaches it. Counts are in memory, so **every deploy restarts the test**.
The baseline is a placeholder until two weeks of `/_rum` show the real one.

## The Lab

An editorial review corrected every post (method descriptions, overclaims, a wrong RFC section,
the translation process in post 4, post 3's production paragraph). The timing programs now live in
the repo: `cargo run --release -p routing --features prep --example timing` and
`cargo run --release -p server --example handshakes -- 127.0.0.1:18843 5000`; reruns land in the
published ranges.

## Also

- **Practice claims:** four of the fourteen are shown, each with its evidence beside it in
  `content/site.toml`: secret scanning on every push (GitHub secret scanning and push
  protection), A/B tests (this page runs one), perceptual comfort (the accessibility audit) and
  speed as UX (the build's budgets). Hidden until they are true or the founder confirms them:
  hardware keys, signed commits, SBOM and reproducible builds, quarterly access reviews, managed
  laptops, incident drills, research, usability rounds, cognitive walkthroughs, and WCAG 2.2 AA
  (one known gap).
- **License** (LICENSE): code MIT or Apache-2.0; Lab posts CC BY 4.0; name, logo, copy and
  portraits all rights reserved; fonts OFL; the road network ODbL.
- **Hosting** is named on the privacy page, with what the provider's edge sees.
- **QUIC RTT:** S2 physics, S6 trace and the terminal use `qrtt` over HTTP/3.
- **Team line** of the founder rewritten from the founder's own site; the co-founder's kept (it
  renders the co-founder's own headline).

## Benchmark on the production machine

`bench/results/2026-10-30-koinot-dyu-1.md` — run with `nice`, at modest rates, beside the other
services (load average 0.04 before, 0.20 after):

| Protocol, rate | Client p50 | Client p99 | Server `app` p50 | Server `app` p99 | CPU |
|---|---:|---:|---:|---:|---:|
| HTTP/1.1, 1,000/s | 0.202 ms | 0.499 ms | 14 µs | 34–35 µs | 12–14 % |
| HTTP/1.1, 4,000/s | 0.291 ms | 0.743 ms | 10–14 µs | 39–47 µs | 25–28 % |
| HTTP/2 + TLS 1.3, 1,000/s | 0.388 ms | 1.153 ms | 10–17 µs | 41–45 µs | 18–21 % |
| HTTP/2 + TLS 1.3, 4,000/s | 0.364 ms | 1.910 ms | 7–14 µs | 27–38 µs | 29–32 % |

## Browser checks (the ones milestone 3 couldn't do)

- **Offline mode:** headless Chrome with the network emulated off reloads from the service worker
  — on the LAN preview (certificate pinned) and on production (real certificate); a page the
  visit never opened also opens offline.
- **Chrome over HTTP/3:** on the LAN preview, `nextHopProtocol` is `h3` and `/_proof` agrees.
- **The proof console over HTTPS:** full TLS facts on the LAN preview; "HTTPS at the hosting
  edge" on production. No console errors; cross-origin isolated on both.

## Security review

docs/SECURITY-REVIEW.md: two medium findings left for decisions (the shared origin; the process
running with docker-group rights — `deploy/koinot/install.sh` moves it to a sandboxed systemd
service in one `sudo` session, and puts the old process back if anything fails), two low ones
fixed (cross-site form posts; one client holding every live-traffic stream), and the hardening
that held, through the real edge as well as directly.

## Russian and Tajik

Drafted by Claude at the founder's request ("you are good at Russian; once ru is done, create
tg"), for review by native speakers — docs/TRANSLATION.md has the glossary (Tajik column filled),
the conventions, the uncertain choices in reading order and a sign-off table. All 977 messages
per language and the four Lab posts; `/` picks the language from `Accept-Language` (else English).

- A build check (`check_translation`) fails on a missing file or message, a changed id, an
  unknown variable or a bad variant key.
- Numbers, units and lists are formatted per language in Rust and TypeScript. Browsers have no
  Tajik locale data: Tajik borrows Russian digits and separators and one/other plurals.
- To fit the budgets, text needed only after a click (terminal, fuzzer, the proof console's
  panel, console help, form messages, trace notes) now loads on first use: English's whole home
  page fell from 31.1 to 26.7 kB.
- Layout: Russian keeps the menu up to 72 rem; Tajik preloads only its Cyrillic fonts; the Tajik
  A/B headlines were kept about as long as the others so line breaks don't skew the test.
- Still English on ru/tg pages: the parts of curl mode the server fills in, and the physics
  table's Latin city name.

## Accessibility (WCAG 2.2 AA)

docs/ACCESSIBILITY.md: every page type at 1440 and 320 px, reduced motion on and off, the
accessibility tree, a Tab walk and keyboard scenarios through every instrument, in headless Chrome
with our own checks (nothing could be installed). Fixed, among others:

- `content-visibility: auto` hid everything below the first screens from the accessibility tree
  (at 320 px the tree held 5 headings and 12 links) — removed.
- Tab skipped instruments that hadn't mounted; each frame is now a tab stop that mounts its island.
- The open proof console covered focused elements (14 at 1440 px, 51 at 320 px); it now closes when
  focus leaves it and gives focus back to its opener. The header menu closes on Escape.
- The hero animation can be paused (a visible button), and so can the live-load panel (2.2.2).
- The page scrolled sideways at 320 px; field borders were 1.71:1 (now ≥ 3.97:1); selected chips
  differed by colour only; the application form's `aria-invalid` never turned on.

Not fixed: choosing your own start and destination on the routing map still needs a pointer
(keyboard users get presets and random pairs). No screen-reader pass was possible here — the doc
lists what a person should test first.

## Tests and budgets

254 Rust tests and 158 TypeScript tests, all green; clippy clean. Release build, brotli, for
every A/B variant:

| | First screen (≤ 14) | Whole home page (≤ 32) | First full view (≤ 100) |
|---|---:|---:|---:|
| tg | 11.9 kB | 30.4 kB | 85.9 kB |
| ru | 11.9 kB | 31.0 kB | 98.3 kB |
| en | 11.6 kB | 26.7 kB | 78.2 kB |

Russian's first view has 1.7 kB left; the lever, if it needs room, is to stop preloading Space
Grotesk Latin for Russian as Tajik does (−22 kB, "Rust"/"English" swap in a moment late).
Budgets raised this round, each with a dated comment in `crates/site/src/lib.rs`: rum 3.0 →
3.2 kB (A/B fields, URL prefix); fuzzer 21 → 21.5 kB and HUD 6.0 → 6.2 kB minified (code that
fetches their text on first use — the spec's 6 KB for the proof console is a transfer size, and
it is ~2.4 kB brotli); sw.js 8 → 11 kB (24 pages × A/B ETags, 77 prefixed asset URLs).

## Waiting on the founder

- The hidden practice claims: confirming the ones that are true (hardware keys, laptops,
  research, usability rounds, walkthroughs); the others need work first.
- Restarting the shared Caddy container: a restart binds the current Caddyfile (a few seconds
  for every app behind it).
- Running `deploy/koinot/install.sh` with `sudo` (one command, README in that directory).
- Adding @mintaka_reply_bot to the team's Telegram group (its id completes the settings), and
  later replacing the temporary token (BotFather `/revoke`, the new one into `~/mintaka-prod/env`).
- Backing up `~/.config/mintaka/briefs-identity.txt` (without it, briefs can't be read).
- Reviewing the Russian and Tajik drafts with native speakers (docs/TRANSLATION.md lists what to read first).
- A hostname of Mintaka's own (the shared origin is the review's first finding).
