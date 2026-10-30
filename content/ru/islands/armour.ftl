# S5 · This page's armour (web/instruments/armour.ts).
# A live scan in the visitor's browser: the headers of a HEAD request to this page, and /_proof.

armour-status-scanning = проверяем
armour-status-done = проверка вживую
armour-scanning = Читаем заголовки, с которыми пришла эта страница…
armour-summary = В ответе этой страницы на месте { $ok } из { $n ->
    [one] { $n } заголовка безопасности
    [few] { $n } заголовков безопасности
    [many] { $n } заголовков безопасности
   *[other] { $n } заголовка безопасности
}.
armour-summary-http = HSTS отправляется только по HTTPS, поэтому на этом сервере разработки он не учитывается.
armour-head-src = Прочитано в вашем браузере только что, из HEAD-запроса к этой странице.
armour-head-error = Не удалось прочитать заголовки этой страницы ({ $reason }). Перезагрузите страницу, чтобы повторить.
armour-offline = Вы офлайн, поэтому заголовки этой страницы нельзя получить с нашего сервера. Проверка запустится сама, когда подключение вернётся.

armour-caption = Заголовки безопасности, с которыми пришла эта страница
armour-col-header = Заголовок
armour-col-status = Статус
armour-col-value = Значение
armour-st-ok = на месте
armour-st-weak = слабый
armour-st-missing = нет
armour-st-na = только HTTPS
armour-not-sent = не отправлен
armour-https-only = отправляется только по HTTPS

armour-why-content-security-policy = Останавливает внедрённые скрипты: выполняться могут только наши собственные файлы, а Trusted Types охраняют каждую точку вставки HTML.
armour-why-cross-origin-opener-policy = Не пускает окна других сайтов в процесс этой страницы.
armour-why-cross-origin-embedder-policy = Загружает только ресурсы, согласные на встраивание, — это завершает изоляцию от других источников.
armour-why-cross-origin-resource-policy = Запрещает другим сайтам встраивать наши файлы.
armour-why-permissions-policy = Отключает возможности браузера, которые этой странице не нужны: камеру, микрофон, геолокацию.
armour-why-referrer-policy = Не сообщает другим сайтам, с какой страницы вы пришли.
armour-why-x-content-type-options = Запрещает браузеру угадывать типы файлов, так что данные никогда не выполнятся как скрипт.
armour-why-strict-transport-security = Велит браузеру открывать этот сайт только по HTTPS, даже если в ссылке указан http.

armour-tls-title = Соединение
armour-tls-version = Версия TLS
armour-tls-cipher = Набор шифров
armour-tls-group = Обмен ключами
armour-tls-unreported = не сообщается
armour-pq-badge-hybrid = постквантовый
armour-pq-badge-pq = постквантовый
armour-pq-badge-classical = классический
armour-pq-badge-unknown = не сообщается
armour-pq-badge-plain = без TLS
armour-pq-badge-edge = на прокси хостинга
armour-pq-hybrid = Ваше соединение использовало { $group } — гибрид классического и постквантового обмена ключами.
armour-pq-pq = Ваше соединение использовало { $group } — постквантовый обмен ключами.
armour-pq-classical = Ваш браузер договорился о { $group } (классический обмен ключами). Мы предлагаем постквантовый обмен каждому браузеру, который его поддерживает.
armour-pq-unknown = Наш сервер не смог определить, какой обмен ключами использовало это соединение, и потому ничего не сообщает, а не гадает.
armour-pq-plain = Сервер разработки: обычный HTTP, без TLS.
armour-pq-edge = HTTPS для этого адреса заканчивается на пограничном прокси нашего хостинг-провайдера, поэтому наш сервер не видит, о какой версии TLS, каком шифре и каком обмене ключами договорился ваш браузер, и не станет гадать. Их показывает панель сведений о сайте в вашем браузере.
armour-tls-src = Из /_proof: о чём наш сервер договорился с вашим браузером для этого соединения.
armour-tls-error = Не удалось получить /_proof ({ $reason }), поэтому сведений о соединении нет. Перезагрузите страницу, чтобы повторить.

armour-iso-title = Изоляция
armour-coi = Изоляция от других источников
armour-coi-yes = да
armour-coi-no = нет
armour-coi-yes-line = COOP и COEP работают: страница выполняется в собственной группе контекстов просмотра, а это ещё и открывает доступ к SharedArrayBuffer.
armour-coi-no-line = Страница не изолирована от других источников, хотя COOP и COEP были отправлены.
armour-coi-no-headers-line = В этом ответе не было сразу и COOP, и COEP, поэтому страница не изолирована от других источников.
armour-tt = Trusted Types
armour-tt-enforced = включены
armour-tt-open = не включены
armour-tt-unsupported = не поддерживаются
armour-tt-enforced-line = Мы попробовали записать обычную строку в innerHTML временного элемента. Ваш браузер это заблокировал.
armour-tt-open-line = Мы попробовали записать обычную строку в innerHTML временного элемента, и запись прошла, хотя наша политика требует Trusted Types.
armour-tt-open-policy-line = Мы попробовали записать обычную строку в innerHTML временного элемента, и запись прошла: Content Security Policy этого ответа не требует Trusted Types.
armour-tt-unsupported-line = Ваш браузер не поддерживает Trusted Types. Content Security Policy всё равно блокирует встроенные скрипты.
armour-iso-src = Проверено в вашем браузере при загрузке этой панели.
