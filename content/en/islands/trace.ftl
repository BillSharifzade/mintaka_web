## S6 · Follow one request (web/instruments/trace.ts).
## The visitor's own page request, hop by hop: Navigation Timing in their browser plus our
## Server-Timing header. Durations are filled in live — never put an example number here. The
## hops' explainers are in lazy/trace.ftl, fetched when a hop is first opened.

trace-status = measured
trace-status-none = not available
trace-unsupported = This browser doesn't report Navigation Timing, so the hops of this page request can't be shown here.
trace-cached = Your browser showed this page from its own cache, so no request crossed the network. Send a fresh request to follow one.
trace-page = Your page request
trace-latest = Latest request
trace-size = { $size } over the network
trace-total = { $total } in total
trace-again = Send another request
trace-failed = The request didn't get through. Check your connection and try again.
trace-offline = You're offline, so the request can't reach our server. Send it again once you're connected.
trace-done = Request { $n }: { $total } in total, { $app } of it inside our server.
trace-done-short = Request { $n }: { $total } in total.
trace-gap = The other { $ms } went to your browser checking its cache and queueing the request.

trace-dns = DNS lookup
trace-dns-short = DNS
trace-tcp = Connection
trace-tcp-short = TCP
trace-tls = TLS 1.3 handshake
trace-tls-short = TLS
trace-tls-server = our side { $ms }
trace-net = Network, there and back
trace-net-short = NET
trace-krtt = kernel RTT { $ms }
trace-qrtt = QUIC RTT { $ms }
trace-app = Inside our server
trace-app-short = APP
trace-app-split = route { $route } · render { $render }
trace-ttfb = Request to first byte
trace-ttfb-short = WAIT
trace-down = Download
trace-down-short = BODY

trace-zero-reused = connection reused
trace-zero-cached = answer already cached
trace-zero-plain = plain HTTP, no TLS (development)
trace-zero-quic = part of the QUIC handshake
