# S5 · Attack our code (web/instruments/fuzzer): what the fuzzer says once the visitor starts it or
# opens "Try to crash it yourself" — a lazy bundle (/assets/strings/fuzzer.<lang>.<hash>.json),
# fetched at that moment. What the instrument shows before is inline in ../fuzzer.ftl. Coverage
# points and parse errors are numbered as in crates/fuzzer/src/points.rs and target.rs: keep the ids.

## Log

fuzzer-log-start = фаззинг { $build } · { $mode } · зерно { $seed }
fuzzer-log-new = новый путь
fuzzer-log-crash = СБОЙ
fuzzer-log-done = остановлен · входов: { $inputs } · путей: { $paths }
fuzzer-build-buggy = v1 (с ошибкой)
fuzzer-build-fixed = v2 (исправлена)
fuzzer-mode-guided = по покрытию
fuzzer-mode-blind = вслепую, случайно

## Result cards

fuzzer-crash-title = Сбой найден на { $inputs }-м входе · { $time }
fuzzer-crash-panic = Сообщение о панике от самого Rust, перехваченное в вашем браузере:
fuzzer-crash-min = Сокращено с { $from } Б до { $to } Б — только тех, что важны:
fuzzer-crash-why = Байт длины в секции имени обещал больше байтов, чем есть в карте. Строка { $line } ему поверила.
fuzzer-crash-safe = В Rust это чистая и безопасная остановка. В C та же строка могла бы прочитать память за концом буфера. В любом случае до релиза это не доходит.
fuzzer-crash-fix = Исправление — одна строка:
fuzzer-fixed-title = Исправленная сборка: входов — { $inputs }, сбоев — 0
fuzzer-fixed-body = Достигнуто путей в коде: { $paths } из { $total }. Тёмным остался один — карта с именем ровно MINTAKA: семь точных байтов без единого ветвления между ними почти не видны для обратной связи по покрытию. Промышленные фаззеры добавляют для этого словари и трассировку сравнений.
fuzzer-blind-title = Вслепую: путей — { $paths } из { $total }, входов — { $inputs }
fuzzer-blind-body = Случайные байты, никакой обратной связи, { $time } вашего процессора. С обратной связью по покрытию тот же движок нашёл ошибку уже на { $guided }-м входе. Вся хитрость — в обратной связи.
fuzzer-blind-body-no-guided = Случайные байты, никакой обратной связи, { $time } вашего процессора. С обратной связью по покрытию тот же движок находит ошибку за доли секунды.

## Try it yourself (built when the panel first opens)

fuzzer-try-lead = Введите карту как текст с экранированием \xNN или как пары шестнадцатеричных цифр. До 64 байт.
fuzzer-try-format = Формат: «ORI» · версия (1–2) · число звёзд (1–7) · 4 байта на звезду (x, y, звёздная величина, флаги) · затем, в версии 2, секции: C (линии) и N (имя: байт длины, затем само имя).
fuzzer-try-label = Ваш ввод
fuzzer-try-run = Запустить
fuzzer-try-build = Против
fuzzer-try-example-valid = Корректная карта
fuzzer-try-example-betelgeuse = Карта с Бетельгейзе
fuzzer-try-ok = Разобрано без ошибок · достигнуто путей в коде: { $paths }.
fuzzer-try-rejected = Безопасно отклонено: { $reason } · достигнуто путей в коде: { $paths }.
fuzzer-try-invalid = Это неверная escape-последовательность. Используйте \x и две шестнадцатеричные цифры.
fuzzer-try-crashed = Вы его уронили. Вы мыслите как атакующий — давайте познакомимся.
fuzzer-try-careers = Открытые вакансии

## Screen-reader announcements

fuzzer-announce-start = Фаззер запущен.
fuzzer-announce-paths = Найдено путей в коде: { $paths } из { $total }.
fuzzer-announce-crash = Сбой найден на { $inputs }-м входе.
fuzzer-announce-done = Прогон завершён: входов — { $inputs }, путей — { $paths }, { $n ->
    [one] { $n } сбой
    [few] { $n } сбоя
    [many] { $n } сбоев
   *[other] { $n } сбоя
}.

## Why a chart was rejected (parse errors in crates/fuzzer/src/target.rs)

fuzzer-reason-bad-magic = карта не начинается с «ORI»
fuzzer-reason-bad-version = неизвестная версия
fuzzer-reason-no-stars = в карте должна быть хотя бы одна звезда
fuzzer-reason-too-many-stars = у Ориона семь звёзд, не больше
fuzzer-reason-truncated = карта обрывается
fuzzer-reason-reserved-flags = установлены зарезервированные биты флагов
fuzzer-reason-unknown-section = неизвестная секция
fuzzer-reason-bad-line = линия указывает на звезду, которой нет

## Coverage points, by id (crates/fuzzer/src/points.rs). Keep the ids.

fuzzer-point-0 = сигнатура «O»
fuzzer-point-1 = сигнатура «OR»
fuzzer-point-2 = сигнатура «ORI»
fuzzer-point-3 = версия 1
fuzzer-point-4 = версия 2
fuzzer-point-5 = неизвестная версия
fuzzer-point-6 = карта без звёзд
fuzzer-point-7 = больше 7 звёзд
fuzzer-point-8 = число звёзд принято
fuzzer-point-9 = записи звёзд обрываются
fuzzer-point-10 = прочитана первая звезда
fuzzer-point-11 = вторая звезда
fuzzer-point-12 = третья звезда — уже пояс
fuzzer-point-13 = седьмая звезда — весь Орион
fuzzer-point-14 = ярче нулевой величины
fuzzer-point-15 = первая величина
fuzzer-point-16 = вторая величина
fuzzer-point-17 = тусклая звезда
fuzzer-point-18 = северо-восточная четверть
fuzzer-point-19 = северо-западная четверть
fuzzer-point-20 = юго-восточная четверть
fuzzer-point-21 = юго-западная четверть
fuzzer-point-22 = переменная звезда
fuzzer-point-23 = кратная звезда
fuzzer-point-24 = гигант
fuzzer-point-25 = сверхгигант
fuzzer-point-26 = зарезервированные биты флагов
fuzzer-point-27 = Бетельгейзе: яркий переменный сверхгигант
fuzzer-point-28 = Минтака: кратный гигант
fuzzer-point-29 = чистый конец карты
fuzzer-point-30 = неизвестная секция
fuzzer-point-31 = секция линий
fuzzer-point-32 = линии проверены
fuzzer-point-33 = линия к несуществующей звезде
fuzzer-point-34 = секция имени
fuzzer-point-35 = длина имени прочитана
fuzzer-point-36 = имя прочитано
fuzzer-point-37 = имя заглавными буквами
fuzzer-point-38 = карта с именем MINTAKA
