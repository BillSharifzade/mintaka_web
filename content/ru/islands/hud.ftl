# Proof console (web/hud/index.ts) and the footer's live line: the pill, the live line and the
# panel's frame. The panel's rows are in lazy/hud.ftl, fetched when the panel first opens.
# Every { $variable } is measured on this page view; the console fills it in.

hud-region = Консоль доказательств
hud-pill-label = Консоль доказательств:
hud-pill-server = время сервера
hud-pill-weight = вес страницы
hud-pill-pq = (постквантовый TLS)
hud-pill-dev = HTTP · разработка
hud-pill-edge = HTTPS · прокси хостинга

hud-id = Доказательства
hud-title = Эта страница в цифрах
hud-close = Закрыть
hud-raw = Показать заголовки как есть

# The A/B test under "Real visitors" (web/hud/experiment.ts), while one runs. { $max } is the
# planned page views per variant, fixed before the test began. { $metric } is an event id (code).

hud-foot-lead = Эта страница:
hud-foot-server = { $value } на сервере
hud-foot-cookies = { $n ->
    [one] { $n } файл cookie
    [few] { $n } файла cookie
    [many] { $n } файлов cookie
   *[other] { $n } файла cookie
}
hud-foot-third = { $n ->
    [one] { $n } сторонний запрос
    [few] { $n } сторонних запроса
    [many] { $n } сторонних запросов
   *[other] { $n } стороннего запроса
}
hud-foot-pq = { $tls } + постквантовый обмен ключами
hud-foot-dev = обычный HTTP (разработка)
hud-foot-edge = HTTPS на прокси хостинга
