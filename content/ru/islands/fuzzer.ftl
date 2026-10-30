# S5 · Attack our code (web/instruments/fuzzer). A coverage-guided fuzzer attacks a small Rust
# parser of ours in the visitor's browser. What it shows before the visitor starts it; the rest
# (log, results, coverage points, "Try to crash it yourself") is in lazy/fuzzer.ftl.
# Counts here are read with t(), not tp(): write them as "label: number" or as an ordinal
# ("на 1234-м входе"), never as a number followed by a noun that has to agree with it.
fuzzer-status-nowasm = WebAssembly недоступен
fuzzer-status-loading = загружаем фаззер
fuzzer-status-failed = не загрузился
fuzzer-status-ready = в вашем браузере

## Chapters and controls

fuzzer-chapter-attack = Атака
fuzzer-chapter-fix = Исправление
fuzzer-chapter-compare = Сравнение
fuzzer-lead = Фаззер забрасывает код миллионами искажённых входных данных, пока что-нибудь не сломается. Этот работает в вашем браузере против нашего настоящего парсера на Rust с одной настоящей ошибкой. Он начинает с четырёх бессмысленных байтов и без всяких подсказок.
fuzzer-start = Выпустить фаззер
fuzzer-fix-next = Атаковать исправленную сборку
fuzzer-blind-next = Теперь вслепую
fuzzer-again = Ещё раз
fuzzer-stop = Стоп
fuzzer-speed = Скорость
fuzzer-speed-watch = Наблюдать
fuzzer-speed-full = Полная скорость
fuzzer-watch-hint = Замедлено, чтобы можно было уследить. На полной скорости ошибка находится за миллисекунды.

## Live counters

fuzzer-stat-inputs = Перебрано входов
fuzzer-stat-rate = В секунду
fuzzer-stat-paths = Пути в коде
fuzzer-stat-crashes = Сбои
fuzzer-stat-time = Время

## Log

fuzzer-log-label = Журнал фаззера

## Try it yourself

fuzzer-try-title = Попробуйте уронить его сами

## Star names on the sky. English names are used where these are absent.

fuzzer-star-meissa = Мейсса
fuzzer-star-betelgeuse = Бетельгейзе
fuzzer-star-bellatrix = Беллатрикс
fuzzer-star-mintaka = Минтака
fuzzer-star-alnilam = Альнилам
fuzzer-star-alnitak = Альнитак
fuzzer-star-rigel = Ригель
fuzzer-star-saiph = Саиф
