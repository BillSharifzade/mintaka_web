# The ~ terminal (web/terminal/index.ts): everything it prints, in a lazy bundle fetched when the
# terminal first opens (/assets/strings/terminal.<lang>.<hash>.json). Its frame and the line that
# reports a failed fetch stay inline in ../terminal.ftl. Command names (help, ping, …) and the
# prompt are code, not copy. Every { $variable } is measured on this page view.
# Messages with { $n } are counts, read with tp(): any of them may select on $n for plurals.

## Lines of its own

terminal-welcome = Терминал Mintaka. Каждый ответ берётся из этого просмотра страницы. Наберите help, чтобы увидеть команды.
terminal-unknown = команда не найдена: { $cmd } — наберите help
terminal-offline = офлайн: этой команде нужен наш сервер. Повторите, когда появится подключение.

## help

terminal-help-help = список команд
terminal-help-whoami = что ваш браузер сообщает этой странице, вычислено локально
terminal-help-pillars = четыре звезды, по строке на каждую
terminal-help-ping = { $n ->
    [one] { $n } запрос
    [few] { $n } запроса
    [many] { $n } запросов
   *[other] { $n } запроса
} к /_ping: мин., медиана, p90, макс.
terminal-help-trace = запрос этой страницы, шаг за шагом
terminal-help-proof = это соединение глазами нашего сервера (/_proof)
terminal-help-lang = читать эту страницу на другом языке
terminal-help-sky = остановить или продолжить поиски на карте города
terminal-help-ctf = испытание в консоли
terminal-help-contact = перейти к форме связи
terminal-help-clear = очистить экран (или Ctrl+L)
terminal-help-exit = закрыть терминал (или Esc)

## pillars

terminal-star-rigel = Ригель
terminal-star-saiph = Саиф
terminal-star-betelgeuse = Бетельгейзе
terminal-star-meissa = Мейсса
terminal-pillar-rigel = Эффективность. Быстрее за счёт математики, а не железа.
terminal-pillar-saiph = Безопасность. Сначала мы атакуем собственную работу.
terminal-pillar-betelgeuse = Высокие нагрузки. Рассчитано на день, когда трафик вспыхнет сверхновой.
terminal-pillar-meissa = Интуитивный дизайн. Спроектировано под то, как на самом деле думают люди.

## whoami

terminal-yes = да
terminal-no = нет
terminal-unknown-browser = неизвестный браузер
terminal-gpu-no-adapter = API есть, но адаптера нет
terminal-who-browser = браузер
terminal-who-langs = языки
terminal-who-tier = уровень отрисовки
terminal-who-tier-none = пока не сообщается
terminal-who-motion = анимация
terminal-motion-reduced = уменьшена, как просят ваши настройки
terminal-motion-full = полная
terminal-who-cores = логические ядра
terminal-who-isolated = crossOriginIsolated
terminal-who-gpu = WebGPU
terminal-who-local = Вычислено в вашем браузере. Ничего не отправлено.

## ping

terminal-ping-start = PING /_ping: { $n ->
    [one] { $n } запрос
    [few] { $n } запроса
    [many] { $n } запросов
   *[other] { $n } запроса
} подряд…
terminal-ping-got = { $n ->
    [one] получен { $n } ответ из { $total }
    [few] получено { $n } ответа из { $total }
    [many] получено { $n } ответов из { $total }
   *[other] получено { $n } ответа из { $total }
}
terminal-ping-stats = мин. { $min } · медиана { $median } · p90 { $p90 } · макс. { $max }
terminal-ping-server = собственное время нашего сервера (Server-Timing: app), медиана { $ms }
terminal-ping-src = Замерено вашим браузером: Resource Timing, от requestStart до responseStart.
terminal-ping-src-clock = Замерено часами страницы вокруг fetch(): для этих запросов не появилось ни одной записи Resource Timing.

## trace

terminal-trace-head = Ваш запрос страницы шаг за шагом (Navigation Timing и Server-Timing):
terminal-trace-none = Для этой страницы не записано ни одной записи Navigation Timing.
terminal-trace-dns = запрос DNS
terminal-trace-connect = соединение (TCP и TLS)
terminal-trace-tls = рукопожатие TLS
terminal-trace-wait = от запроса до первого байта
terminal-trace-st-app = наш сервер (app)
terminal-trace-st-krtt = RTT ядра (krtt)
terminal-trace-st-qrtt = RTT QUIC (qrtt)
terminal-trace-st-tls = рукопожатие TLS, наша сторона (tls)
terminal-trace-download = загрузка
terminal-trace-total = итого
terminal-note-reused = соединение переиспользовано
terminal-note-plain = обычный HTTP (разработка)

## proof, lang, sky, ctf, contact

terminal-proof-src = Из /_proof, получено только что.
terminal-lang-same = Вы уже читаете версию { $code }.
terminal-lang-go = Открываем /{ $code }/…
terminal-lang-unknown = Версии { $code } у этой страницы нет. Доступны: { $list }
terminal-lang-list = Использование: lang <code>. Доступны: { $list }
terminal-sky-on = Поиски по городу продолжаются.
terminal-sky-off = Поиски по городу на паузе.
terminal-sky-none = Карта города на этом устройстве не запущена, так что ставить на паузу нечего.
terminal-ctf = Испытание откроется вместе с запуском сайта.
terminal-contact-none = Форма заявки — на странице студии, открываем её.
