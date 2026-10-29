+++
slug = "serving-a-page-in-under-a-millisecond"
title = "How this website serves a page in under a millisecond"
pillar = "highload"
date = 2026-10-29
summary = "The server behind this site reads every file into memory at startup, with its compressed variants and complete response headers already built, so a request costs a lookup, a header parse and two clones of things built in advance. Under a fixed 20,000 requests per second on a laptop, its own time for the 28.8 kB home page was 4–5 µs at the median and 22–30 µs at p99. The load generator, on the same machine, saw 0.19 ms. This post is about both numbers, and why they differ."
repo = "crates/server"
+++

## The problem

This site claims that every number on it is measured. The first number a visitor's browser can measure is how long our server takes to answer, because the server sends it in a `Server-Timing` header with every response. If that number is large, the claim is embarrassing. If it is small, it should be small for reasons we can explain line by line.

The budget in our spec is one millisecond of server time per page. A millisecond is generous: light in fibre needs at least 47.1 ms for a round trip between Dushanbe and Frankfurt (the table in the physics section of the home page shows the calculation), so a server that answers in a millisecond already spends a fiftieth of the unavoidable delay. We wanted the server's share to be a rounding error, and we wanted to measure it honestly.

## Method

### Do the work before the first request

A static site has no reason to touch the disk, compress anything or assemble headers while a visitor waits. The generator writes `dist/manifest.json`, which lists every route with its file, content type, cache policy and precompressed variants (brotli at quality 11 and gzip at level 9, each kept only if it saves at least 5 %). At startup, [`Site::load`](repo:crates/server/src/assets.rs) reads the manifest and every file it names, and serving a page never touches the file system again.

For every route and every variant, it builds the complete header map of a `200` and of a `304` up front: type, length, cache policy, `ETag`, `Vary`, encoding and language. The `ETag` is the first 128 bits of the SHA-256 of the uncompressed bytes, with `-br` or `-gz` appended, because RFC 9110 requires a strong validator to differ between representations. The security headers, including the Content Security Policy with the hashes of our inline styles, are built once for plain HTTP and once for TLS.

What is left for a request is small: find the route, read `Accept-Encoding` (brotli before gzip before identity, honouring `q=0`, looking at no more than 32 items), compare `If-None-Match` with the variant's `ETag`, and clone what was built at startup. Nothing is compressed at request time.

### Measuring the server from inside

The connection layer takes a timestamp at the moment hyper hands us a parsed request head. After the handler, `routes::finish` applies the security headers, takes a second timestamp, writes `app;dur=…` (that interval), `route` and `render` into `Server-Timing`, and records `app` in an HDR histogram. The histograms are sharded by worker thread so the cores don't share one lock, keep two significant figures (exact below 256 µs), and are folded once a second into a ten-second window that [`/_telemetry`](repo:crates/server/src/telemetry.rs) streams as p50, p99 and p99.9.

Be precise about what `app` leaves out: reading and parsing the request head (before the first timestamp), writing the body to the socket, TLS encryption, and the kernel. It is the server's own decision time, nothing more. That is why the client sees a larger number, and why we report both.

### Load at a fixed rate

Most quick benchmarks are closed loops: each connection sends its next request when the previous answer arrives. When the server stalls, such a client stops sending, so the stall produces one slow sample per connection instead of the hundreds of requests that would have queued behind it in real life. Gil Tene named this coordinated omission. We use oha at a fixed rate (`-q`) with `--latency-correction`, which measures every request from the moment it was due, not from the moment it was finally sent.

## The mathematics

At a fixed rate $r$, request $i$ is due at

$$
t_i = t_0 + \frac{i}{r}
$$

and its latency is counted as $L_i = c_i - t_i$, where $c_i$ is when its response completed. A closed-loop tool measures $c_i - s_i$ instead, with $s_i$ the moment it actually sent the request; when a stall delays $s_i$, the waiting disappears from the data.

Little's law ties rate, time and concurrency together: the average number of requests in a system is

$$
N = r W
$$

for an average time $W$ in it. With $r = 20000$ per second and oha's mean latency of 0.210 ms, there were about 4.2 requests in flight among 64 connections. Inside the server the same law gives a bound: if every request had taken the p99 time of 30 µs, there would be 0.6 requests in the server on average, so for at least 40 % of the time there would be none.

## The code

The bytes are leaked on purpose. They live as long as the process, so they can be `'static`:

```rust include="crates/server/src/assets.rs" from="/// Process-lifetime bytes"
```

A `Bytes` made from a static slice owns nothing. Cloning it for a response copies a small handle; there is no reference count for sixteen threads to fight over. Header values get the same treatment through `HeaderValue::from_static`, so cloning a prebuilt header map copies pointers, not strings.

