# Accessibility audit — WCAG 2.2 AA (30 October 2026)

What was tested, how, what was wrong and how it was fixed, and what a person still has to test.
The page's own rules are in `docs/CONTRACTS.md` §10.1 (rule 5); this file is the evidence.

## How it was tested

- **Build:** the release site built for the root (`MINTAKA_BASE= cargo xtask build --release`),
  served by the release server over plain HTTP on loopback.
- **Browser:** headless Chrome 156, driven over the DevTools protocol from Node 26 with small
  scripts written for this audit (not in the repository). No third-party checker could be used here
  (no downloads), so every rule below was implemented and read directly:
  - **Contrast** for every visible text node: computed colour with ancestor opacity, over the
    composited background colours of its ancestors (4.5:1, or 3:1 from 24 px / 18.66 px bold), plus
    placeholders. Text over a background image, or over the hero's animated canvas, is flagged for a
    manual look instead of being scored.
  - **Non-text contrast** (1.4.11): form-field edges and fills against what surrounds them; the
    focus ring's colour against the background behind it, measured on every tab stop.
  - **Names, roles, landmarks, headings** from the accessibility tree
    (`Accessibility.getFullAXTree` and `getPartialAXTree`); heading order in document order.
  - **Images and SVG** (alt, names, `aria-hidden` on drawings), duplicate ids and broken
    `aria-labelledby` / `aria-describedby` / `for` / `#fragment` references.
  - **Target size** (2.5.8) of every visible control, with the inline-link and spacing exceptions.
  - **Reflow** (1.4.10): document width at 320 CSS px, and which element overflows.
  - **Text spacing** (1.4.12): the WCAG overrides injected, then clipped or overflowing text.
  - **Keyboard:** Tab pressed through each page (`Input.dispatchKeyEvent`); for every stop the
    focused element, its ring, and whether fixed parts of the page (header, proof console) cover it
    (`elementFromPoint` at its corners and centre). Scripted scenarios for the menu, the proof
    console, the terminal, the contact form and each instrument (activate with Enter, read the live
    regions, check where focus went).
  - **Pointer-only handlers:** `DOMDebugger.getEventListeners` on every element of every
    instrument, for click/pointer listeners on elements a keyboard cannot reach.
- **Pages:** `/en/`, `/en/privacy/`, `/en/sent/`, a 404 (`/en/no-such-page/`), `/en/lab/`,
  `/en/lab/contraction-hierarchies/`.
- **Viewports and settings:** 1440 × 900 and 320 × 640 (mobile emulation), each with
  `prefers-reduced-motion` on and off; reflow also at 320 px with a classic scrollbar (305 px of
  content). The home page was checked both on a fresh load and with every instrument mounted.

## Found and fixed

