## S6 · Live load (web/instruments/live.ts).
## This server's own traffic, one tick per second from GET /_telemetry (Server-Sent Events).
## Aggregates only. Values are filled in live — never put an example number here.

live-connecting = connecting
live-live = live
live-reconnecting = reconnecting
live-offline = offline
live-paused = paused
live-unsupported = This browser can't open a Server-Sent Events stream, so live load isn't available here.
live-rps = Requests per second
live-p50 = Latency, median
live-p99 = Latency, 99th percentile
live-p999 = Latency, 99.9th percentile
live-cpu = CPU, share of one core
live-rss = Memory in use (RSS)
live-conns = Open connections
live-total = Requests since start
live-idle = no requests
live-range = { $min } to { $max } over the last minute
live-dots = { $n ->
    [1] Each dot is one request.
   *[other] Each dot is { $n } requests.
}
live-server = Server { $region } · { $n ->
    [one] { $n } core
   *[other] { $n } cores
} · build { $build }
live-window = Latency covers the last 10 seconds; the lines show the last 60.
live-foot = Aggregates only: no IP addresses, paths or user agents ever reach this stream.
live-said-live = Live data connected.
live-said-lost = Live data interrupted. Trying again.
live-said-offline = Live data unavailable. Trying again in the background.
