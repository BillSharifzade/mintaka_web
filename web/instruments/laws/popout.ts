// Pop-out (Treisman & Gelade, 1980): a star that differs from all the others by one feature —
// colour, or shape — is found at a glance; one defined by a combination of colour and shape has
// to be searched for. 60 stars, 5 trials per condition, in the order colour → shape → both.
// Tap the odd star, or answer with its quarter of the field (keys 1–4, or the arrow keys and
// Enter). Timed from the animation frame that painted the field to the answer's event.timeStamp.

import { h, render, s } from '../../shared/dom.ts';
import { avg, fill, fmt, foreperiod, frame, L, num, P, slot, type Experiment, type Item } from './kit.ts';
import { bars, plotBox } from './plot.ts';
import { shuffle } from './stats.ts';

type Cond = 'colour' | 'shape' | 'both';
const CONDS: readonly Cond[] = ['colour', 'shape', 'both'];
const PER = 5;
const STARS = 60;
const SLOW = 5000;

/** A star: a circle or a diamond, plain or in the odd colour (CSS: .laws-star.is-odd). */
function star(x: number, y: number, r: number, diamond: boolean, odd: boolean): SVGElement {
  const cls = odd ? 'laws-star is-odd' : 'laws-star';
  if (!diamond) return s('circle', { class: cls, cx: x, cy: y, r });
  const d = r * 1.3;
  return s('path', { class: cls, d: `M${x} ${y - d}L${x + d} ${y}L${x} ${y + d}L${x - d} ${y}Z` });
}

/** The target of each condition, as drawn on the bars: shape and colour, never colour alone. */
const MARK: Record<Cond, readonly [diamond: boolean, odd: boolean]> = {
  colour: [false, true],
  shape: [true, false],
  both: [true, true],
};

