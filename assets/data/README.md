# Data

`dushanbe.bin` — Dushanbe's car-routable road network for the S4 routing race: 8,393
intersections, 18,980 arcs with full road geometry, and a Contraction Hierarchy (built by
`crates/routing`, feature `prep`). Rebuild with `cargo xtask graph --fetch`; the query, the
OpenStreetMap timestamp and the counts are in `dushanbe.meta.json`.

Map data © OpenStreetMap contributors, available under the Open Database Licence (ODbL 1.0):
https://www.openstreetmap.org/copyright. `dushanbe.bin` is a derived database under the same
licence; the attribution is shown on the page next to the map.
