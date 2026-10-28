// Hick–Hyman law (Hick 1952, Hyman 1953): choice reaction time grows with the information in
// the choice, log2(n + 1) bits. Blocks of 8 trials with 2, 4 and 8 options, in random order.
// After a random foreperiod a target shape and the options appear together; the reaction time
// runs from the animation frame that painted them to the answer's event.timeStamp (pointerdown,
// a number key, arrow keys + Enter, or a click from assistive technology).

import { h, render, s } from '../../shared/dom.ts';
import { avg, fill, fmt, foreperiod, frame, L, num, P, slot, trials, type Experiment, type Item } from './kit.ts';
import { plotBox, scatter } from './plot.ts';
import { fit, hickBits, shuffle, type Pt } from './stats.ts';

/** Eight shapes on a 24-unit grid, told apart by outline alone (no colour). */
export const SHAPES: Record<string, string> = {
  circle: 'M12 4a8 8 0 1 1 0 16a8 8 0 1 1 0-16z',
  ring: 'M12 3a9 9 0 1 1 0 18a9 9 0 1 1 0-18zm0 4a5 5 0 1 0 0 10a5 5 0 1 0 0-10z',
  square: 'M5 5h14v14H5z',
  triangle: 'M12 3l10 17H2z',
  diamond: 'M12 2l10 10-10 10L2 12z',
  plus: 'M9.5 3h5v6.5H21v5h-6.5V21h-5v-6.5H3v-5h6.5z',
  cross: 'M5.6 2.1L12 8.5l6.4-6.4 3.5 3.5-6.4 6.4 6.4 6.4-3.5 3.5-6.4-6.4-6.4 6.4-3.5-3.5 6.4-6.4z',
  hexagon: 'M12 2l8.7 5v10L12 22l-8.7-5V7z',
};
const NAMES = Object.keys(SHAPES);

const glyph = (name: string): SVGSVGElement =>
  s(
    'svg',
    { class: 'laws-glyph', viewBox: '0 0 24 24', 'aria-hidden': 'true', focusable: 'false' },
    s('path', { d: SHAPES[name] ?? '', 'fill-rule': 'evenodd' }),
  );

const BLOCK = 8;
const SLOW = 3000; // slower answers are left out of the fit: the visitor probably looked away

