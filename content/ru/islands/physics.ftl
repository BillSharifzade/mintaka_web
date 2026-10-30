# S2 · Edge of physics (web/instruments/physics.ts).
# Every { $variable } is a value measured on this page view; the island fills it in.
# { $city } and { $server_city } are place names from the time zone database and the server's
# configuration, in Latin script and uninflected: keep them out of positions that need a case.

physics-intro = Наш сервер собрал эту страницу за { $server }.
physics-measure = Измерить моё расстояние
physics-measuring = Измеряем…
physics-again = Измерить снова
physics-hint = Засекает путь туда и обратно до нашего сервера и читает часовой пояс вашего устройства — всё внутри браузера.
physics-status-ready = готово
physics-status-measured = измерено

physics-rtt-label = Ваш путь туда и обратно
physics-bound-label = Вы не дальше
physics-floor-label = Свету нужно не меньше
physics-src-kernel = Время пути туда и обратно для этого соединения по данным ядра Linux на нашем сервере (Server-Timing: krtt)
physics-src-navigation = Ваш браузер: от requestStart до responseStart за вычетом времени нашего сервера
physics-src-quic = Собственная оценка времени пути туда и обратно, которую QUIC ведёт для этого соединения HTTP/3 по подтверждениям (Server-Timing: qrtt)
physics-src-ping = Лучший из { $n ->
    [one] { $n } запроса
    [few] { $n } запросов
    [many] { $n } запросов
   *[other] { $n } запроса
} к /_ping за вычетом времени нашего сервера
physics-bound-src = Половина пути туда и обратно × 204,2 км/мс, скорость света в стекле (c ÷ 1,468)
physics-floor-src = Часовой пояс { $zone }: { $city }, расстояние { $km }. 2 × расстояние ÷ 204,2 км/мс.
physics-floor-none = Нужны город вашего часового пояса и местоположение нашего сервера.

physics-bound = Ваши пакеты вернулись за { $rtt }, значит, вы не дальше { $km } от нашего сервера: дальше свет в стекле за это время не успеет.
physics-floor = Свету в оптоволокне нужно не меньше { $floor } на путь { $city } — наш сервер ({ $server_city }) и обратно. Ваш путь туда и обратно занял { $rtt }.
physics-gap = Остальные { $detours } — это сеть: кабели, которые идут не по дуге большого круга, маршрутизаторы и ваше собственное подключение.
physics-server = Наш сервер добавил сверху { $server }.
physics-faster = Физика говорит, что вы не можете находиться в городе { $city }: ваш путь туда и обратно короче, чем нужно свету. Вы не дальше { $km } от нашего сервера.
physics-no-zone = Часовой пояс вашего устройства ({ $zone }) не указывает на конкретное место, поэтому сравнивать не с чем.
physics-no-server = Местоположение нашего сервера не настроено (сервер разработки), поэтому показана только верхняя граница.
physics-no-proof = Не удалось получить из /_proof местоположение нашего сервера, поэтому показана только верхняя граница.
physics-error = Не удалось замерить путь туда и обратно: { $reason }. Попробуйте снова.
physics-offline = Вы офлайн. Для замера нужен один путь туда и обратно до нашего сервера, так что нажмите кнопку снова, когда появится подключение.

physics-bar-label = Куда ушло время · { $total }
physics-part-floor = Предел физики
physics-part-net = Обходы сети
physics-part-srv = Наш сервер
physics-part-rtt = Путь по сети туда и обратно
physics-foot = Местоположение оценивается по часовому поясу устройства, в вашем браузере. Нам ничего не отправляется.
