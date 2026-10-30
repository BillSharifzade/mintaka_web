# S4 · Route across Dushanbe (web/instruments/routing). Counts are intersections: what the
# searches actually settle. They are read with t(), so write them as "label — number".
routing-status-nowasm = WebAssembly недоступен
routing-status-loading = загружаем карту
routing-status-failed = не загрузилось
routing-status-ready = в вашем браузере
routing-route = Маршрут
routing-mode = Режим
routing-mode-watch = Наблюдать
routing-mode-race = Гонка
routing-preset-airport = От вокзала до аэропорта
routing-preset-random = Случайная пара
routing-hint = Или коснитесь карты дважды: сначала старт, потом финиш. Маршруты кратчайшие по расстоянию, улицы с односторонним движением учитываются.
routing-dijkstra = Дейкстра
routing-dijkstra-what = Расходится во все стороны, пока не доберётся до цели.
routing-astar = A*
routing-astar-what = Предпочитает дороги, ведущие к цели, поэтому ищет внутри конуса.
routing-ch = Contraction Hierarchies
routing-ch-what = Использует заранее подготовленные сокращения: поднимается на важные дороги и перескакивает.
routing-explored = Обойдено перекрёстков
routing-time = Время на маршрут
routing-speedup = Ускорение
routing-baseline = эталон
routing-summary = Те же дороги. Тот же ответ. В { $less } меньше работы.
routing-announce = Маршрут найден: { $km } км. Обойдено перекрёстков: Дейкстра — { $d }, Contraction Hierarchies — { $c }.
routing-graph = Граф: перекрёстков — { $n }, дуг дорог — { $arcs }, сокращений — { $shortcuts }; подготовлен при сборке на Rust. Время — медиана 9 прогонов, замерено в этом браузере.