export const hick: Experiment = (panel, api) => {
  const vis = h('div', { class: 'laws-cue', 'aria-hidden': 'true' });
  const cue = h('p', { class: 'visually-hidden', 'aria-live': 'polite' });
  const opts = NAMES.map(() => h('button', { class: 'laws-opt', type: 'button', tabindex: '-1', hidden: true }));
  const group = h('div', { class: 'laws-opts is-off', role: 'group', 'aria-label': L('hick') }, opts);
  const start = h('button', { class: 'btn btn--ghost btn--sm', type: 'button', onclick: () => void run() }, L('start'));
  const count = h('p', { class: 'laws-count' });
  const svg = plotBox();
  const side = h('div', { class: 'laws-side' }, svg);
  const said = h('p', { class: 'laws-said', 'aria-live': 'polite' });
  const answer = slot<readonly [choice: number, at: number]>();
  let shown = 0; // options that answer right now (0 between trials)
  let token = 0;

  const focusOpt = (i: number) => {
    opts.forEach((b, j) => (b.tabIndex = i === j ? 0 : -1));
    opts[i]?.focus();
  };
  const pick = (i: number, at: number) => {
    if (i < shown) answer.give([i, at]);
  };
  opts.forEach((b, i) => {
    b.addEventListener('pointerdown', (e: PointerEvent) => {
      if (e.isPrimary && e.button === 0) pick(i, e.timeStamp);
    });
    b.addEventListener('click', (e: MouseEvent) => pick(i, e.timeStamp)); // AT activation; dupes are ignored
  });
  panel.addEventListener('keydown', (e: KeyboardEvent) => {
    if (/^[1-8]$/.test(e.key) && answer.open) {
      e.preventDefault();
      return pick(Number(e.key) - 1, e.timeStamp);
    }
    const at = opts.indexOf(document.activeElement as HTMLButtonElement);
    if (at < 0) return;
    const step = ({ ArrowRight: 1, ArrowDown: 1, ArrowLeft: -1, ArrowUp: -1 } as Record<string, number>)[e.key];
    const n = opts.filter((b) => !b.hidden).length;
    if (step) {
      e.preventDefault();
      focusOpt((at + step + n) % n);
    } else if (e.key === 'Enter' || e.key === ' ') {
      e.preventDefault();
      pick(at, e.timeStamp);
    }
  });

  const blank = () => {
    shown = 0;
    group.classList.add('is-off');
    render(vis, h('span', { class: 'laws-fix' }, '+'));
    cue.textContent = '';
  };

  const draw = (data: Pt[], by: Map<number, number[]>) => {
    const f = fit(data);
    const means: Pt[] = [...by].map(([n, xs]) => [hickBits(n), avg(xs) ?? 0]);
    scatter(svg, { pts: data, means, fit: f, xMax: 4, x: L('hick-x'), y: 'ms' });
    return f;
  };

  const idle = (why = '') => {
    token++;
    answer.close();
    blank();
    for (const b of opts) b.hidden = true;
    start.hidden = false;
    count.textContent = '';
    said.textContent = why;
  };

  async function run(): Promise<void> {
    const me = ++token;
    const data: Pt[] = [];
    const by = new Map<number, number[]>();
    const rows: string[][] = [];
    start.hidden = true;
    said.textContent = '';
    side.replaceChildren(svg);
    draw(data, by);
    for (const n of shuffle([2, 4, 8])) {
      opts.forEach((b, i) => (b.hidden = i >= n));
      // Keep keyboard focus in the experiment when its button vanished (Start, or an option past n).
      const a = document.activeElement;
      if (a === start || a === document.body || (a instanceof HTMLButtonElement && opts.includes(a) && a.hidden)) focusOpt(0);
      for (let k = 1; k <= BLOCK; k++) {
        count.textContent = `${P('hick-block', n)} · ${L('count', { n: k, total: BLOCK })}`;
        blank();
        await foreperiod();
        if (me !== token) return;
        const names = shuffle([...NAMES]).slice(0, n);
        const target = Math.floor(Math.random() * n);
        names.forEach((name, i) =>
          render(opts[i]!, glyph(name), h('span', { class: 'laws-key' }, String(i + 1)), h('span', { class: 'visually-hidden' }, L(`shape-${name}`))),
        );
        render(vis, glyph(names[target]!));
        cue.textContent = `${L('shapes-cue')} ${L(`shape-${names[target]}`)}`;
        group.classList.remove('is-off');
        shown = n;
        const got = answer.wait();
        const onset = await frame();
        const [choice, at] = await got;
        if (me !== token) return;
        const rt = at - onset;
        if (choice === target && rt > 0 && rt < SLOW) {
          data.push([hickBits(n), rt]);
          by.set(n, [...(by.get(n) ?? []), rt]);
          rows.push([String(n), fmt(rt)]);
        }
        draw(data, by);
      }
    }
    idle();
    start.textContent = L('again');
    start.focus();
    const f = draw(data, by);
    if (!f) return;
    side.replaceChildren(svg, trials([L('col-n'), L('col-ms')], rows));
    const items: Item[] = [
      [L('b'), fmt(f.b), 'ms/bit'],
      [L('a'), fmt(f.a), 'ms'],
      [L('r2'), fmt(f.r2, 2), ''],
    ];
    api.report({
      items,
      plain: f.b >= 1 ? fill('hick-plain', { b: num(fmt(f.b), 'ms') }) : undefined,
      line: L('hick-line'),
      note: f.b < 1 ? L('hick-flat') : undefined,
    });
  }

  panel.append(
    h('p', null, L('hick-intro')),
    h('div', { class: 'laws-row' }, h('div', { class: 'laws-stage' }, vis, group, count, start), side),
    cue,
    said,
  );
  blank();
  draw([], new Map());
  return (why?: string) => {
    if (start.hidden) idle(why);
  };
};
