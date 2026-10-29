# Benchmark run · 2026-10-29-dev-laptop

Written by `bench/run.sh`; method and how to read it in `bench/README.md`. The load generator ran
on the **same machine** as the server, so both compete for the same cores.

## Machine

| | |
|---|---|
| CPU | 13th Gen Intel(R) Core(TM) i7-13620H (16 logical CPUs), governor powersave |
| RAM | 15.3 GiB (MemTotal) |
| OS | Linux 7.2.11-zen1-1-zen |
| Toolchain | rustc 1.99.1 (58cf8f9c3 2026-10-13) (Arch Linux rust 1:1.99.1-1), release profile (`lto = "fat"`, `codegen-units = 1`, `panic = "abort"`) |
| Load generator | oha 1.14.0, 64 connections, 10 s per rate, latency correction on |
| Page | `GET /en/` from `dist`, brotli, 30,242 B per response |

## Client view (oha, end to end over loopback)

| Protocol | Rate (fixed) | Achieved | 200 OK | p50 | p99 | p99.9 | p99.99 | Errors |
|---|---|---|---|---|---|---|---|---|
| HTTP/1.1, plain | 20,000 /s | 19,990 /s | 199,976 | 0.163 ms | 0.386 ms | 0.812 ms | 1.879 ms | none |
| HTTP/1.1, plain | 50,000 /s | 49,990 /s | 499,950 | 0.269 ms | 0.676 ms | 1.374 ms | 5.700 ms | aborted due to deadline: 1 |
| HTTP/2, TLS 1.3 | 20,000 /s | 20,004 /s | 199,983 | 0.464 ms | 1.321 ms | 2.750 ms | 3.831 ms | aborted due to deadline: 61 |
| HTTP/2, TLS 1.3 | 50,000 /s | 49,984 /s | 499,843 | 1.056 ms | 34.001 ms | 41.492 ms | 42.774 ms | aborted due to deadline: 64 |

## Server view (`app` from Server-Timing, aggregated by the server in /_telemetry)

Ranges over the one-second ticks of each run's steady part (its first two seconds skipped);
each tick's percentiles cover the last 10 s.

| Protocol | Rate | `app` p50 | `app` p99 | `app` p99.9 | process CPU | RSS |
|---|---|---|---|---|---|---|
| HTTP/1.1, plain | 20,000 /s | 4 µs | 23 µs–32 µs | 55 µs–64 µs | 60 %–64 % | 22 MB–23 MB |
| HTTP/1.1, plain | 50,000 /s | 4 µs | 15 µs–20 µs | 53 µs–80 µs | 121 %–134 % | 24 MB |
| HTTP/2, TLS 1.3 | 20,000 /s | 4 µs–8 µs | 15 µs–20 µs | 41 µs–75 µs | 128 %–177 % | 26 MB |
| HTTP/2, TLS 1.3 | 50,000 /s | 7 µs–8 µs | 21 µs–28 µs | 46 µs–99 µs | 397 %–480 % | 26 MB |
