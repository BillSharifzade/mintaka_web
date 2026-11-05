/**
 * In-page links ("Start a project" in the hero and the header) glide to their section. The browser's
 * own smooth scroll bursts off and coasts: on this page, whose pinned scenes make it long, it covers
 * 60 % of the ~9,000 px to the brief in its first half second. The glide eases in and out evenly
 * instead, over 0.8–1.9 s (longer for longer distances), so the reader sees the page pass.
 *
 * Everything else stays the browser's: the address gains the fragment before the glide (so Back
 * returns to where the reader was), and on arrival a fragment navigation of the browser's own sets
 * the exact place and moves the keyboard's starting point to the section. Any wheel, touch, press or
 * key hands the scroll back to the reader at once. The skip link is left alone; with reduced motion
 * this never loads and the jump is instant.
 */

import { clamp, easeInOutSine, glideMs } from './math.ts';

const INPUT = ['wheel', 'touchstart', 'pointerdown', 'keydown'] as const;

export function glide(): void {
  const root = document.documentElement;
  let stop: (() => void) | undefined;
  document.addEventListener('click', (e) => {
    const a = (e.target as Element).closest?.<HTMLAnchorElement>('a[href^="#"]:not(.skip)');
    if (!a || e.defaultPrevented || e.button || e.metaKey || e.ctrlKey || e.shiftKey || e.altKey) return;
    const el = document.getElementById(decodeURIComponent(a.hash.slice(1)));
    if (!el) return;
    e.preventDefault();
    stop?.();
    if (location.hash !== a.hash) history.pushState(null, '', a.hash);
    // Where the browser itself puts the section: under the header, by the page's scroll padding.
    const off = parseFloat(getComputedStyle(root).scrollPaddingTop) || 0;
    // Measured each frame, in case the page settles while it moves.
    const to = () => Math.min(el.getBoundingClientRect().top + scrollY - off, root.scrollHeight - innerHeight);
    const from = scrollY;
    const ms = glideMs(Math.abs(to() - from));
    const t0 = performance.now();
    let raf = 0;
    const cancel = () => {
      cancelAnimationFrame(raf);
      for (const t of INPUT) removeEventListener(t, cancel);
      stop = undefined;
    };
    const step = (now: number) => {
      const k = clamp((now - t0) / ms);
      scrollTo({ top: from + (to() - from) * easeInOutSine(k), behavior: 'instant' });
      if (k < 1) raf = requestAnimationFrame(step);
      else {
        cancel();
        location.replace(a.hash);
      }
    };
    for (const t of INPUT) addEventListener(t, cancel, { passive: true });
    stop = cancel;
    raf = requestAnimationFrame(step);
  });
}
