## S6 · Live load (web/instruments/live.ts).
## This server's own traffic, one tick per second from GET /_telemetry (Server-Sent Events).
## Aggregates only. Values are filled in live — never put an example number here.

live-connecting = подключаемся
live-live = в эфире
live-reconnecting = переподключаемся
live-offline = офлайн
live-paused = на паузе
# The button that stops and restarts the live updates.
live-pause = Пауза
live-resume = Продолжить
live-unsupported = Этот браузер не умеет открывать поток Server-Sent Events, поэтому нагрузку вживую здесь не показать.
live-rps = Запросов в секунду
live-p50 = Задержка, медиана
live-p99 = Задержка, 99-й процентиль
live-p999 = Задержка, 99,9-й процентиль
live-cpu = CPU, доля одного ядра
live-rss = Занятая память (RSS)
live-conns = Открытые соединения
live-total = Запросов с момента запуска
live-idle = запросов нет
live-range = от { $min } до { $max } за последнюю минуту
live-dots = { $n ->
    [1] Каждая точка — один запрос.
    [one] Каждая точка — { $n } запрос.
    [few] Каждая точка — { $n } запроса.
    [many] Каждая точка — { $n } запросов.
   *[other] Каждая точка — { $n } запроса.
}
live-server = Сервер { $region } · { $n ->
    [one] { $n } ядро
    [few] { $n } ядра
    [many] { $n } ядер
   *[other] { $n } ядра
} · сборка { $build }
live-window = Задержка — за последние 10 секунд; графики показывают последние 60.
live-foot = Только агрегаты: IP-адреса, пути и строки User-Agent в этот поток не попадают никогда.
live-said-live = Живые данные подключены.
live-said-lost = Поток данных прервался. Пробуем снова.
live-said-offline = Живые данные недоступны. Пробуем снова в фоне.
