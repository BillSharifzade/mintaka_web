// Constellation navigation (spec §7.2). The links already work without JavaScript; this adds
// "you are here": the star and header link of the section under the reading line light up, and
// hovering or focusing a star shows its name.

import { $, $$, h } from '../shared/dom.ts';

export function start(): void {
  const nav = $('.orion-nav');
  const headerLinks = $$<HTMLAnchorElement>('.site-nav a, .menu__panel a');
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
