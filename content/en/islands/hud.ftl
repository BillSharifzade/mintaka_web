# Proof console (web/hud/index.ts) and the footer's live line.
# Every { $variable } is measured on this page view; the console fills it in.
# { $n } is a count, read with tp() / pluralKey(): select on it for plurals.

hud-region = Proof console
hud-pill-label = Proof console:
hud-pill-server = server time
hud-pill-weight = page weight
hud-pill-pq = (post-quantum TLS)
hud-pill-dev = HTTP · dev

hud-id = Proof
hud-title = This page, measured
hud-close = Close
hud-raw = View raw headers
hud-pending = not measured yet
hud-na = not available in this browser
hud-not-reported = not reported by the server

hud-k-server = Server time
hud-k-weight = Page weight
hud-k-third = Third-party requests
hud-k-cookies = Cookies
hud-k-tls = Connection
hud-k-headers = Security headers
hud-k-lcp = Largest Contentful Paint (LCP)
hud-k-inp = Slowest interaction so far (INP)
hud-k-cls = Cumulative Layout Shift (CLS)
hud-k-tier = Hero render tier

hud-src-server = Server-Timing header of this page (app): from request parsed to response ready
hud-src-weight = Resource Timing: transferSize of every request so far, summed
hud-src-third = Resource Timing: requests grouped by origin
hud-src-cookies = document.cookie, plus the Set-Cookie count from /_proof
hud-src-tls = /_proof: TLS version, cipher, key exchange and HTTP version our server negotiated
hud-src-headers = A HEAD request to this page, read in your browser
hud-src-lcp = PerformanceObserver: largest-contentful-paint
hud-src-inp = Event Timing: the longest interaction measured on this page so far
hud-src-cls = PerformanceObserver: layout-shift, largest session window
hud-src-tier = The hero engine, on this device

hud-requests = { $n ->
    [one] { $n } request
   *[other] { $n } requests
}, { $cached } from cache
hud-inp-none = no interactions yet
hud-tier-a = A · WebGPU
hud-tier-b = B · WebGL2
hud-tier-c = C · Canvas 2D
hud-tier-d = D · static poster
hud-tier-none = not reported yet
hud-plain = Plain HTTP (development) · { $http }
hud-sent = sent
hud-missing = missing
hud-cookies-ours = set by this site
hud-cookies-foreign = not set by this site
hud-https-only = HTTPS only
hud-err-proof = Couldn't reach /_proof. Close and reopen this panel to try again.
hud-err-head = Couldn't read the headers ({ $reason }). Close and reopen this panel to try again.

hud-foot-lead = This page:
hud-foot-server = { $value } server
hud-foot-cookies = { $n ->
    [one] { $n } cookie
   *[other] { $n } cookies
}
hud-foot-third = { $n ->
    [one] { $n } third-party request
   *[other] { $n } third-party requests
}
hud-foot-pq = { $tls } + post-quantum
hud-foot-dev = plain HTTP (development)