Once the router has found the route, serving it is this:

```rust include="crates/server/src/routes/static_files.rs" from="/// Picks the encoding, answers"
```

And this is where the clock starts, in the hyper service of each connection:

```rust include="crates/server/src/conn.rs" from="fn call(&self, req: Request<Incoming>)"
```

Every transport ends in the same function, HTTP/3 included, so every response gets the same headers and the same second timestamp:

```rust include="crates/server/src/conn.rs" from="pub(crate) async fn respond<B>"
```

## The benchmark

Measured on 2026-10-29 on the laptop described in [the benchmark notes](repo:bench/README.md): a 13th-generation Intel Core i7-13620H (6 performance and 4 efficiency cores, 16 threads), 16 GB of RAM, Arch Linux with kernel 7.2.11-zen1, the `powersave` governor, rustc 1.99.1 with the release profile (`lto = "fat"`, `codegen-units = 1`). The load generator, oha 1.14.0, ran on the same machine, sharing the 16 threads with the server and with a desktop session (load average 2–3.5 before each run). Access logging was on.

```sh title="The commands"
cargo xtask build --release && cargo build -p server --release
RUST_LOG=info ./target/release/mintaka-server --dist dist --data-dir /tmp/mintaka-bench \
    --listen 127.0.0.1:18880 --tls-self-signed --tls-listen 127.0.0.1:18843 > server.log &
curl -sN http://127.0.0.1:18880/_telemetry > telemetry.txt &
oha -z 10s -q 20000 -c 64 --latency-correction --no-tui \
    -H 'accept-encoding: br' -H 'user-agent: Mozilla/5.0 (oha benchmark)' \
    http://127.0.0.1:18880/en/
```

The browser-like `User-Agent` matters: `curl/…` gets the plain-text curl mode. The page was the home page of that day's build: 28,773 bytes of brotli per response.

| Run | Achieved | p50 | p99 | p99.9 | p99.99 |
|---|---:|---:|---:|---:|---:|
| Home page, 20,000/s, 64 connections | 19,996/s | 0.187 ms | 0.571 ms | 1.107 ms | 3.187 ms |
| Home page, 50,000/s, 128 connections | 49,988/s | 0.356 ms | 1.399 ms | 3.350 ms | 14.958 ms |
| 281-byte test page, 20,000/s, 64 connections | 19,997/s | 0.136 ms | 0.307 ms | 0.694 ms | 1.606 ms |

Table: Client latency from oha, with latency correction, 10 s per run. All responses were 200 except 1–4 per run that oha cut off at its deadline.

| Run | `app` p50 | `app` p99 | `app` p99.9 | Process CPU |
|---|---:|---:|---:|---:|
| Home page, 20,000/s | 4–5 µs | 22–30 µs | 49–61 µs | 58–100 % |
| Home page, 50,000/s | 4–5 µs | 17–21 µs | 81–125 µs | 121–179 % |
| 281-byte test page, 20,000/s | 4 µs | 28–29 µs | 54–55 µs | 47–49 % |

Table: Server time from the `/_telemetry` ticks of the steady seconds of the same runs (ranges over those ticks). CPU is user plus system time as a share of one core.

The last row repeats the test page of our earlier notes, which measured 0.147 ms at p50 and 4 µs inside the server on 27 October; the machine behaved the same way two days later. Single requests with curl show the same picture: 12–21 µs of `app` on a fresh connection, then 18, 10, 4, 3 and 2 µs for five requests on one kept-alive connection, as caches warm up.

One run is missing from the tables, and it is the most instructive. Our first 20,000/s run on the home page measured a client p99 of 11.9 ms, twenty times the repeat, and the server's own p99.9 rose to 1.77 ms in one second. `app` is wall-clock time, so when another process takes the core in the middle of a request, the server's histogram shows it too. That is the value of measuring on both sides: they agreed that the stall was on the machine, not in the request path.

## Why the two medians differ

At 20,000/s, the client's median was 187 µs and the server's was 4–5 µs. The difference is everything `app` leaves out: oha building and sending the request, two trips through the loopback TCP stack, hyper reading and parsing the head, copying 28.8 kB into a socket buffer and out again, oha parsing the response, and the context switches between them, with client and server on the same cores. The CPU column says the same thing another way: 58–100 % of a core at 20,000/s is 29–50 µs of process time per request, of which `app` accounts for 4–5. On a real network none of this matters next to the round trip; it matters here because it tells you where to look next.

What this benchmark does not show: TLS (it ran over plain HTTP/1.1), HTTP/2, a real network, or production hardware. The numbers come from a laptop, and we say so.
