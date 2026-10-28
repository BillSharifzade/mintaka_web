// Fitts's law (1954): movement time grows with the index of difficulty ID = log2(D/W + 1).
// 15 targets from a fixed design (three widths × five IDs from 1 to 5 bits), in random order,
// each placed at its distance from the last one when the arena allows (the ID is computed from
// where it really landed). Movement time runs from one pointerdown to the next, event.timeStamp
// to event.timeStamp. Mouse, touch and pen alike; keyboard users are told what this measures.

import { h, render } from '../../shared/dom.ts';
import { fill, fmt, L, num, P, trials, type Experiment, type Item } from './kit.ts';
import { plotBox, scatter } from './plot.ts';
import { fit, fittsId, shuffle, throughput, type Pt } from './stats.ts';

/** [width px, ID bits]: widths 64, 40 and 24 px — the last is WCAG 2.2's 24 px minimum target. */
export const DESIGN: readonly (readonly [number, number])[] = [
  [64, 1],
  [40, 2],
  [24, 3],
].flatMap(([w, id]) => [0, 0.5, 1, 1.5, 2].map((k) => [w!, id! + k] as const));

export interface Spot {
  x: number;
  y: number;
}

/**
 * Where a target of radius `r` goes in a `w`×`h` arena: `d` away from `p` in a random direction
 * if one fits, else toward the farthest corner, as far as `d` allows. Always fully inside.
 */
export function place(p: Spot, d: number, r: number, w: number, h: number, random = Math.random): Spot {
  const inside = (x: number, y: number) => x >= r && x <= w - r && y >= r && y <= h - r;
  const a0 = random() * 2 * Math.PI;
  for (let i = 0; i < 36; i++) {
    const x = p.x + d * Math.cos(a0 + (i * Math.PI) / 18);
    const y = p.y + d * Math.sin(a0 + (i * Math.PI) / 18);
    if (inside(x, y)) return { x, y };
  }
  const cx = p.x < w / 2 ? w - r : r;
  const cy = p.y < h / 2 ? h - r : r;
  const k = Math.min(1, d / (Math.hypot(cx - p.x, cy - p.y) || 1));
  const x = p.x + (cx - p.x) * k;
  const y = p.y + (cy - p.y) * k;
  return { x: Math.min(Math.max(x, r), w - r), y: Math.min(Math.max(y, r), h - r) };
}

const START = 56; // diameter of the start circle

export const fitts: Experiment = (panel, api) => {
  const dot = h('div', { class: 'laws-dot is-start' });
  const go = h('p', { class: 'laws-go' }, L('fitts-go'));
  const count = h('p', { class: 'laws-count' });
  const arena = h('div', { class: 'laws-arena grid-bg', 'aria-hidden': 'true' }, dot, go, count);
  const svg = plotBox();
  const side = h('div', { class: 'laws-side' }, svg);
  const said = h('p', { class: 'laws-said', 'aria-live': 'polite' });

  let running = false;
  let order: (readonly [number, number])[] = [];
  let data: Pt[] = [];
  let misses = 0;
  let last = 0;
  let cur = { x: 0, y: 0, w: START, id: 0 };

  const put = (x: number, y: number, w: number) => {
    dot.style.setProperty('--x', `${x}px`);
    dot.style.setProperty('--y', `${y}px`);
    dot.style.setProperty('--d', `${w}px`);
  };

  const plot = () => {
    const f = fit(data);
    scatter(svg, { pts: data, fit: f, xMax: 6, x: L('fitts-x'), y: 'ms' });
    return f;
  };

  const idle = (why = '') => {
    running = false;
    arena.classList.remove('is-run');
    dot.classList.add('is-start');
    dot.style.removeProperty('--x');
    dot.style.removeProperty('--y');
    dot.style.setProperty('--d', `${START}px`);
    go.hidden = false;
    count.textContent = '';
    if (why) said.textContent = why;
  };

  const next = (aw: number, ah: number) => {
    const [width, id] = order[data.length]!;
    const spot = place(cur, width * (2 ** id - 1), width / 2, aw, ah);
    cur = { ...spot, w: width, id: fittsId(Math.hypot(spot.x - cur.x, spot.y - cur.y), width) };
    put(spot.x, spot.y, width);
    count.textContent = [L('count', { n: data.length + 1, total: order.length }), misses && P('misses', misses)]
      .filter(Boolean)
      .join(' · ');
  };

  const finish = () => {
    const f = plot();
    idle();
    if (!f) return;
    const items: Item[] = [
      [L('a'), fmt(f.a), 'ms'],
      [L('b'), fmt(f.b), 'ms/bit'],
      [L('r2'), fmt(f.r2, 2), ''],
      [L('tp'), fmt(throughput(data), 1), 'bit/s'],
    ];
    side.replaceChildren(
      svg,
      trials(
        [L('fitts-x'), L('col-ms')],
        data.map(([x, y]) => [fmt(x, 2), fmt(y)]),
      ),
    );
    said.textContent = '';
    api.report({
      items,
      plain: f.b >= 1 ? fill('fitts-plain', { b: num(fmt(f.b), 'ms') }) : undefined,
      line: L('fitts-line'),
      note: f.r2 < 0.5 ? L('fitts-loose') : undefined,
    });
  };

  arena.addEventListener('pointerdown', (e: PointerEvent) => {
    if (!e.isPrimary || e.button !== 0) return;
    const r = arena.getBoundingClientRect();
    const x = e.clientX - r.left;
    const y = e.clientY - r.top;
    if (!running) cur = { x: r.width / 2, y: r.height / 2, w: START, id: 0 };
    const hit = Math.hypot(x - cur.x, y - cur.y) <= cur.w / 2 + 3;
    if (running) e.preventDefault(); // no text selection or focus changes mid-run
    if (!hit) {
      if (running) misses++;
      return;
    }
    if (!running) {
      running = true;
      order = shuffle([...DESIGN]);
      data = [];
      misses = 0;
      side.replaceChildren(svg);
      plot();
      arena.classList.add('is-run');
      dot.classList.remove('is-start');
      go.hidden = true;
      said.textContent = '';
    } else {
      data.push([cur.id, e.timeStamp - last]);
      plot();
      if (data.length === order.length) return finish();
    }
    last = e.timeStamp;
    next(r.width, r.height);
  });

  render(panel);
  panel.append(
    h('p', null, L('fitts-intro')),
    h(
      'p',
      { class: 'laws-note' },
      L('fitts-keys'),
      ' ',
      h('button', { class: 'btn btn--quiet btn--sm', type: 'button', onclick: () => api.next() }, L('fitts-skip')),
    ),
    h('div', { class: 'laws-row' }, arena, side),
    said,
  );
  idle();
  plot();
  return (why?: string) => {
    if (running) idle(why);
  };
};
