# S5 · This page's armour (web/instruments/armour.ts).
# A live scan in the visitor's browser: the headers of a HEAD request to this page, and /_proof.

armour-status-scanning = scanning
armour-status-done = live scan
armour-scanning = Reading the headers this page was served with…
armour-summary = { $ok } of { $n ->
    [one] { $n } security header
   *[other] { $n } security headers
} in place on this page's response.
armour-summary-http = HSTS is only sent over HTTPS, so it isn't counted on this development server.
armour-head-src = Read in your browser from a HEAD request to this page, just now.
armour-head-error = Couldn't read this page's headers ({ $reason }). Reload the page to try again.

armour-caption = Security headers this page was served with
armour-col-header = Header
armour-col-status = Status
armour-col-value = Value
armour-st-ok = in place
armour-st-weak = weak
armour-st-missing = missing
armour-st-na = HTTPS only
armour-not-sent = not sent
armour-https-only = sent only over HTTPS

armour-why-content-security-policy = Stops injected scripts: only our own files may run, and Trusted Types guard every HTML sink.
armour-why-cross-origin-opener-policy = Keeps windows from other sites out of this page's process.
armour-why-cross-origin-embedder-policy = Loads only resources that agree to be embedded, which completes cross-origin isolation.
armour-why-cross-origin-resource-policy = Stops other sites from embedding our files.
armour-why-permissions-policy = Switches off browser features this page doesn't use, such as the camera, microphone and location.
armour-why-referrer-policy = Doesn't tell other sites which page you came from.
armour-why-x-content-type-options = Stops the browser from guessing file types, so data is never run as a script.
armour-why-strict-transport-security = Tells your browser to use only HTTPS for this site, even when a link says http.

armour-tls-title = Connection
armour-tls-version = TLS version
armour-tls-cipher = Cipher suite
armour-tls-group = Key exchange
armour-pq-badge-hybrid = post-quantum
armour-pq-badge-pq = post-quantum
armour-pq-badge-classical = classical
armour-pq-badge-plain = no TLS
armour-pq-hybrid = Your connection used { $group } — a hybrid of classical and post-quantum key exchange.
armour-pq-pq = Your connection used { $group } — post-quantum key exchange.
armour-pq-classical = Your browser negotiated { $group } (classical). We offer post-quantum key exchange to every browser that supports it.
armour-pq-plain = Development server: plain HTTP, no TLS.
armour-tls-src = From /_proof: what our server negotiated with your browser for this connection.
armour-tls-error = Couldn't reach /_proof ({ $reason }), so the connection details are missing. Reload to try again.

armour-iso-title = Isolation
armour-coi = Cross-origin isolated
armour-coi-yes = yes
armour-coi-no = no
armour-coi-yes-line = COOP and COEP are working: this page runs in its own browsing context group, which also unlocks SharedArrayBuffer.
armour-coi-no-line = This page isn't cross-origin isolated, even though COOP and COEP were sent.
armour-coi-no-headers-line = This response didn't carry both COOP and COEP, so the page isn't cross-origin isolated.
armour-tt = Trusted Types
armour-tt-enforced = enforced
armour-tt-open = not enforced
armour-tt-unsupported = not supported
armour-tt-enforced-line = We tried to write a plain string into innerHTML of a throwaway element. Your browser blocked it.
armour-tt-open-line = We tried to write a plain string into innerHTML of a throwaway element, and it went through even though our policy requires Trusted Types.
armour-tt-open-policy-line = We tried to write a plain string into innerHTML of a throwaway element, and it went through: this response's Content Security Policy doesn't require Trusted Types.
armour-tt-unsupported-line = Your browser doesn't support Trusted Types. The Content Security Policy still blocks inline scripts.
armour-iso-src = Checked in your browser when this panel loaded.
