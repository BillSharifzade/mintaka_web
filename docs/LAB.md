# The lab

The home page (`/{lang}/`) is the lab and nothing else: no team, no brief form, no pitch. It opens
with an experiment already running, then the instruments, then the lab notes. Starting a project
happens on the studio page; the header's "Start a project" and the terminal's `contact` go there.
(Rebuilt 2026-11-09; the particle hourglass it replaces is gone.)

## The opening experiment: Dushanbe, searched live

`web/city/` draws Dushanbe's streets (OpenStreetMap, the same extract as the routing instrument)
and searches them in the visitor's browser, in plain TypeScript on typed arrays:

- **Searches run on their own** — Dijkstra and A* in turn, between intersections on screen — and
  each is drawn the way it worked: every intersection it settled, in the order it settled them
  (Dijkstra spreads in rings, A* reaches like a cone), then the route it found.
- **Pointing at the map routes from the pointer** (A*) whenever the nearest intersection changes; a
  click moves the destination; a tap (touch) searches from there and shows the search.
- **Every number is measured**: searches, median time per algorithm, intersections settled, and
  what drawing a frame costs. A search over the whole city takes about a millisecond.
- Three canvases: streets (drawn once per size), searches (incremental, fading), routes (a few lines
  a frame). It sleeps off screen, in a hidden tab and when paused (WCAG 2.2.2: the hero's button,
  the terminal's `sky`); with reduced motion it draws the streets and routes from the pointer only;
  with Data Saver it doesn't fetch the map. It emits `tier` (C live, D still) and `sky` as the
  particle hero did, so the proof console, terminal, RUM and offline mode carry on unchanged.

**The streets file** (`city.bin`, crates/site/src/city.rs, built from `assets/data/dushanbe.bin`):
8,393 intersections and 10,533 streets in 46.7 kB raw, **28 kB brotli** — against 424 kB for the
routing instrument's graph, which also carries the Contraction Hierarchy. Shapes are simplified to
10 m and positions rounded to 4 m (under half a pixel with the city on screen); the streets are
written as a breadth-first walk, so a street that leads somewhere new names no intersection at all
and stores its far end as a short hop; numbers of one kind sit together (columns). A* stays exact:
straight-line distance never exceeds a street's length, rounded or not. Tested in
`web/city/graph.test.ts` (decoder, one-way streets, A* = Dijkstra, the real file).

## The instruments

Eight stations (`STATIONS` in crates/site/src/pages/home.rs), each saying what it tests and where it
runs (your browser, our server, both, a simulation), then the instrument. The first station of each
discipline keeps its old anchor (`#physics`, `#efficiency`, `#security`, `#highload`, `#design`), so
links from the studio page still land on an experiment. A rail lists all eight beside them (a strip
on narrow screens); its lamps mirror each instrument's own status lamp (web/main.ts), and
web/nav/index.ts marks the station being read. The instruments' code is unchanged; the armour
instrument's text is a lazy bundle fetched with its code.

## Weight (release build, 2026-11-09)

Whole page 18.2 kB (en) / 21.0 kB (ru) / 20.5 kB (tg) brotli, first screen 10.2–10.7 kB, inline CSS
31.9 kB minified — down from 26.4 / 30.3 / 29.7 kB, 12.9–13.3 kB and 47.7 kB. The first full view
(page, fonts, boot, the city map and its code, HUD, RUM, offline) is 86.5 / 105.2 / 100.1 kB; its
cap went from 100 to 110 kB for the map, which is fetched after the first paint.
