# Benchmark: static page, constant request rate

What it measures: `GET /en/` of the test fixture (`crates/server/tests/fixtures/dist`, brotli
variant, 281 B on the wire) served by the release build, at a **fixed request rate** with oha's
latency correction — so a stalled request is counted from when it *should* have been sent and
coordinated omission does not hide queueing.

Two views of the same run:

* **client** — oha's end-to-end latency over loopback (TCP, HTTP/1.1 keep-alive, oha's own work);
* **server** — `app` from `Server-Timing`, as aggregated by the server itself in `/_telemetry`
  (p50/p99/p99.9 over the last 10 s, HDR histogram).

## Machine (2026-10-27)

| | |
|---|---|
| CPU | 13th Gen Intel Core i7-13620H (6 P + 4 E cores, 16 threads), laptop, default governor |
| RAM | 16 GB |
| OS | Arch Linux, kernel 7.2.11-zen1 |
| Toolchain | rustc 1.99.1, release profile: `lto = "fat"`, `codegen-units = 1`, `panic = "abort"` |
| Load generator | oha 1.14.0, **same machine** (client and server share the 16 threads) |

Access logging was on (`RUST_LOG=info`, every request written to a file through the
non-blocking writer; all ~700 k lines of both runs were written, none dropped).

## Commands

```sh
cargo build -p server --release
./target/release/mintaka-server --dist crates/server/tests/fixtures/dist \
    --listen 127.0.0.1:18080 --data-dir /path/to/scratch/data > server.log 2>&1 &

# server-side view, one JSON tick per second
curl -sN http://127.0.0.1:18080/_telemetry > telemetry.txt &

# 20 000 requests/s for 10 s over 64 connections
oha -z 10s -q 20000 -c 64 --latency-correction --no-tui \
    -H 'accept-encoding: br' -H 'user-agent: Mozilla/5.0 (oha benchmark)' \
    http://127.0.0.1:18080/en/

# 50 000 requests/s for 10 s over 128 connections
oha -z 10s -q 50000 -c 128 --latency-correction --no-tui \
    -H 'accept-encoding: br' -H 'user-agent: Mozilla/5.0 (oha benchmark)' \
    http://127.0.0.1:18080/en/
```

(The browser-like `User-Agent` matters: `curl/…` would get the curl-mode text instead.)

## Results

| Rate (fixed) | Achieved | 200 OK | client p50 | client p99 | client p99.9 | client p99.99 |
|---|---|---|---|---|---|---|
| 20 000 /s, 64 conns | 19 933 /s | 199 964 | 0.147 ms | 0.329 ms | 0.865 ms | 1.657 ms |
| 50 000 /s, 128 conns | 49 990 /s | 499 967 | 0.212 ms | 0.611 ms | 1.394 ms | 9.197 ms |

No errors other than oha's own "aborted due to deadline" for the 1 (20 k) and 4 (50 k) requests
still in flight when the 10 s window closed.

Server-side, from the `/_telemetry` ticks during the steady part of each run:

| Rate | `app` p50 | `app` p99 | `app` p99.9 | process CPU | RSS |
|---|---|---|---|---|---|
| 20 000 /s | 4 µs | 19 µs | 53–55 µs | 54–55 % of one core | 16 MB |
| 50 000 /s | 3 µs | 25–26 µs | 71–77 µs | 92–94 % of one core | 19 MB |

Single requests on a fresh connection (curl, release build) show `app;dur=0.016`–`0.025` ms
(16–25 µs: first request on a connection, cold caches); five requests on one keep-alive
connection showed `0.019`, `0.011`, `0.004`, `0.002`, `0.002` ms. Target was `app` < 50 µs for
static routes.

The slowest client samples (≈ 10–12 ms) are the initial connection burst: oha's `DNS+dialup`
slowest was 9.7 ms (20 k) and 12.0 ms (50 k) — 64/128 connections opened at once on a busy
machine — not request handling.

## Reading the numbers

* The gap between client p50 (≈ 150 µs) and server p50 (4 µs) is the loopback TCP stack, two
  context switches and the load generator itself, all on the same CPUs. On a real network the
  round trip dominates by orders of magnitude.
* The tail grows at 50 k/s mostly because oha and the server compete for the same cores; the
  server's own p99.9 stays under 0.1 ms.
* CPU is from `/proc/self/stat` (10 ms clock ticks, so ±1 % per one-second sample).
