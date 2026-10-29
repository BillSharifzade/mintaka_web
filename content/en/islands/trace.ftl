## S6 · Follow one request (web/instruments/trace.ts).
## The visitor's own page request, hop by hop: Navigation Timing in their browser plus our
## Server-Timing header. Durations are filled in live — never put an example number here.

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
trace-dns-about = Your device asked a DNS resolver for our server's IP address. Resolvers and browsers cache the answer, so a repeat visit usually skips this step.
trace-tcp = Connection
trace-tcp-short = TCP
trace-tcp-about = Your browser and our Linux kernel exchanged SYN, SYN-ACK and ACK. The kernel completes this handshake on its own and queues the connection until our server accepts it.
trace-tls = TLS 1.3 handshake
trace-tls-short = TLS
trace-tls-about = One round trip to agree on keys — with the post-quantum hybrid X25519MLKEM768 when your browser offers it — and for our server to prove it holds the key to its certificate.
trace-tls-server = our side { $ms }
trace-net = Network, there and back
trace-net-short = NET
trace-net-about = Your request crossed the network, waited in our network card's queue, rose through the kernel's TCP stack into a socket, and the first byte of the answer travelled back. Mostly distance: about one round trip.
trace-krtt = kernel RTT { $ms }
trace-app = Inside our server
trace-app-short = APP
trace-app-about = A Tokio task parsed the request, looked up the route and picked the pre-rendered, pre-compressed page from memory. No disk, no database, no template on the way.
trace-app-about-ping = A Tokio task parsed the request, matched the route and answered 204 No Content straight away.
trace-app-split = route { $route } · render { $render }
trace-ttfb = Request to first byte
trace-ttfb-short = WAIT
trace-ttfb-about = From your browser sending the request to the first byte of the answer. This browser doesn't pass on our Server-Timing header, so network and server time can't be told apart here.
trace-down = Download
trace-down-short = BODY
trace-down-about = The rest of the answer streamed to your browser. Pages are compressed once, at build time, so there are fewer bytes to carry and no work to do per request.
trace-down-about-ping = A 204 answer has no body, so only the end of the response is left to arrive.

trace-zero-reused = connection reused
trace-zero-cached = answer already cached
trace-zero-plain = plain HTTP, no TLS (development)
trace-zero-quic = part of the QUIC handshake