export const popout: Experiment = (panel, api) => {
  const field = s('svg', { class: 'laws-field', 'aria-hidden': 'true', focusable: 'false' });
  const keys = h('p', { class: 'visually-hidden', id: 'laws-pop-keys' }, L('pop-area'));
  const arena = h(
    'div',
    { class: 'laws-arena laws-search grid-bg', tabindex: '0', role: 'group', 'aria-label': L('pop'), 'aria-describedby': 'laws-pop-keys' },
    field,
  );
  const start = h('button', { class: 'btn btn--ghost btn--sm', type: 'button', onclick: () => void run() }, L('start'));
  const count = h('p', { class: 'laws-count' });
  const svg = plotBox();
  const said = h('p', { class: 'laws-said', 'aria-live': 'polite' });
  const answer = slot<readonly [right: boolean, at: number]>();
  const quadRect = s('rect', { class: 'laws-quad', hidden: true });
  let target = { x: 0, y: 0, q: 0, reach: 0 };
  let quad = -1;
  let w = 0;
  let hh = 0;
  let token = 0;

  /** The quarter lines, their numbers (shown to keyboard users) and the keyboard highlight. */
  const guides = (): SVGElement[] => [
    s('path', { class: 'laws-guide', d: `M${w / 2} 0V${hh}M0 ${hh / 2}H${w}` }),
    ...[1, 2, 3, 4].map((n, i) =>
      s('text', { class: 'laws-qn', x: i % 2 ? w - 8 : 8, y: i < 2 ? 18 : hh - 8, 'text-anchor': i % 2 ? 'end' : 'start' }, String(n)),
    ),
    quadRect,
  ];

  const highlight = (q: number) => {
    quad = q;
    quadRect.toggleAttribute('hidden', q < 0);
    if (q >= 0) {
      for (const [k, v] of Object.entries({ x: q % 2 ? w / 2 : 0, y: q > 1 ? hh / 2 : 0, width: w / 2, height: hh / 2 })) {
        quadRect.setAttribute(k, String(v));
      }
    }
  };

  const measure = () => {
    const r = arena.getBoundingClientRect();
    w = Math.round(r.width);
    hh = Math.round(r.height);
    field.setAttribute('viewBox', `0 0 ${w} ${hh}`);
  };

  const fixation = () => {
    highlight(-1);
    render(field, guides(), s('text', { class: 'laws-fix', x: w / 2, y: hh / 2 + 6, 'text-anchor': 'middle' }, '+'));
  };

  const draw = (cond: Cond) => {
    const cols = w >= hh ? 10 : 6;
    const rows = STARS / cols;
    const cw = w / cols;
    const ch = hh / rows;
    const cell = Math.min(cw, ch);
    const r = Math.max(4, Math.min(8, cell * 0.16));
    const jitter = Math.max(0, cell / 2 - r * 1.3 - 3) * 0.8;
    const at = Math.floor(Math.random() * STARS);
    const kinds = shuffle(Array.from({ length: STARS - 1 }, (_, i) => i % 2 === 0)); // "both": half each
    const kids = guides();
    for (let i = 0, j = 0; i < STARS; i++) {
      const c = i % cols;
      const row = Math.floor(i / cols);
      const x = (c + 0.5) * cw + (Math.random() * 2 - 1) * jitter;
      const y = (row + 0.5) * ch + (Math.random() * 2 - 1) * jitter;
      let [diamond, odd] = [false, false];
      if (i === at) {
        [diamond, odd] = MARK[cond];
        target = { x, y, q: (row >= rows / 2 ? 2 : 0) + (c >= cols / 2 ? 1 : 0), reach: cell / 2 };
      } else if (cond === 'both') {
        diamond = kinds[j++]!;
        odd = !diamond;
      }
      kids.push(star(x, y, r, diamond, odd));
    }
    render(field, kids);
  };

  arena.addEventListener('pointerdown', (e: PointerEvent) => {
    if (!e.isPrimary || e.button !== 0 || !answer.open) return;
    e.preventDefault();
    const r = arena.getBoundingClientRect();
    answer.give([Math.hypot(e.clientX - r.left - target.x, e.clientY - r.top - target.y) <= target.reach, e.timeStamp]);
  });
  arena.addEventListener('keydown', (e: KeyboardEvent) => {
    const k = e.key;
    let q = /^[1-4]$/.test(k) ? Number(k) - 1 : -1;
    if (k === 'Enter' || k === ' ') q = quad;
    else if (k.startsWith('Arrow')) {
      e.preventDefault();
      const from = Math.max(quad, 0);
      highlight(k === 'ArrowRight' ? from | 1 : k === 'ArrowLeft' ? from & 2 : k === 'ArrowDown' ? from | 2 : from & 1);
      return;
    }
    if (q < 0 || !answer.open) return;
    e.preventDefault();
    answer.give([q === target.q, e.timeStamp]);
  });

  const plot = (rts: Record<Cond, number[]>) => {
    const short: Record<Cond, string> = { colour: L('pop-colour'), shape: L('pop-shape'), both: L('pop-both-short') };
    bars(
      svg,
      CONDS.map((c) => ({ label: short[c], value: avg(rts[c]), mark: (x: number, y: number) => star(x, y, 5, ...MARK[c]) })),
      'ms',
    );
  };

  const idle = (why = '') => {
    token++;
    answer.close();
    start.hidden = false;
    count.textContent = '';
    said.textContent = why;
    if (w) fixation();
  };

  async function run(): Promise<void> {
    const me = ++token;
    const rts: Record<Cond, number[]> = { colour: [], shape: [], both: [] };
    let wrong = 0;
    start.hidden = true;
    said.textContent = '';
    arena.focus();
    measure();
    plot(rts);
    for (const cond of CONDS) {
      for (let k = 1; k <= PER; k++) {
        count.textContent = [L(`pop-${cond}`), L('count', { n: k, total: PER }), wrong && P('wrong', wrong)]
          .filter(Boolean)
          .join(' · ');
        fixation();
        await foreperiod();
        if (me !== token) return;
        draw(cond);
        const got = answer.wait();
        const onset = await frame();
        const [right, at] = await got;
        if (me !== token) return;
        const rt = at - onset;
        if (!right) wrong++;
        else if (rt > 0 && rt < SLOW) rts[cond].push(rt);
        plot(rts);
      }
    }
    idle();
    start.textContent = L('again');
    const items: Item[] = CONDS.map((c) => {
      const m = avg(rts[c]);
      return [L(`pop-${c}`), m === null ? '—' : fmt(m), m === null ? '' : 'ms'];
    });
    const colour = avg(rts.colour);
    const both = avg(rts.both);
    api.report({
      items,
      plain: colour && both ? fill('pop-plain', { x: num(fmt(both / colour, 1), '×') }) : undefined,
      line: L('pop-line'),
    });
    start.focus();
  }

  panel.append(
    h('p', null, L('pop-intro')),
    keys,
    h('div', { class: 'laws-row' }, h('div', { class: 'laws-stage' }, arena, count, start), h('div', { class: 'laws-side' }, svg)),
    said,
  );
  measure();
  fixation();
  plot({ colour: [], shape: [], both: [] });
  return (why?: string) => {
    if (start.hidden) idle(why);
  };
};
