# The studio landing page

`/{lang}/studio/` (en, ru, tg) is the company's front door; the rest of the site is the lab that
proves what it says. It has its own document, stylesheet and one script, and shares none of the
lab's chrome.

## The idea: the mark unfolds

The page is the Mintaka mark, opened up. Each chapter is one of the mark's colourways from the
brand kit, and each change of chapter is a part of the mark growing to fill the screen:

| Chapter | Colourway | Content | The way out |
|---|---|---|---|
| Hero | on-dark (Void) | Orion constructs itself into the mark; headline, two calls to action | The mark's upper body, zoomed into until it is the whole screen |
| Measured | on-light (Starlight) | Four promises as figures, each linked to the lab section that proves it; the page's own live numbers | — |
| How we work | on-light | The belt: Measure, Build, Prove, rising at the belt's 1:2 slope | Mintaka, growing until the screen is its blue |
| The lab, the team | on-blue | The lab's live instruments; the founders; why "Mintaka" | Betelgeuse (a button: "Start a project"), growing until the screen is its rust |
| Start a project | on-rust | The brief and the direct contact | The page slides off the footer |
| Footer | on-dark | The wordmark, links, languages, build | — |

The header follows the chapter beneath it (on-dark → on-light → on-blue → on-rust), logo included:
the inlined logo's colours are classes (`lg-s`, `lg-b`, `lg-m`, see `svg::brand_classed`).

## Files

- `crates/site/src/pages/landing.rs` — the markup. Headings are split into word boxes in the HTML
  (`words()`), so they read normally with no script; `sky_data()` hands the hero's script the star
  catalogue (real positions fitted onto the logo's construction points, names, facts, the fit).
- `web/styles/sections/landing.css` — everything visual. Themes are `[data-theme]` blocks; all
  motion that hides or moves content sits under `.motion`.
- `web/studio/` — the one script (~6 kB compressed, first-party, no dependencies):
  - `studio.ts` — entry: adds `.js`, and `.motion` unless the reader prefers reduced motion.
  - `sky.ts` — the hero canvas: the construction, the star chart under the pointer, the zoom.
  - `astro.ts` — the projection, its inverse and the coordinate format (tested in `astro.test.ts`).
  - `scenes.ts` — one scroll loop: pinned scenes (CSS `position: sticky`), the header's colourway,
    the footer's wordmark. Native scrolling throughout; nothing hijacks the wheel.
  - `reveal.ts`, `magnetic.ts`, `readout.ts` — reveals and counters, buttons that lean towards the
    pointer, the page's own measurements (Resource and Navigation Timing; nothing is sent).

## Rules it keeps

- **Reads without the script.** Every word, link and portrait is in the HTML; without the script
  the mark is a still SVG and the chapters simply follow each other.
- **Motion off means off.** With `prefers-reduced-motion: reduce` there is no `.motion`: no
  construction, no pinned scenes, no reveals; the script only measures and colours the header.
- **No third-party code**, and the CSP stays as strict as the lab's (`script-src 'self'`, Trusted
  Types; the script sets styles through the CSSOM, never `style` attributes).
- **Budgets** (release builds fail over them): first screen ≤ 14 kB and whole page ≤ 32 kB
  compressed, page + script ≤ 40 kB, script ≤ 16 kB minified. The footer states the script's real
  compressed size, computed at build time.
- **Every number is sourced or live.** < 50 µs and 4,000 req/s: the production benchmark of
  2026-10-30 (docs/MILESTONE-4.md, 2-vCPU VM, 25–28 % CPU). < 14 kB: this page's own budget. The
  readout is measured in the visitor's browser.

## Previewing

`cargo xtask build`, then serve `dist/` so that `/mintaka/...` resolves (e.g. a directory holding a
`mintaka` symlink to `dist/`, served with `python3 -m http.server`). `MINTAKA_BASE=` builds for the
root instead.
