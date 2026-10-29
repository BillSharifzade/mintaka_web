+++
slug = "contraction-hierarchies"
title = "Contraction Hierarchies on Dushanbe's roads"
pillar = "efficiency"
date = 2026-10-29
summary = "Our own Contraction Hierarchies, built from an OpenStreetMap extract of Dushanbe: 8,393 intersections, 17,578 shortcuts, about 0.1 s of preprocessing. Over 10,000 random routes, a query settles a median of 90 intersections instead of Dijkstra's 4,141 and takes 12.4 µs instead of 291 µs, with exactly the same distance every time. This post explains the node order, the shortcut test, and why the two-sided upward search cannot miss the shortest route."
repo = "crates/routing"
+++

## The problem

The efficiency section of our home page races three algorithms across the same map of Dushanbe in your browser. It is there to make one claim visible: the fastest way to answer a question is often to do mathematics before anyone asks it. Dijkstra's algorithm, given a start and a destination, explores the city outwards in rings until the destination comes up. A* bends those rings towards the destination with a straight-line estimate. Contraction Hierarchies (Geisberger, Sanders, Schultes and Delling, 2008) spend a moment before the first query to rank every intersection, and then answer each query by searching only upwards in that ranking.

We wrote our own implementation instead of using a library, because the point of the section is that we can explain every step. This post is that explanation, with the measurements.

## Method

### The road graph

The input is an OpenStreetMap extract of 28 October 2026: every car-routable road in a box around Dushanbe, 5,493 ways and 43,906 nodes. [`prep.rs`](repo:crates/routing/src/prep.rs) keeps the nodes where roads meet or end, gives one-way streets, roundabouts and motorways arcs in one direction only, and keeps the largest strongly connected component, so every intersection can reach every other. That leaves 8,393 intersections and 18,980 directed arcs. An arc's weight is its length along the road in decimetres, rounded up, so the straight line never overestimates and A* stays exact. The counts, the query and the timestamp are in [`dushanbe.meta.json`](repo:assets/data/dushanbe.meta.json).

### Contraction

Contracting an intersection $v$ removes it from the graph while keeping every shortest path between the intersections that remain. For each road into $v$ from some $u$ and each road out of it to some $w$, the path $u$, $v$, $w$ might be the only shortest way from $u$ to $w$. If it is, we add a shortcut from $u$ to $w$ whose weight is the sum of the two, and remember $v$ as its middle. If some other path among the remaining intersections is at least as short, the shortcut is unnecessary; that other path is called a witness.

Intersections are contracted one at a time, least important first, and the order becomes their rank: the first one contracted has rank 0. Importance is an estimate (below), re-checked when an intersection reaches the front of the queue; if it has grown past the next one in line, the intersection goes back into the queue.

When an intersection is contracted, its remaining arcs all lead to intersections that will be contracted later. They are stored as its upward arcs (to higher ranks) and downward arcs (from higher ranks). That is the whole hierarchy: 36,497 arcs, of which 17,578 are shortcuts. The contraction counted 17,605 shortcut insertions; the 27 extra were later replaced by shorter shortcuts between the same pair.

### The query

A query runs two Dijkstra searches at once. The forward one starts at $s$ and follows only upward arcs. The backward one starts at $t$ and follows downward arcs in reverse, which also climbs in rank. Every time a search settles an intersection the other one has already reached, the sum of the two distances is a candidate route. Both stop when neither can still produce a better one. The route is then unpacked: each shortcut is replaced by its two halves through the remembered middle, until only road arcs remain, which is what the map draws.

## The mathematics

The order comes from a priority, the smaller the earlier:

$$
p(v) = 2S(v) - A(v) + C(v)
$$

$S(v)$ is the number of shortcuts contracting $v$ would add, $A(v)$ the number of arcs it would remove, and $C(v)$ the number of arcs it has already lost to contracted neighbours. Adding few shortcuts while removing many arcs keeps the graph small; the last term is there to spread contraction across the city instead of finishing one district first.

A shortcut from $u$ to $w$ through $v$ is added exactly when

$$
d_{G - v}(u, w) > c(u, v) + c(v, w)
$$

where $c$ is an arc weight and $d_{G - v}$ is the distance among the intersections not yet contracted, without $v$. The witness search computes that distance with Dijkstra, but gives up after 600 settled intersections or once it passes $c(u,v) + c(v,w)$. Giving up early can only overestimate the left side, so the test can only add a shortcut that wasn't needed; it can never leave out one that was. A useless shortcut costs memory, never a wrong answer.

### Why the upward search is correct

