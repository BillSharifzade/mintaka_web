## S6 · Follow one request (web/instruments/trace.ts).
## The visitor's own page request, hop by hop: Navigation Timing in their browser plus our
## Server-Timing header. Durations are filled in live — never put an example number here. The
## hops' explainers are in lazy/trace.ftl, fetched when a hop is first opened.

trace-status = измерено
trace-status-none = недоступно
trace-unsupported = Этот браузер не сообщает Navigation Timing, поэтому этапы запроса этой страницы здесь не показать.
trace-cached = Браузер показал эту страницу из собственного кеша, так что ни один запрос не прошёл по сети. Отправьте новый запрос, чтобы его проследить.
trace-page = Ваш запрос страницы
trace-latest = Последний запрос
trace-size = { $size } по сети
trace-total = всего { $total }
trace-again = Отправить ещё запрос
trace-failed = Запрос не прошёл. Проверьте подключение и попробуйте снова.
trace-offline = Вы офлайн, поэтому запрос не дойдёт до нашего сервера. Отправьте его снова, когда появится подключение.
trace-done = Запрос { $n }: всего { $total }, из них { $app } — внутри нашего сервера.
trace-done-short = Запрос { $n }: всего { $total }.
trace-gap = Остальные { $ms } браузер потратил на проверку своего кеша и постановку запроса в очередь.

trace-dns = Запрос DNS
trace-dns-short = DNS
trace-tcp = Соединение
trace-tcp-short = TCP
trace-tls = Рукопожатие TLS 1.3
trace-tls-short = TLS
trace-tls-server = наша сторона { $ms }
trace-net = Сеть, туда и обратно
trace-net-short = СЕТЬ
trace-krtt = RTT ядра { $ms }
trace-qrtt = RTT QUIC { $ms }
trace-app = Внутри нашего сервера
trace-app-short = СЕРВ
trace-app-split = маршрут { $route } · рендер { $render }
trace-ttfb = От запроса до первого байта
trace-ttfb-short = ЖДЁМ
trace-down = Загрузка
trace-down-short = ТЕЛО

trace-zero-reused = соединение переиспользовано
trace-zero-cached = ответ уже в кеше
trace-zero-plain = обычный HTTP, без TLS (разработка)
trace-zero-quic = часть рукопожатия QUIC