| WCAG | What was wrong | Fix | Where |
|---|---|---|---|
| 1.3.1, 4.1.2 | Sections below the first screens had `content-visibility: auto`. Chrome drops the contents of skipped sections from the accessibility tree ("notRendered"): on load at 320 px the tree held 5 headings and 12 links; every section from Efficiency down had no name, its instruments were not regions, and its headings, links and form fields were missing until scrolled into view. | Removed. | `web/styles/sections/layout.css`, `home.css` |
| 2.1.1, 2.4.3 | Instruments mount only when they scroll near, and an unmounted instrument has nothing focusable, so Tab jumped from the pillar cards straight to the proof wall, past S4–S7. | An instrument frame is a tab stop until its island has mounted; focusing it mounts the island, and once mounted and left, the frame leaves the tab order to the island's controls. | `crates/site/src/pages/mod.rs` (`instrument`), `web/main.ts` |
| 2.4.11 | The proof console (a non-modal panel) stayed open while focus moved on into the page and covered the focused element completely: 14 tab stops at 1440 px, 51 at 320 px. | The panel closes when focus lands anywhere outside it. 0 covered after the fix. | `web/hud/index.ts` |
| 2.4.3 | Closing the proof console sent focus to its pill in the corner, not to the button that opened it. | Focus returns to whatever opened it (the pill if that was the page itself). | `web/hud/index.ts` |
| 2.4.3, 2.1.1 | Focus fell to the top of the page after "Measure my distance" (the button was `disabled` while measuring) and after "Unleash the fuzzer" / "Stop" (the pressed button hides). | `aria-disabled` while measuring; focus moves to the button that takes the pressed one's place. | `web/instruments/physics.ts`, `web/instruments/fuzzer/index.ts` |
| 2.1.1 (Escape) | The header menu (`<details>`) did not close with Escape and stayed open over the page after choosing a section. | Escape closes it and returns focus to "Sections"; choosing a section, tabbing away or clicking elsewhere closes it. | `web/main.ts` |
| 2.2.2 | The hero's particle animation runs for as long as the hero is on screen; the only ways to stop it were the reduced-motion setting and the terminal's `sky` command. Live load updates its numbers every second and keeps its dots moving, with no control. | A visible "Pause the animation" / "Play the animation" button in the hero (shown only once the engine draws). Pause / Resume in Live load. | `crates/site/src/pages/home.rs`, `web/main.ts`, `web/styles/sections/hero.css`, `web/instruments/live.ts`, `web/styles/islands/live.css` |
| 1.4.10 | Horizontal scrolling at 320 px (document 344 px wide): the hero eyebrow's parts never wrapped (312 px), and the proof wall's "Open the proof console" button could not wrap inside its card. | Eyebrow parts and buttons may wrap on narrow screens. 320 px wide afterwards; only code blocks scroll, inside their own focusable region. | `web/styles/sections/hero.css`, `web/styles/components.css` |
| 1.4.11 | Text fields' edges were Slate on Void / Ink: 1.71:1. | Steel edges: 4.31:1 on Void, 3.97:1 on Ink (hover: Mist). Same for the fuzzer's input. | `web/styles/components.css`, `web/styles/islands/fuzzer.css` |
| 1.4.3 | The Telegram field's placeholder was Steel: 4.31:1. | A Steel–Mist mix: 5.84:1. | `web/styles/components.css` |
| 1.4.1 | Selected chips (contact form, routing, trace, fuzzer) and the selected Laws tab differed from the others by colour only. | A tick before the label of every selected chip and tab. | `web/styles/components.css` |
| 4.1.2, 3.3.1 | The console challenge's application form marked invalid fields with `aria-invalid=""`, which means false: screen readers heard no error state and the red border never appeared. | `aria-invalid="true"`, removed when fixed. | `web/console/ctf.ts` |
| 1.4.12 | At 320 px with the text-spacing overrides, the fuzzer's statistics ran up to 11 px out of their cells. | They may wrap. | `web/styles/islands/fuzzer.css` |
| 1.1.1 | The constellation navigation's SVG was exposed as an unnamed graphic around its links. | `role="none"` (the links name themselves). | `crates/site/src/pages/svg.rs` |

## Checked and in order

- **Language and titles:** `lang="en"` and `dir="ltr"`; the Tajik word in the name story is marked
  `lang="tg"`; every page has its own title.
- **Structure:** one `h1` per page, no skipped heading levels (document order), named landmarks
  (banner, two navigations, main, contentinfo, proof console as complementary, sections and
  instruments as regions; the small-screen menu's navigation is named after the menu and its
  language switch is a group, so no two landmarks share a name), no duplicate ids, no broken
  references.
- **Names:** no unnamed control on any page; canvases and drawings are `aria-hidden` with the
  meaning in text, tables or live regions next to them.
- **Text contrast:** no failures on any tested page after the fixes (the fuzzer's star names are
  transparent until lit; lit, they are Mist on the sky, about 12:1).
- **Focus:** a visible ring on every tab stop (Mintaka blue, 4.72:1 on Ink, 5.13:1 on Void); the
  skip link appears on top of the header; the sticky header never hid a focused element
  (`scroll-padding-top`).
