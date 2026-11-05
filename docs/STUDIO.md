# The studio landing page

`/{lang}/studio/` (en, ru, tg) is the company's front door; the rest of the site is the lab that
proves what it says. It has its own document, stylesheet and one script, and shares none of the
lab's chrome.

## The idea: the mark unfolds

The page is the Mintaka mark, opened up. Each chapter is one of the mark's colourways from the
brand kit, and each change of chapter is a part of the mark growing to fill the screen:

| Chapter | Colourway | Content | The way out |
|---|---|---|---|
| Hero | on-dark (Void) | Orion constructs itself into the mark; headline, two calls to action; a star chart under the pointer | The camera flies into the mark's upper body until it is the whole screen |
| Measured | on-light (Starlight) | Four promises as figures, each under its star (as in the lab), with a small chart of Orion as their legend; the page's own live numbers | — |
| How we work | on-light | The belt: Measure, Build, Prove, rising at the belt's 1:2 slope | Mintaka, growing until the screen is its blue |
| The lab, the team | on-blue | The lab's live instruments; the founders and their sites; why "Mintaka" | — |
| Start a project | on-rust | The brief and the direct contact | The page slides off the footer |
| Footer | on-dark | The wordmark (its letters rise; Mintaka, the i's dot, lands last), links, languages, build | — |

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
  - `sky.ts` — the hero canvas: the construction, the star chart under the pointer (it locks onto
    a star anywhere on its dot, and turns Void over the mark's body), and the flight into the
    mark: a sky in depth (some stars between us and the mark stream past in streaks while the
    scroll is fast), a slight bank, the body's own faint stars on the way in. It draws nothing
    once the body covers the screen, and the idle twinkle runs at 20 frames a second.
  - `astro.ts` — the projection, its inverse and the coordinate format (tested in `astro.test.ts`).
  - `scenes.ts` — one scroll loop: pinned scenes (CSS `position: sticky`), the header's colourway,
    the footer's wordmark (`.in` once a third of it shows). Native scrolling throughout; nothing
    hijacks the wheel. Measuring reads everything before it writes anything.
  - `reveal.ts`, `readout.ts` — reveals and counters, the page's own measurements (Resource and
    Navigation Timing; nothing is sent).

## Rules it keeps

- **Reads without the script.** Every word, link and portrait is in the HTML; without the script
  the mark is a still SVG and the chapters simply follow each other.
- **Motion off means off.** With `prefers-reduced-motion: reduce` there is no `.motion`: no
  construction, no pinned scenes, no reveals; the script only measures and colours the header.
- **No third-party code**, and the CSP stays as strict as the lab's (`script-src 'self'`, Trusted
  Types; the script sets styles through the CSSOM, never `style` attributes).
- **Budgets** (release builds fail over them): first screen ≤ 14 kB and whole page ≤ 32 kB
  compressed, page + script ≤ 40 kB, script ≤ 17 kB minified. The footer states the script's real
  compressed size, computed at build time.
- **Lean by construction.** The page's stylesheet drops the shared tokens it never reads
  (`css::prune_custom_properties`). Pages not written in Cyrillic draw their few Russian and Tajik
  words (the language names, the Tajik word in the name story) from small cuts of Onest
  (`fonts.rs`, "Onest Names"): 8.6 kB instead of 26 kB on English pages, 2.5 kB instead of 10.5 kB
  on Russian ones, with identical outlines; a letter the cuts lack falls through to the full font.
  The Latin fonts are cut the same way to the characters the site writes (Space Grotesk 15 kB
  instead of 22, IBM Plex Mono 9.7 instead of 14.7), declared after their full files so browsers
  try them first. Code and key styles live in `base-code.css`, which only the lab imports.
- **Every screen.** Pinned scenes use the small viewport (`svh`), so a phone's toolbar never
  covers them. The hero has its own arrangements for short phones held upright, phones held
  sideways (words and mark side by side), tablets held upright (mark above the words) and short
  desktop windows; long Russian step titles shrink only when a short screen narrows the staircase.
- **Every number is sourced or live.** < 50 µs and 4,000 req/s: the production benchmark of
  2026-10-30 (docs/MILESTONE-4.md, 2-vCPU VM, 25–28 % CPU). < 14 kB: this page's own budget. The
  readout is measured in the visitor's browser.

## Previewing

`cargo xtask build`, then serve `dist/` so that `/mintaka/...` resolves (e.g. a directory holding a
`mintaka` symlink to `dist/`, served with `python3 -m http.server`). `MINTAKA_BASE=` builds for the
root instead.
