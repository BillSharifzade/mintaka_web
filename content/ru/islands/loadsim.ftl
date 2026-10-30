## S6 · Overload lab (web/instruments/loadsim/): a simulation, compiled from Rust to WebAssembly
## and run in the visitor's browser. Every number it shows comes from that run — nothing here is
## measured on our servers, and the copy must keep saying so.
## Counts here are whole numbers, so long messages give only the forms Russian needs for them
## (one, few, many), with the commonest as the default: each variant is a full copy of the text.

loadsim-status = симуляция
loadsim-loading = загрузка
loadsim-nowasm = Этот браузер не умеет запускать WebAssembly, поэтому симуляция здесь не стартует.
loadsim-failed = Симуляция не загрузилась, так что запускать здесь нечего.
loadsim-load = Нагрузка
# { $rate } is the multiplier × the base rate, a multiple of 10 today, so "запросов" always agrees.
loadsim-load-value = ×{ $x } — { $rate } запросов/с
loadsim-pause = Пауза
loadsim-play = Пуск
loadsim-reset = Заново
loadsim-about = Это симуляция, а не замер этого сайта. Запросы приходят случайно (поток Пуассона) — { $base } в секунду при ×1, и каждому в среднем нужно { $mean } работы, с длинным хвостом (логнормальное распределение). Клиенты сдаются через { $timeout } и повторяют запрос { $n ->
    [1] один раз
    [few] { $n } раза
   *[many] { $n } раз
}. Обе системы получают одни и те же запросы.
loadsim-naive = Наивная
loadsim-naive-about = { $n ->
    [1] Один обработчик
    [one] { $n } обработчик
    [few] { $n } обработчика
    [many] { $n } обработчиков
   *[other] { $n } обработчика
} и одна очередь без ограничений. Каждый запрос ждёт своей очереди, даже когда его клиент уже сдался, — и работа делается ни для кого.
loadsim-ours = По-нашему
loadsim-ours-about = Начинает с { $n ->
    [1] одного обработчика
    [one] { $n } обработчика
   *[many] { $n } обработчиков
}; автомасштабирование добавляет новые, до { $max }, и каждому нужно { $boot } на запуск. Когда запросы простаивают в очереди дольше { $target } на протяжении { $interval }, новые сразу получают ответ 503 — как и залежавшиеся; в очереди никогда не бывает больше { $per } на обработчик.
loadsim-p99 = p99 задержки успешных ответов
loadsim-none = вовремя — ни одного ответа
loadsim-served = Отвечено вовремя / поступило
loadsim-served-value = { $good } из { $offered }/с
loadsim-refused = Отказано (503) · истёк таймаут
loadsim-refused-value = { $shed } · { $timeouts }/с
loadsim-state = Очередь и обработчики
loadsim-waiting = { $n ->
    [one] { $n } запрос в очереди
    [few] { $n } запроса в очереди
    [many] { $n } запросов в очереди
   *[other] { $n } запроса в очереди
}
loadsim-busy = занято { $busy } из { $n ->
    [one] { $n } обработчика
    [few] { $n } обработчиков
    [many] { $n } обработчиков
   *[other] { $n } обработчика
}
loadsim-starting = { $n ->
    [one] ещё { $n } запускается
   *[other] ещё { $n } запускаются
}
loadsim-wasted = { $n }/с работы для клиентов, которые уже сдались
loadsim-limit = (предел симуляции)
loadsim-little = { $l } в системе ≈ { $lambda }/с × { $w } = { $lw }
loadsim-little-about = Закон Литтла: среднее число запросов в системе равно скорости, с которой они её покидают, умноженной на время, которое каждый в ней проводит. Все три величины измеряются отдельно за последние 4 с; они сходятся, пока система стабильна, и расходятся, пока её очередь растёт или рассасывается.
# { $cap } is 10 000 (crates/loadsim/src/sim.rs, queue_cap), so "ожидающих запросов" agrees.
loadsim-legend = Графики показывают последнюю минуту в одном масштабе на обеих панелях. Задержка — в логарифмической шкале от 10 мс до 10 с; пунктир — таймаут клиентов, { $timeout }. Чтобы ограничить память, наивная очередь вмещает не больше { $cap } ожидающих запросов; дальше новые теряются, а их клиенты ждут до таймаута.
loadsim-try = Включите ×40 на полминуты, а потом верните ×1: наивная система лежит, пока не разгребёт запросы, которых уже никто не ждёт.
