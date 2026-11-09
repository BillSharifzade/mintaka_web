// Constellation navigation (spec §7.2). The links already work without JavaScript; this adds
// "you are here": the star and header link of the section under the reading line light up, and
// hovering or focusing a star shows its name.

import { $, $$, h } from '../shared/dom.ts';

export function start(): void {
  const nav = $('.orion-nav');
  // Section links only: the menu also holds the language switch, whose aria-current is the page's.
  const headerLinks = $$<HTMLAnchorElement>('.site-nav a, .menu__panel > nav:not(.lang) a');
  const stars = $$<SVGAElement>('.orion-nav__star');
  const ids = ['efficiency', 'security', 'highload', 'design', 'process', 'team'];
  const sections = ids.map((id) => document.getElementById(id)).filter((s): s is HTMLElement => !!s);

  const mark = (id: string | null) => {
    for (const a of headerLinks) {
      if (id && a.hash === `#${id}`) a.setAttribute('aria-current', 'location');
      else a.removeAttribute('aria-current');
    }
    for (const s of stars) {
      if (id && s.dataset['section'] === id) s.setAttribute('aria-current', 'location');
      else s.removeAttribute('aria-current');
    }
  };

  // A reading line 40 % down the viewport: the section crossing it is "current".
  const visible = new Set<string>();
  const io = new IntersectionObserver(
    (entries) => {
      for (const e of entries) {
        if (e.isIntersecting) visible.add(e.target.id);
        else visible.delete(e.target.id);
      }
      mark(ids.find((id) => visible.has(id)) ?? null);
    },
    { rootMargin: '-40% 0px -59% 0px' },
  );
  for (const s of sections) io.observe(s);

  // The room under the fixed parts: the header (at its bottom edge) and the Orion navigation (at
  // the middle of the screen) take the bench's colours while the lit bench is under them. The
  // bench is measured on load and resize, so scrolling reads no layout.
  const bench = $('.bench');
  const header = $('.site-header');
  if (bench) {
    let top = 0;
    let bottom = 0;
    let headerH = 0;
    const paint = () => {
      const y = scrollY;
      const under = (line: number) => line >= top && line < bottom;
      header?.classList.toggle('lit', under(y + headerH));
      nav?.classList.toggle('lit', under(y + innerHeight / 2));
    };
    const measure = () => {
      const r = bench.getBoundingClientRect();
      top = r.top + scrollY;
      bottom = r.bottom + scrollY;
      headerH = header?.offsetHeight ?? 0;
      paint();
    };
    let queued = false;
    addEventListener('scroll', () => {
      if (queued) return;
      queued = true;
      requestAnimationFrame(() => ((queued = false), paint()));
    }, { passive: true });
    new ResizeObserver(measure).observe(document.body);
    measure();
  }

  if (!nav) return;
  const label = h('span', { class: 'orion-nav__label', hidden: true });
  nav.append(label);
  const show = (star: SVGAElement) => {
    const name = star.getAttribute('aria-label') ?? '';
    const sr = star.getBoundingClientRect();
    const nr = nav.getBoundingClientRect();
    label.textContent = name;
    label.style.setProperty('top', `${sr.top + sr.height / 2 - nr.top}px`);
    label.hidden = false;
  };
  const hide = () => (label.hidden = true);
  for (const s of stars) {
    s.addEventListener('pointerenter', () => show(s));
    s.addEventListener('pointerleave', hide);
    s.addEventListener('focus', () => show(s));
    s.addEventListener('blur', hide);
  }
}