Call a path up-down if the ranks along it rise to one highest intersection and then fall. The claim is that for any $s$ and $t$ there is a shortest path, using shortcuts, that is up-down. Take any shortest path and look for a valley: an intersection $v$ whose neighbours on the path, $u$ before and $w$ after, both rank higher. When $v$ was contracted, $u$ and $w$ were still in the graph, so the test above ran on exactly this pair. Either it added the shortcut from $u$ to $w$, or a witness at least as short existed among intersections contracted after $v$. In both cases we can replace $u$, $v$, $w$ by something no longer whose new intersections all rank above $v$. Each replacement removes one intersection and adds only higher-ranked ones, so the process cannot go on forever, and when it stops there is no valley left.

The first half of that up-down path is found by the forward search and the second half, reversed, by the backward one. So

$$
d(s, t) = \min_{v} \left( d_{\uparrow}(s, v) + d_{\downarrow}(v, t) \right)
$$

where $d_{\uparrow}$ uses only upward arcs and $d_{\downarrow}$ only downward ones, and the minimum is reached at the path's highest intersection. The searches may stop as soon as

$$
\min(k_f, k_b) \ge B
$$

with $k_f$ and $k_b$ the smallest keys left in the two queues and $B$ the best sum found so far. If a better route existed, both halves of it would be shorter than $B$, so both searches would already have settled its highest intersection, and the second of them would have recorded it.

## The code

The priority and the witness test, from [`prep.rs`](repo:crates/routing/src/prep.rs). `witness.run` is the bounded Dijkstra; `deleted[v]` is $C(v)$:

```rust include="crates/routing/src/prep.rs" from="/// Shortcuts contracting"
```

The two-sided upward query, from [`search.rs`](repo:crates/routing/src/search.rs). One loop serves both directions: it always advances the side with the smaller key, and checks for a meeting whenever it settles an intersection:

```rust include="crates/routing/src/search.rs" from="let mut best = u32::MAX;"
```

## The benchmark

We measured on 2026-10-29 on a laptop with a 13th-generation Intel Core i7-13620H, Arch Linux (kernel 7.2.11-zen1), rustc 1.99.1, `opt-level = 3` with fat LTO, a desktop session running. The harness is a scratch program outside the repository that links the `routing` crate. It loads the committed `assets/data/dushanbe.bin`, draws 10,000 random start and destination pairs with a xorshift generator (seed 42), and times each query with `Instant::now()`. Each algorithm runs over all pairs in its own pass after 1,000 warm-up queries. Every timing includes rebuilding the route as road arcs.

| Algorithm | Median time | p99 time | Median settled | p99 settled |
|---|---:|---:|---:|---:|
| Dijkstra | 291.1 µs | 583.0 µs | 4,141 | 8,304 |
| A* | 78.5 µs | 474.6 µs | 874 | 5,129 |
| Contraction Hierarchies | 12.4 µs | 20.1 µs | 90 | 143 |

Table: 10,000 random routes across Dushanbe, one thread. All three returned the same distance for every pair.

At the median, the hierarchy does 46 times less work than Dijkstra and answers 23.5 times sooner. The ratios differ because each settled intersection costs the hierarchy more: two queues, a meeting check, and unpacking shortcuts, which searches the adjacency lists of each middle intersection. The tail matters more than the median: Dijkstra's p99 settles almost the whole city, the hierarchy's p99 settles 143 intersections.

The home page's preset route, from the railway station to the airport, is 9,098.9 m long. Natively it took Dijkstra 304.8 µs and 4,867 settled intersections, A* 28.8 µs and 879, and the hierarchy 4.6 µs and 88 (medians of 101 runs). Our milestone notes measured the same counts in Chrome with WebAssembly: 290 µs, 34 µs and 6.4 µs.

Preprocessing is cheap at this size. From the raw Overpass JSON to the finished hierarchy took a median of 105 ms over five runs, of which 16 ms was parsing JSON, so building the graph, finding the strongly connected component and contracting it took about 89 ms. (The 0.6 s in our milestone notes was measured through `cargo xtask graph`, which runs the same code without release optimisations.) The rebuilt file was byte for byte identical to the committed one, 1,138,156 bytes: the contraction is deterministic.

Correctness has its own tests, which compare A* and the hierarchy with Dijkstra on four random grids with one-way streets (300 pairs each) and on 1,000 random pairs across Dushanbe, and check that every route is a connected chain of real arcs whose weights add up:

```sh title="Run the correctness tests and rebuild the graph"
cargo test -p routing --features prep --release
cargo xtask graph          # add --fetch for a fresh OpenStreetMap extract
```

What we did not do: stall-on-demand, a smarter witness search, or the multi-threaded batch of 10,000 routes the spec asks the browser to run. With 8,393 intersections none of them is needed for the page; on a country-sized graph the first two would be the next work.
