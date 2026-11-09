# The lab: the observatory and the bench

The home page (`/{lang}/`) is the lab: every claim the studio makes is an instrument the visitor runs.
Its design (2026-11-05) follows one idea — a real laboratory has two rooms.

## Two rooms

- **The observatory** (night, the default colours): the hero, the speed of light and Orion. These
  are the particle engine's three anchors (the hourglass, the ray, the constellation), so they stay
  dark — the engine draws light on a dark sky.
- **The bench** (`div.bench.lit` in `crates/site/src/pages/home.rs`): every experiment, how we
  work, the Lab notes, the team and the brief form, on Starlight ruled like graph paper. `.lit`
  (tokens.css) re-points the semantic colours for the lit room; the brand palette never changes,
  so islands that draw with `--starlight` or `--graphite` keep working.
- The page ends at a night window over the Pamir (`horizon()`), then the dark title-block footer.
- The fixed parts follow the room under them: `web/nav/index.ts` measures the bench on load and
  resize and toggles `.lit` on the header (at its bottom edge) and the Orion navigation (at the
  middle of the screen) — no layout reads while scrolling. Without JavaScript the header stays dark.

## Instruments are devices

`instrument()` (pages/mod.rs) draws each live demo as a device (components.css): a faceplate
(`--plate`, light on the bench, graphite at night) with a model plate (the instrument's id), the
title, and a **status lamp** — the island's own status badge (`badge--info`, `--live`, `--ok`,
`--warn`), dark while the frame is still static. The screen (`.instrument__body`) points the
semantic colours back to night, so every island interface looks the same in either room. When an
island's code arrives, `web/main.ts` adds `.is-on` and the screen sweeps once (reduced motion: no
sweep).

## Records

A pillar section opens like a lab record (`pillar_head()`): a rule, the star it is filed under
with its catalogue position (orion.rs) as the record's id, the pillar; the claim large on the left,
the lead beside it. The record's instruments come straight after its head — the experiment first —
and what explains them follows: the methods as a specification sheet (`spec()`), Security's two
protocols (offence: a rust diamond; defence: a blue square), Highload's layers as an apparatus list
and Betelgeuse as a footnote under its instruments. The code specimen is a printed listing with
numbered lines (`numbered()`, CSS counters), not a device: it is static. How we work keeps the
belt (Measure → Build → Prove) and lists the Proof Pack as documents. Where the browser has scroll
timelines, a record's rule is drawn from the left and its claim rises as it scrolls in (CSS only).
The lab keeps near-square corners (2–4 px, layout.css); only lamps and stars are round.

Nothing on the bench asks to be taken on trust: the sections that only asserted (the qualities
nobody demos, a proof wall waiting for its first client results) were removed in this redesign.

## The star chart

Orion (S3) is a chart, not a card grid: each pillar's label stands beside the figure at its star's
height and points at it with a leader line — half drawn in the SVG (`svg::orion_chart`), half in
CSS — and the figure is sized to fit beside its heading. Narrow screens list the labels under it.

## The proof console

On screens wide enough to have margins (≥ 90rem) the console's pill stands in the right margin as a
narrow column of readings, so it never covers an instrument's numbers; narrower screens keep the
compact pill.

## The hero's readout

The hourglass sits in a viewfinder. Once the engine settles on a tier, it emits `render` (bus.ts)
with what it measured of its own frames — median and 90th percentile, GPU time on WebGPU and
WebGL 2 — and `web/main.ts` prints it under the figure: "Drawn live · WebGPU · 120 000 particles ·
0.4 ms a frame". Tier D (reduced motion, Data Saver, no JavaScript) shows the still mark and no
readout.

## Weight

The redesign kept the page inside its budgets by moving what only islands need out of the inline
CSS (`islands/shared.css`: key–value rows, the overlay sheet, the construction grid, lit lamps,
the power-on sweep, the careers form) and by making the armour instrument's text a lazy bundle
(`content/{lang}/islands/lazy/armour.ftl`), fetched together with its code (`lazyText` in
web/main.ts). Release build, 2026-11-05: whole page 26.4 kB (en) / 30.3 kB (ru) / 29.7 kB (tg)
brotli, first screen 12.9–13.3 kB, against 32 kB and 14 kB caps (before the redesign:
27.1 / 31.4 / 30.8 kB and 11.8–12.1 kB).
