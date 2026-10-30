# Proof console (web/hud/index.ts) and the footer's live line: the pill, the live line and the
# panel's frame. The panel's rows are in lazy/hud.ftl, fetched when the panel first opens.
# Every { $variable } is measured on this page view; the console fills it in.

hud-region = Proof console
hud-pill-label = Proof console:
hud-pill-server = server time
hud-pill-weight = page weight
hud-pill-pq = (post-quantum TLS)
hud-pill-dev = HTTP · dev
hud-pill-edge = HTTPS · edge

hud-id = Proof
hud-title = This page, measured
hud-close = Close
hud-raw = View raw headers

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
hud-foot-edge = HTTPS at the hosting edge
