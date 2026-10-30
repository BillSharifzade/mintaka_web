# Proof console (web/hud/index.ts): the panel's rows — labels, sources, values, real visitors and
# the A/B test — in a lazy bundle fetched when the panel first opens
# (/assets/strings/hud.<lang>.<hash>.json). The pill, the footer's live line and the panel's frame
# stay inline in ../hud.ftl. Every { $variable } is measured on this page view.
# { $n } is a count, read with tp() / pluralKey(): select on it for plurals.

hud-pending = ещё не измерено
hud-na = недоступно в этом браузере
hud-not-reported = сервер не сообщил
hud-k-server = Время сервера
hud-k-weight = Вес страницы
hud-k-third = Сторонние запросы
hud-k-cookies = Файлы cookie
hud-k-tls = Соединение
hud-k-headers = Заголовки безопасности
hud-k-lcp = Отрисовка самого крупного контента (LCP)
hud-k-inp = Самое медленное взаимодействие (INP)
hud-k-cls = Совокупное смещение макета (CLS)
hud-k-tier = Уровень отрисовки заставки
hud-k-visitors = Реальные посетители этого сервера
hud-src-server = Заголовок Server-Timing этой страницы (app): от разбора запроса до готового ответа
hud-src-weight = Resource Timing: сумма transferSize всех запросов на этот момент
hud-src-third = Resource Timing: запросы, сгруппированные по источнику (origin)
hud-src-cookies = document.cookie плюс число Set-Cookie из /_proof
hud-src-tls = /_proof: версия TLS, шифр, обмен ключами и версия HTTP, о которых договорился наш сервер
hud-src-edge = /_proof сообщает, что запрос пришёл через пограничный прокси хостинга; версия HTTP — та, что видит ваш браузер (Navigation Timing)
hud-src-headers = HEAD-запрос к этой странице, прочитанный в вашем браузере
hud-src-lcp = PerformanceObserver: largest-contentful-paint
hud-src-inp = Event Timing: самое долгое взаимодействие, измеренное на этой странице на этот момент
hud-src-cls = PerformanceObserver: layout-shift, наибольшее окно сессии
hud-src-tier = Движок заставки, на этом устройстве
hud-src-visitors = GET /_rum: метрики Web Vitals, которые браузеры посетителей прислали этому серверу с момента его запуска, и счётчики A/B-теста по вариантам; хранятся только итоги, в памяти. Без cookie, IP-адресов и сведений о браузерах.
hud-requests = { $n ->
    [one] { $n } запрос
    [few] { $n } запроса
    [many] { $n } запросов
   *[other] { $n } запроса
}, из кеша — { $cached }
hud-inp-none = взаимодействий пока не было
hud-tier-a = A · WebGPU
hud-tier-b = B · WebGL2
hud-tier-c = C · Canvas 2D
hud-tier-d = D · статичный постер
hud-tier-none = пока не сообщается
hud-plain = Обычный HTTP (разработка) · { $http }
hud-edge = HTTPS до пограничного прокси нашего хостинг-провайдера · { $http }. Наш сервер стоит за ним и не видит параметров TLS.
hud-sent = отправлен
hud-missing = нет
hud-cookies-ours = установлены этим сайтом
hud-cookies-foreign = установлены не этим сайтом
hud-https-only = только HTTPS
hud-err-proof = Не удалось получить /_proof. Закройте и снова откройте панель, чтобы повторить.
hud-err-head = Не удалось прочитать заголовки ({ $reason }). Закройте и снова откройте панель, чтобы повторить.
hud-offline = Вы офлайн, а эта строка обращается к нашему серверу. Откройте панель снова, когда появится подключение.
hud-offline-copy = Не измерено: эта копия страницы взята из офлайн-кеша вашего браузера, а не с нашего сервера.
hud-visitors-since = { $n ->
    [one] С { $date } измерен { $n } просмотр страницы
    [few] С { $date } измерено { $n } просмотра страниц
    [many] С { $date } измерено { $n } просмотров страниц
   *[other] С { $date } измерено { $n } просмотра страницы
}
hud-visitors-p75 = 75-й процентиль из { $n }
hud-visitors-few = { $n ->
    [one] Пока { $n } замер — слишком мало для 75-го процентиля (нужно { $min })
    [few] Пока { $n } замера — слишком мало для 75-го процентиля (нужно { $min })
    [many] Пока { $n } замеров — слишком мало для 75-го процентиля (нужно { $min })
   *[other] Пока { $n } замера — слишком мало для 75-го процентиля (нужно { $min })
}
hud-visitors-declined = Ваш браузер просит сайты его не измерять (Global Privacy Control или Do Not Track), поэтому этот просмотр не учитывается.
hud-visitors-error = Не удалось получить /_rum ({ $reason }). Закройте и снова откройте панель, чтобы повторить.
hud-ab-lead = A/B-тест { $id }: при каждой загрузке страницы наш сервер случайно выбирает один вариант и считает, сколько просмотров заканчиваются событием { $metric }. Лимит просмотров на вариант: { $max }.
hud-ab-control = контрольный
hud-ab-yours = этот просмотр
hud-ab-views = { $max ->
    [one] { $n } из { $max } просмотра
    [few] { $n } из { $max } просмотров
    [many] { $n } из { $max } просмотров
   *[other] { $n } из { $max } просмотра
}
hud-ab-rate = { $metric }: { $n } ({ $rate })
hud-ab-wait = Вердикта не будет, пока каждый вариант не наберёт все свои просмотры: разница, замеченная раньше, слишком часто оказывается случайностью.
hud-ab-better = Лучше контрольного (p = { $p }).
hud-ab-worse = Хуже контрольного (p = { $p }).
hud-ab-same = Явной разницы с контрольным нет (p = { $p }).
hud-ab-done = Завершено: счёт остановлен. Разница явная, если p меньше { $alpha }: это общий уровень { $family }, поделённый между сравнениями (их { $k }).