- **Forms:** labels, hints and errors tied with `aria-describedby`; after a failed submit, focus goes
  to the first invalid field and a `role="status"` line lists what to fix; the proof of work runs
  by itself, so there is no puzzle to solve (3.3.8); nothing is asked twice (3.3.7).
- **Status messages (4.1.3):** polite live regions for physics, routing, the fuzzer, armour, trace,
  Live load (losses and recoveries only), the overload lab (`<output>`), the Laws trials, the
  contact form and the application form.
- **Instruments by keyboard:** routing presets and modes, the fuzzer and its "try it yourself" form,
  the armour and trace details, the overload lab's slider (arrow keys, `aria-valuetext`), the
  Laws tabs (arrow keys, roving tabindex). Hick–Hyman answers with number keys or arrows and Enter;
  pop-out search with quarters 1–4 or arrows and Enter, explained in the field's description.
- **Dialogs:** the terminal is a modal `<dialog>`: focus goes to its prompt, Escape closes it,
  focus returns to the opener; Tab completes a command only when something is typed, so it still
  moves focus out of an empty prompt (the keys are described to screen readers).
- **Target size (2.5.8):** nothing under 24 × 24 px. Below the house goal of 44 px but within WCAG:
  the constellation's stars (24 px), Lab post titles (33 px tall), team links (38 px), footer links
  (28–32 px).
- **Dragging (2.5.7):** nothing needs a drag; the slider also takes clicks and keys.
- **Consistent help (3.2.6):** "Start a project" sits in the same place in the header of every page.
- **Content on hover or focus (1.4.13):** nothing appears only on hover; the pillar cards' star
  highlight reacts to focus as well, and is decoration.
- **Reduced motion (2.3.3):** with the setting on, no CSS animation runs, the hero shows its still
  poster and the instruments draw their end states.
- **Lab pages:** no scripts; tables, formulas and code that scroll on their own are focusable
  regions with names.

## Known limits

- **Routing map:** choosing your own start and destination needs a pointer (tap a map twice). By
  keyboard there are the station → airport route and random pairs, which show the same race. A
  keyboard way in would be a focusable map with an arrow-key cursor and Enter to set each point.
- **Lab pages** keep the plain `<details>` menu (no scripts there), so Escape does not close it.
- **The proof console's pill** is fixed to the bottom-right corner and can sit over a little of the
  content behind it; it never hid a focused element in the tests.
- **Text over the hero animation** could not be measured automatically; a visual check found the
  particles dark enough behind the text, but a person should look at it on a bright display.
- **Not covered here:** the contact form's "Continue in Telegram" success link, added after this
  snapshot.

## What a person should test first

No screen reader could be run here: no NVDA, JAWS, VoiceOver or TalkBack pass has been done. In
order of risk:

1. **Headings and landmarks lists** on the home page with NVDA + Chrome and VoiceOver + Safari:
   every section and instrument should now be listed on load.
2. **Tabbing through the instruments** with a screen reader: the frame is announced as a region,
   the next Tab enters the instrument; nothing should be announced twice.
3. **Announcements:** physics, a routing race, a fuzzer chapter, a Laws block. They should be heard
   once, politely, and Live load should stay quiet while it works.
4. **The contact form:** errors read on focus, the status line after a failed submit, the wait
   while the proof of work runs, the success screen.
5. **The proof console and the terminal:** open with P and ~, close with Escape, focus back where
   it was; the console's raw headers.
6. **Hick–Hyman and pop-out search** with a screen reader running: the reaction time includes the
   screen reader's speech, so check the instructions make that clear enough.
7. **Phones:** TalkBack and VoiceOver on the header menu, the chips (visually hidden inputs) and
   the Laws tabs; 320 px with the system font size raised.
8. **Windows contrast themes** (forced colours) and desktop zoom at 400 %.
