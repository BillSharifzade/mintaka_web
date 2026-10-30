# Proof console (web/hud/index.ts): the panel's rows — labels, sources, values, real visitors and
# the A/B test — in a lazy bundle fetched when the panel first opens
# (/assets/strings/hud.<lang>.<hash>.json). The pill, the footer's live line and the panel's frame
# stay inline in ../hud.ftl. Every { $variable } is measured on this page view.
# { $n } is a count, read with tp() / pluralKey(): select on it for plurals.

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
hud-k-visitors = Real visitors to this server
hud-src-server = Server-Timing header of this page (app): from request parsed to response ready
hud-src-weight = Resource Timing: transferSize of every request so far, summed
hud-src-third = Resource Timing: requests grouped by origin
hud-src-cookies = document.cookie, plus the Set-Cookie count from /_proof
hud-src-tls = /_proof: TLS version, cipher, key exchange and HTTP version our server negotiated
hud-src-edge = /_proof says the request came through the hosting edge; the HTTP version is your browser's own (Navigation Timing)
hud-src-headers = A HEAD request to this page, read in your browser
hud-src-lcp = PerformanceObserver: largest-contentful-paint
hud-src-inp = Event Timing: the longest interaction measured on this page so far
hud-src-cls = PerformanceObserver: layout-shift, largest session window
hud-src-tier = The hero engine, on this device
hud-src-visitors = GET /_rum: the Web Vitals visitors' browsers sent this server since it started, and the A/B test's counts per variant, kept only as totals in memory. No cookies, IP addresses or browser details.
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
hud-edge = HTTPS to our hosting provider's edge proxy · { $http }. Our server sits behind it and can't see the TLS details.
hud-sent = sent
hud-missing = missing
hud-cookies-ours = set by this site
hud-cookies-foreign = not set by this site
hud-https-only = HTTPS only
hud-err-proof = Couldn't reach /_proof. Close and reopen this panel to try again.
hud-err-head = Couldn't read the headers ({ $reason }). Close and reopen this panel to try again.
hud-offline = You're offline, and this row asks our server. Reopen the panel once you're connected.
hud-offline-copy = Not timed: this copy of the page came from your browser's offline cache, not from our server.
hud-visitors-since = { $n ->
    [one] Since { $date }: { $n } page view measured
   *[other] Since { $date }: { $n } page views measured
}
hud-visitors-p75 = 75th percentile of { $n }
hud-visitors-few = { $n ->
    [one] { $n } sample so far, too few for a 75th percentile (it needs { $min })
   *[other] { $n } samples so far, too few for a 75th percentile (it needs { $min })
}
hud-visitors-declined = Your browser asks sites not to measure it (Global Privacy Control or Do Not Track), so this page view isn't counted.
hud-visitors-error = Couldn't reach /_rum ({ $reason }). Close and reopen this panel to try again.

# The A/B test under "Real visitors" (web/hud/experiment.ts), while one runs. { $max } is the
# planned page views per variant, fixed before the test began.
hud-ab-lead = A/B test { $id }: our server picks one variant per page load, at random, and counts how many page views lead to { $metric }. It stops at { $max } page views of each.
hud-ab-control = control
hud-ab-yours = this page view
hud-ab-views = { $max ->
    [one] { $n } of { $max } page view
   *[other] { $n } of { $max } page views
}
hud-ab-rate = { $n } { $metric } ({ $rate })
hud-ab-wait = No verdict until every variant has all its page views: a difference read early is too often chance.
hud-ab-better = Better than the control (p = { $p }).
hud-ab-worse = Worse than the control (p = { $p }).
hud-ab-same = No clear difference from the control (p = { $p }).
hud-ab-done = Complete: counting has stopped. A difference is clear when p is below { $alpha }, which is { $family } shared by { $k } comparisons.
