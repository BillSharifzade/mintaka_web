## S6 · Follow one request (web/instruments/trace.ts): what each hop's explainer says — a lazy
## bundle (/assets/strings/trace.<lang>.<hash>.json), fetched the first time someone opens a hop.

trace-dns-about = Your device asked a DNS resolver for our server's IP address. Resolvers and browsers cache the answer, so a repeat visit usually skips this step.
trace-tcp-about = Your browser and our Linux kernel exchanged SYN, SYN-ACK and ACK. The kernel completes this handshake on its own and queues the connection until our server accepts it.
trace-tcp-about-edge = Your browser and our hosting provider's edge proxy exchanged SYN, SYN-ACK and ACK. The proxy holds this connection, not our server; it passes your request on inside the data centre.
trace-tls-about = One round trip to agree on keys — with the post-quantum hybrid X25519MLKEM768 when your browser offers it — and for our server to prove it holds the key to its certificate.
trace-tls-about-edge = One round trip to agree on keys with our hosting provider's edge proxy, which holds this domain's certificate. Encryption ends there, before the request reaches our server.
trace-net-about = Your request crossed the network, waited in our network card's queue, rose through the kernel's TCP stack into a socket, and the first byte of the answer travelled back. Mostly distance: about one round trip.
trace-net-about-edge = Your request crossed the network to the provider's edge, went through two proxies inside the data centre to our server, and the first byte of the answer travelled back the same way.
trace-app-about = A Tokio task parsed the request, looked up the route and picked the pre-rendered, pre-compressed page from memory. No disk, no database, no template on the way.
trace-app-about-ping = A Tokio task parsed the request, matched the route and answered 204 No Content straight away.
trace-ttfb-about = From your browser sending the request to the first byte of the answer. This browser doesn't pass on our Server-Timing header, so network and server time can't be told apart here.
trace-down-about = The rest of the answer streamed to your browser. Pages are compressed once, at build time, so there are fewer bytes to carry and no work to do per request.
trace-down-about-ping = A 204 answer has no body, so only the end of the response is left to arrive.
