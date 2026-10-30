# Penetration test of the site — 30 October 2026

Spec phase 4 asks for "a penetration test of the site itself". This is that test, done by us (an
AI engineer working for the founder) — not an independent audit; an outside tester should repeat
it before anyone relies on it.

## Scope and method

- **In scope:** Mintaka's own endpoints, nothing else on the machines:
  - production, `https://dchr.koinotinav.com/mintaka/` (through the provider's edge and the
    host's Caddy) and its upstream `172.20.0.1:8790`;
  - the LAN preview, `http://192.168.1.156:8761` and `https://192.168.1.156:8762` (TLS 1.3,
    HTTP/3);
  - the code in this repository and its dependencies.
- **Out of scope:** the provider's edge (OpenResty), the shared Caddy, the other applications on
  the host, the operating system. Nothing was aimed at them; findings about them are noted only
  where they affect us.
- **Method:** black-box probes with a script of ours (`deploy/probe.py`: one line per check,
  non-destructive, rate-limited by design), `openssl s_client` for TLS, headless Chrome for the
  browser-side checks, a code review of the request path, and `cargo audit` against the RustSec
  database (local copy of 22 September 2026). Every finding below has a reproduction in the
  probe output or a test.

## Findings

| # | Severity | Finding | Status |
|---|---|---|---|
| M1 | Medium | Production shares its origin with other applications | Open: needs a hostname of our own |
| M2 | Medium | The production process runs with root-equivalent rights | Fix ready, needs `sudo` once |
| L1 | Low | Cross-site form posts to `/api/contact` were accepted | **Fixed** |
| L2 | Low | One client could hold every `/_telemetry` stream | **Fixed** |
| L3 | Low | The shared Caddy's container reads a stale Caddyfile | Open: a restart of the container fixes it |
| I1 | Info | Request heads up to ~400 KiB were buffered | **Fixed** (64 KiB) |
| I2 | Info | HTTP/1.1 requests without `Host` were answered | **Fixed** (400) |
| I3 | Info | The trusted-proxy range is the whole docker network | Accepted |
| I4 | Info | Two build-time dependencies have advisories | Accepted (not shipped) |
| I5 | Info | Bare-LF line endings are accepted | Accepted |
| I6 | Info | Real-user and A/B counts can be skewed by forged beacons | Accepted |

**M1 — shared origin.** `dchr.koinotinav.com` also serves several other applications, each under
a path of its own. A browser treats all of them as one origin:
their scripts can read our pages' storage (the console challenge's progress token, the shortcut
setting), script our pages in the same tab group, and send our APIs requests our server can't
tell from ours. Their cookies reach our server (it ignores them; the proof console reports them
as not ours). Our CSP, Trusted Types and COOP/COEP protect our pages from third parties, not from
the same origin. *Fix:* a hostname of our own — `mintaka.koinotinav.com` is covered by the
provider's wildcard certificate but needs DNS and an edge route from the provider — or our own
domain. Until then, nothing secret lives in the browser.

**M2 — process rights.** `mintaka-prod` runs as `dchradmin`, which is in the `docker` group;
anyone who could run code as our server could start a privileged container and own the host.
Nothing found suggests such a hole (Rust, no shell-outs, no file paths from requests), but the
blast radius is the whole shared machine. *Fix:* `deploy/koinot/` runs it under an account of
its own in a systemd sandbox (read-only system, no capabilities, no `/home`, a syscall filter;
`systemd-analyze security` exposure 1.3) and keeps no-sudo deploys; installing it takes one
`sudo` session (`deploy/koinot/install.sh`, which puts the old process back if anything fails).

**L1 — cross-site briefs.** The no-JavaScript contact form posts
`application/x-www-form-urlencoded`, which any site can make a visitor's browser send. Hostile
pages could have used visitors' own browsers — and networks, past the per-network limit — to
send us briefs. *Fixed:* requests whose `Sec-Fetch-Site` is anything but `same-origin` get `403`
(clients without the header, such as curl, are unaffected); test
`protocol_and_cross_site_guards`. The JSON paths were already protected by the CORS preflight.

**L2 — live-panel exhaustion.** `/_telemetry` allowed 64 concurrent streams in total; one client
opened 64 and everyone else got `503` (probe: 64 × 200, then 503). *Fixed:* at most 4 streams per
/24 or /48, inside the total; unit test `stream_limit`.

**L3 — the stale Caddyfile (found while deploying, about the host).** The host's Caddy
container mounts its Caddyfile as a single file, and that file was replaced after the container
started (in July), so the container still sees the old copy, without the routes added since.
Reloading Caddy from its configured path therefore dropped another application's route for about
two minutes on 30 October, until the routes were loaded from a copy of the current file. *Fix:*
restart the container. A restart binds the file as it is now (tested with a throwaway container),
and Caddy already runs that file's configuration, so only the mount changes — a few seconds'
interruption for every app behind it. An editor that replaces the file makes the copy stale
again; mounting the directory instead of the file ends that.

