# S4 · Route across Dushanbe (web/instruments/routing). Counts are intersections: what the
# searches actually settle.
routing-status-nowasm = WebAssembly unavailable
routing-status-loading = loading the map
routing-status-failed = couldn't load
routing-status-ready = in your browser
routing-route = Route
routing-mode = Mode
routing-mode-watch = Watch
routing-mode-race = Race
routing-preset-airport = Railway station to airport
routing-preset-random = Random pair
routing-hint = Or tap a map twice: the start, then the destination. Routes are the shortest by distance; one-way streets are respected.
routing-dijkstra = Dijkstra
routing-dijkstra-what = Spreads out in every direction until it reaches the destination.
routing-astar = A*
routing-astar-what = Prefers roads that point at the destination, so it searches a cone.
routing-ch = Contraction Hierarchies
routing-ch-what = Uses shortcuts prepared in advance: it climbs to the important roads and jumps.
routing-explored = Intersections explored
routing-time = Time per route
routing-speedup = Speed-up
routing-baseline = baseline
routing-summary = Same roads. Same answer. { $less } less work.
routing-announce = Route found: { $km } km. Dijkstra explored { $d } intersections, Contraction Hierarchies { $c }.
routing-graph = Graph: { $n } intersections, { $arcs } road arcs and { $shortcuts } shortcuts, prepared at build time in Rust. Times are medians of 9 runs, measured in this browser.
