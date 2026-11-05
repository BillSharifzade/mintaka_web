/**
 * Things rise into view once, as they arrive: blocks (`data-reveal`) fade up, headings
 * (`data-split`, whose words are already boxed in the HTML) rise word by word, and the big figures
 * (`data-count`) count up to the number the HTML already shows. What arrives together is staggered
 * in reading order.
 */

import { easeOutExpo, numberLocale } from './math.ts';

export function reveal(lang: string): void {
  const fmt = new Intl.NumberFormat(numberLocale(lang));
  const io = new IntersectionObserver(
    (entries) => {
      const arriving = entries.filter((e) => e.isIntersecting).map((e) => e.target as HTMLElement);
      arriving.forEach((el, i) => {
        const delay = Math.min(i, 6) * 80;
        if (delay) el.style.transitionDelay = `${delay}ms`;
        el.classList.add('in');
        io.unobserve(el);
        for (const n of el.querySelectorAll<HTMLElement>('[data-count]')) count(n, fmt, delay + 120);
        // Once it has played, the delay must not slow the element's hover transitions.
        if (delay) setTimeout(() => (el.style.transitionDelay = ''), 1300 + delay);
      });
    },
    { rootMargin: '0px 0px -10% 0px', threshold: 0.1 },
  );
  for (const el of document.querySelectorAll<HTMLElement>('[data-reveal], [data-split]')) io.observe(el);
}

/** Count up to the figure in the HTML — only if the browser would write it the same way. */
function count(el: HTMLElement, fmt: Intl.NumberFormat, delay: number): void {
  const target = Number(el.dataset['count']);
  if (!Number.isFinite(target) || target <= 0 || fmt.format(target) !== el.textContent) return;
  const duration = 1100;
  let start = 0;
  el.textContent = fmt.format(0);
  // One text node, rewritten in place: no new node a frame.
  const text = el.firstChild as Text;
  const step = (now: number) => {
    if (!start) start = now + delay;
    const t = Math.min(1, Math.max(0, (now - start) / duration));
    text.data = fmt.format(Math.round(target * easeOutExpo(t)));
    if (t < 1) requestAnimationFrame(step);
  };
  requestAnimationFrame(step);
}