**I1, I2.** hyper buffers request heads up to ~400 KiB by default and answers HTTP/1.1 without
`Host` (RFC 9112 §3.2 says 400). Now: heads over 64 KiB get `431`; missing `Host` gets `400`.

**I3.** `--trusted-proxy 172.20.0.0/16` trusts every container on that docker network to name
a visitor's address. The worst any of them can do with it is move requests between rate-limit
buckets. Narrow it to the Caddy container's address if that address is ever fixed.

**I4.** `rkyv` 0.7.46 (RUSTSEC-2026-0235: out-of-bounds reads in archives with `Rc`/`Arc`) and the
unmaintained `ttf-parser` 0.25.1 come in through `lightningcss` and the glyph check: build-time
only, never fed untrusted input, not in the server or the WebAssembly modules.

**I5.** hyper accepts LF-only line endings, as RFC 9112 allows. No request-smuggling difference
showed up with the proxies in front (below); noted for whoever changes the proxy chain.

**I6.** `/_rum` counts what browsers report, so anyone can send well-formed beacons: Web Vitals,
event counts, and the A/B test's exposures and conversions. Each network gets a small token
bucket, the values must be in range and name our own routes and variants, and the A/B result is
computed once at the planned sample size — but someone with many networks could still nudge the
totals. Treat them as an honest crowd's measurements, not evidence against an adversary.

## What held

- **Headers on every response,** errors and redirects included: CSP (`default-src 'none'`,
  hashes, Trusted Types), COOP, COEP, CORP, Permissions-Policy, Referrer-Policy,
  `X-Content-Type-Options`; HSTS over our own TLS only. No `Server`/`X-Powered-By`, no cookies,
  no CORS headers, `no-store` on every dynamic endpoint, correct `Vary`.
- **TLS (LAN preview):** TLS 1.2 and older refused; TLS 1.3 with X25519MLKEM768 first, then
  X25519 and P-256; no 0-RTT, no compression; ALPN h2/http1.1 on TCP and h3 on QUIC.
- **Request smuggling:** CL+TE, duplicate `Content-Length`, obfuscated or listed
  `Transfer-Encoding`, obs-fold, a space before the colon, bad chunk sizes and HTTP/0.9 all get
  `400`; through the edge and Caddy as well as directly.
- **Paths:** traversal (`..`, `%2e%2e`, `//`), case, NUL, internal file names (`manifest.json`,
  `text/…`, `.br` siblings, `data/`, `ctf.key`, `.env`, `.git`) — nothing outside the route table
  is ever served, and 404 pages never echo the path. Outside `/mintaka` the server answers a bare
  404. No open redirect (`//host`, `/\host`).
- **Methods:** TRACE, OPTIONS, PUT, DELETE, PATCH get `405` with `Allow`; nothing is reflected.
- **Limits:** URL 100 kB → 414; 200 headers → 431; bodies over the limit → 413, a 1 MiB chunked
  body after 48 KiB; JSON nested 5,000 deep → 400; slow-header and silent connections closed after
  10–12 s.
- **APIs:** forged console-challenge tokens refused (constant-time MAC check); briefs require a
  valid single-use proof of work or the stricter no-JS rate limit; Telegram notifications carry no
  text a visitor wrote; beacons from other sites refused.
- **Behind the edge:** the visitor's network is taken per request from a header Caddy sets
  (`X-Mintaka-Client-Ip`, from the edge's `X-Forwarded-For`, last address). Forging
  `X-Forwarded-For` or `X-Mintaka-Client-Ip` from outside didn't move a client out of its
  rate-limit bucket, while a second network kept its own (probe: 20 × 200, then 429 for this
  network; 200 for another). `/_proof` reports the edge instead of TLS it can't see.
- **Through the edge** the provider's OpenResty answers malformed requests itself (400/501 or a
  closed connection for the smuggling probes, its own 414/431/413 limits) and holds slow or silent
  connections for 30 s and more without forwarding anything — its timeouts, not ours, and our
  server never sees an incomplete request. The cross-site form post is refused (403) end to end.
- **The Telegram bot** (added after the probes; code review, plus its tests against a stand-in
  Bot API): start codes are HMAC-signed with their own key file and checked in constant time,
  expire after 7 days and can't take over a brief another chat holds; honeypot submissions get a
  decoy link the bot ignores; only private chats and the configured team chat are heard, with
  per-chat limits; the chat memory is 0600, written atomically and holds no message text; the
  token travels only in request URLs and errors are logged without them; the test-only API
  override accepts plain HTTP to this machine alone.
- **No secrets** in the repository or its history (the PEM markers found are test code), none in
  responses, and no visitor addresses in the server's log.

## Repeat it

```sh
python3 -I deploy/probe.py 192.168.1.156 8761 ""          # LAN preview, plain
python3 -I deploy/probe.py 172.20.0.1 8790 /mintaka       # on the server: production upstream
```
