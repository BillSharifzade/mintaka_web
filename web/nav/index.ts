// "You are here" for the lab: the instrument under the reading line lights up in the rail, and
// the header's link to the instruments or the notes while either is on screen. The links already
// work without JavaScript; this only marks them.

import { $$ } from '../shared/dom.ts';

export function start(): void {
  const links = $$<HTMLAnchorElement>('.rail__link, .site-nav a, .menu__panel > nav a');
  const targets = [...new Set(links.map((a) => a.hash).filter((h) => h.length > 1))]
    .map((h) => document.getElementById(h.slice(1)))
    .filter((el): el is HTMLElement => !!el);
  if (!targets.length) return;
  const visible = new Set<string>();
  // Stations never overlap, so at most one of them is under the line (with its section).
  const mark = () => {
    for (const a of links) {
      if (visible.has(a.hash.slice(1))) a.setAttribute('aria-current', 'location');
      else a.removeAttribute('aria-current');
    }
  };
  // A reading line 40 % down the viewport: whatever crosses it is "here".
  const io = new IntersectionObserver(
    (entries) => {
      for (const e of entries) {
        if (e.isIntersecting) visible.add(e.target.id);
        else visible.delete(e.target.id);
      }
      mark();
    },
    { rootMargin: '-40% 0px -59% 0px' },
  );
  for (const t of targets) io.observe(t);
}
