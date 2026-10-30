# Proof console (web/hud/index.ts) and the footer's live line: the pill, the live line and the
# panel's frame. The panel's rows are in lazy/hud.ftl, fetched when the panel first opens.
# Every { $variable } is measured on this page view; the console fills it in.
# A Tajik noun after a numeral stays singular (3 дархост), so counts need no selector.

hud-region = Консоли далелҳо
hud-pill-label = Консоли далелҳо:
hud-pill-server = вақти сервер
hud-pill-weight = вазни саҳифа
hud-pill-pq = (TLS-и постквантӣ)
hud-pill-dev = HTTP · таҳия
hud-pill-edge = HTTPS · канори хостинг

hud-id = Далелҳо
hud-title = Ин саҳифа дар рақамҳо
hud-close = Пӯшидан
hud-raw = Дидани сарлавҳаҳои хом

# The A/B test under "Real visitors" (web/hud/experiment.ts), while one runs. { $max } is the
# planned page views per variant, fixed before the test began. { $metric } is an event id (code).

hud-foot-lead = Ин саҳифа:
hud-foot-server = { $value } дар сервер
hud-foot-cookies = { $n } cookie
hud-foot-third = { $n } дархости тарафи сеюм
hud-foot-pq = { $tls } + постквантӣ
hud-foot-dev = HTTP-и оддӣ (таҳия)
hud-foot-edge = HTTPS дар канори хостинг
