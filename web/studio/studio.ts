/**
 * The studio landing page's one script: first-party, no dependencies, loaded as a module after the
 * HTML. Everything it does is an enhancement — the page reads completely without it, and with
 * motion turned off it only measures the page and keeps the header's colours right.
 *
 *   sky.ts       the hero: Orion constructs itself into the mark; a star chart under the pointer
 *   scenes.ts    one scroll loop: the pinned scenes, the header's colourway, the footer
 *   reveal.ts    blocks and words rising into view, figures counting up
 *   magnetic.ts  buttons that lean towards the pointer
 *   readout.ts   this page's own numbers, from the browser's timing records
 */

import { magnetic } from './magnetic.ts';
import { readout } from './readout.ts';
import { reveal } from './reveal.ts';
import { beltScene, heroScene, run, type Scene } from './scenes.ts';
import { sky } from './sky.ts';

const root = document.documentElement;
const lang = root.lang || 'en';
const motion = !matchMedia('(prefers-reduced-motion: reduce)').matches;
// Without a script the still mark fades in at 1.1 s; arriving later than that, skip the construction
// rather than take away a mark the reader is already looking at.
const intro = performance.now() < 1000;
const header = document.querySelector<HTMLElement>('.hd');

root.classList.add('js');
readout(lang);

if (motion) {
  root.classList.add('motion');
  const fine = matchMedia('(hover: hover) and (pointer: fine)').matches;
  const scenes: Scene[] = [];
  const hero = document.querySelector<HTMLElement>('[data-scene="hero"]');
  const pin = hero?.querySelector<HTMLElement>('.hero__pin');
  const canvas = hero?.querySelector<HTMLCanvasElement>('.hero__sky');
  const fig = hero?.querySelector<HTMLElement>('.hero__fig');
  if (hero && pin && canvas && fig) {
    const chart = sky(pin, canvas, fig, { intro, fine });
    scenes.push(heroScene(hero, chart));
    // The sky's twinkle never ends on its own, so the reader can stop it (WCAG 2.2.2).
    const pause = hero.querySelector<HTMLButtonElement>('.pause');
    if (pause) {
      pause.hidden = false;
      pause.addEventListener('click', () => {
        const paused = !pause.hasAttribute('data-paused');
        pause.toggleAttribute('data-paused', paused);
        pause.setAttribute('aria-label', (paused ? pause.dataset['play'] : pause.dataset['pause']) ?? '');
        chart.setPaused(paused);
      });
    }
  }
  const belt = document.querySelector<HTMLElement>('[data-scene="belt"]');
  if (belt) scenes.push(beltScene(belt));
  run(header, scenes);
  reveal(lang);
  if (fine) magnetic();
  // The pinned tracks have just grown the page: return to the anchor the link pointed at.
  if (location.hash.length > 1) document.getElementById(decodeURIComponent(location.hash.slice(1)))?.scrollIntoView();
} else {
  run(header, []);
}
