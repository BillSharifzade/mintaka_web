// S6 · Overload lab: the naive service and the one built our way, under the same load, simulated
// by a discrete-event model in Rust compiled to WebAssembly (crates/loadsim) and drawn here.
// The only request is the .wasm itself; the seed is Math.random(), the clock performance.now(),
// and simulated time runs at real speed — paused off screen, in a hidden tab, or on request.

import init, { Lab, params } from '../../wasm/loadsim/loadsim.js';
import { assetUrl } from '../../shared/assets.ts';
import { $, h, num, render, s, type Child } from '../../shared/dom.ts';
import { offline, reducedMotion } from '../../shared/env.ts';
import { duration } from '../../shared/format.ts';
import { lang, pluralKey, t, tn } from '../../shared/i18n.ts';
import { area, line, linear, loadRate, log, nice, Series } from './chart.ts';

// Fields per system per window — crates/loadsim/src/sim.rs, FIELDS.
const F = { offered: 1, good: 3, shed: 4, timeouts: 5, wasted: 6, p99: 8, queue: 9, busy: 10, workers: 11, starting: 12, l: 13, lambda: 14, w: 15 };
// params() — crates/loadsim/src/wasm.rs.
const P = { stride: 0, fields: 1, base: 3, mean: 4, timeout: 6, retries: 7, cap: 9, naive: 10, min: 11, max: 12, boot: 13, target: 16, interval: 17, per: 18 };
const FIELDS = 16;
const W = 240; // chart boxes, scaled by CSS
const H = 48;
const LAT = log(10, 10_000); // latency charts: 10 ms … 10 s

type Kind = 'p99' | 'offered' | 'good' | 'shed' | 'timeouts';
interface Panel {
  series: Record<Kind, Series>;
  paths: Record<Kind, SVGPathElement>;
  p99: HTMLElement;
  served: HTMLElement;
  refused: HTMLElement;
  state: HTMLElement;
  little: HTMLElement;
}

const nf = (digits: number) => new Intl.NumberFormat(lang(), { maximumFractionDigits: digits, minimumFractionDigits: digits });
const n0 = (v: number) => ({ value: nf(0).format(v), unit: '' });
const n1 = (v: number) => ({ value: nf(v < 10 ? 1 : 0).format(v), unit: '' });
/** A model parameter in words: 2000 → "2 s", 50 → "50 ms". */
const span = (ms: number) => (ms >= 1000 ? `${new Intl.NumberFormat(lang()).format(ms / 1000)} s` : `${ms} ms`);

export function mount(root: HTMLElement): void {
  const body = $('.instrument__body', root);
  const badge = $('.instrument__status', root);
  const say = (key: string, kind: string) => {
    if (!badge) return;
    badge.textContent = t(key);
    badge.className = `instrument__status badge badge--${kind}`;
  };
  if (!body) return;
  // Without WebAssembly the static explanation stays, with a line saying why nothing runs.
  const fail = (key: string) => {
    body.append(h('p', { class: 'instrument__fallback' }, t(key)));
    say('loadsim-status', 'plain');
  };
  if (typeof WebAssembly !== 'object') return fail('loadsim-nowasm');
  say('loadsim-loading', 'plain');
  init({ module_or_path: assetUrl('loadsim.wasm') })
    .then(() => start(root, body, say))
    .catch((err: unknown) => {
      // Offline before the WebAssembly was ever stored: say so, load once reconnected.
      if (offline()) return void import('../../offline/index.ts').then((o) => o.unavailable(root, () => mount(root), 'wasm'));
      console.error('[mintaka] loadsim', err);
      fail('loadsim-failed');
    });
}

function start(root: HTMLElement, body: HTMLElement, say: (key: string, kind: string) => void): void {
  const p = params();
  if (p[P.fields] !== FIELDS) throw new Error('loadsim: the page and the WebAssembly disagree on the layout');
  const at = (i: number) => p[i]!;
  const stride = at(P.stride);
  const base = at(P.base);
  const timeout = at(P.timeout);
  const cap = at(P.cap);
  const seed = () => Math.floor(Math.random() * 2 ** 32);
  let lab = new Lab(seed());
  let x = 1;

  // ── panels ─────────────────────────────────────────────────────────────
  const key = (k: string) => h('span', { class: `ls-key ls-key--${k}`, 'aria-hidden': 'true' });
  const chart = (...kids: SVGElement[]) =>
    s('svg', { class: 'ls-chart', viewBox: `0 0 ${W} ${H}`, preserveAspectRatio: 'none', 'aria-hidden': 'true', focusable: 'false' }, kids);
  const gy = (ms: number) => +(H - 1 - LAT(ms) * (H - 2)).toFixed(1);
  const metric = (label: Child, value: HTMLElement, svg?: SVGElement) =>
    h('div', { class: 'ls-metric' }, h('p', { class: 'ls-metric__head' }, h('span', null, label), ' ', value), svg);

  const panel = (id: string, about: (Node | string)[]): [HTMLElement, Panel] => {
    const path = (k: Kind) => s('path', { class: `ls-${k}` });
    const paths = { p99: path('p99'), offered: path('offered'), good: path('good'), shed: path('shed'), timeouts: path('timeouts') };
    const out = () => h('span', { class: 'ls-value' });
    const pn: Panel = {
      series: { p99: new Series(), offered: new Series(), good: new Series(), shed: new Series(), timeouts: new Series() },
      paths,
      p99: out(),
      served: out(),
      refused: out(),
      state: h('p', { class: 'ls-state' }),
      little: h('span'),
    };
    const el = h(
      'section',
      { class: 'ls-sys', 'aria-labelledby': `ls-${id}-title` },
      h('h4', { class: 'ls-sys__title', id: `ls-${id}-title` }, t(`loadsim-${id}`)),
      h('p', { class: 'ls-sys__about' }, about),
      metric(
        t('loadsim-p99'),
        pn.p99,
        chart(
          s('path', { class: 'ls-grid', d: [100, 1000].map((v) => `M0 ${gy(v)}H${W}`).join('') }),
          s('path', { class: 'ls-timeout', d: `M0 ${gy(timeout)}H${W}` }),
          paths.p99,
        ),
      ),
      metric([key('good'), key('offered'), t('loadsim-served')], pn.served, chart(paths.offered, paths.good)),
      metric([key('shed'), key('timeouts'), t('loadsim-refused')], pn.refused, chart(paths.shed, paths.timeouts)),
      h('div', { class: 'ls-metric' }, h('p', { class: 'ls-metric__head' }, t('loadsim-state')), pn.state),
      h('p', { class: 'ls-little' }, h('span', { class: 'ls-little__law' }, 'L = λW'), ' ', pn.little),
    );
    return [el, pn];
  };

  const [naiveEl, naive] = panel(
    'naive',
    tn(pluralKey('loadsim-naive-about', at(P.naive)), { n: at(P.naive) }),
  );
  const [oursEl, ours] = panel(
    'ours',
    tn(pluralKey('loadsim-ours-about', at(P.min)), {
      n: at(P.min),
      max: at(P.max),
      boot: span(at(P.boot)),
      target: span(at(P.target)),
      interval: span(at(P.interval)),
      per: at(P.per),
    }),
  );
  const panels = [naive, ours];

  // ── numbers ────────────────────────────────────────────────────────────
  const values = (pn: Panel, r: (f: number) => number) => {
    const p99 = r(F.p99);
    render(pn.p99, Number.isNaN(p99) ? t('loadsim-none') : num(duration(p99)));
    render(pn.served, ...tn('loadsim-served-value', { good: num(n0(r(F.good))), offered: num(n0(r(F.offered))) }));
    render(pn.refused, ...tn('loadsim-refused-value', { shed: num(n0(r(F.shed))), timeouts: num(n0(r(F.timeouts))) }));
    const [queue, workers, starting, wasted] = [r(F.queue), r(F.workers), r(F.starting), r(F.wasted)];
    render(
      pn.state,
      ...tn(pluralKey('loadsim-waiting', queue), { n: num(n0(queue)) }),
      queue >= cap && ` ${t('loadsim-limit')}`,
      ' · ',
      ...tn(pluralKey('loadsim-busy', workers), { busy: num(n1(r(F.busy))), n: num(n0(workers)) }),
      starting > 0 && [' · ', ...tn(pluralKey('loadsim-starting', starting), { n: num(n0(starting)) })],
      wasted > 0 && [' · ', ...tn('loadsim-wasted', { n: num(n0(wasted)) })],
    );
    const [l, lambda, w] = [r(F.l), r(F.lambda), r(F.w)];
    render(
      pn.little,
      ...tn('loadsim-little', {
        l: num(n1(l)),
        lambda: num(n0(lambda)),
        w: num(Number.isNaN(w) ? { value: '—', unit: '' } : duration(w)),
        lw: num(n1(Number.isNaN(w) ? 0 : (lambda * w) / 1000)),
      }),
    );
  };

  // ── charts: the same scales in both panels, so they compare directly ────
  const draw = () => {
    const rate = linear(nice(Math.max(...panels.map((pn) => pn.series.offered.max())), 20));
    const bad = linear(nice(Math.max(...panels.flatMap((pn) => [pn.series.shed.max(), pn.series.timeouts.max()])), 20));
    for (const pn of panels) {
      const d = (k: Kind, path: string) => pn.paths[k].setAttribute('d', path);
      d('p99', line(pn.series.p99.v, W, H, LAT));
      d('offered', area(pn.series.offered.v, W, H, rate));
      d('good', line(pn.series.good.v, W, H, rate));
      d('shed', line(pn.series.shed.v, W, H, bad));
      d('timeouts', line(pn.series.timeouts.v, W, H, bad));
    }
  };

  const advance = (ms: number) => {
    const out = lab.step(ms);
    const rows = out.length / stride;
    if (!rows) return;
    for (let i = 0; i < rows; i++) {
      panels.forEach((pn, j) => {
        const r = (f: number) => out[i * stride + 1 + j * FIELDS + f]!;
        pn.series.p99.push(r(F.p99));
        pn.series.offered.push(r(F.offered));
        pn.series.good.push(r(F.good));
        pn.series.shed.push(r(F.shed));
        pn.series.timeouts.push(r(F.timeouts));
        if (i === rows - 1) values(pn, r);
      });
    }
    draw();
  };

  // ── controls ───────────────────────────────────────────────────────────
  const range = h('input', { class: 'ls-range', type: 'range', id: 'ls-load', min: 1, max: 100, step: 1, value: 1 });
  const shown = h('output', { class: 'ls-x', for: 'ls-load' });
  const setLoad = () => {
    x = Number(range.value);
    const text = t('loadsim-load-value', { x, rate: loadRate(x, base) });
    shown.textContent = text;
    range.setAttribute('aria-valuetext', text);
    lab.setRate(x * base);
  };
  range.addEventListener('input', setLoad);

  let playing = true;
  let onScreen = false;
  let raf = 0;
  let timer = 0;
  const pause = h('button', { class: 'btn btn--ghost btn--sm', type: 'button' }, t('loadsim-pause'));
  pause.addEventListener('click', () => {
    playing = !playing;
    pause.textContent = t(playing ? 'loadsim-pause' : 'loadsim-play');
    run();
  });
  const reset = h('button', { class: 'btn btn--quiet btn--sm', type: 'button' }, t('loadsim-reset'));
  reset.addEventListener('click', () => {
    lab.free();
    lab = new Lab(seed());
    lab.setRate(x * base);
    for (const pn of panels) for (const k of Object.keys(pn.series) as Kind[]) pn.series[k].clear();
    draw();
  });

  // ── the clock ──────────────────────────────────────────────────────────
  const stop = () => {
    cancelAnimationFrame(raf);
    clearInterval(timer);
    raf = timer = 0;
  };
  function run(): void {
    stop();
    if (!playing || !onScreen || document.visibilityState !== 'visible') return;
    if (reducedMotion()) {
      timer = window.setInterval(() => advance(1000), 1000); // one discrete update a second
      return;
    }
    let last = performance.now();
    const frame = (now: number) => {
      advance(Math.min(250, now - last)); // a long frame doesn't make the simulation jump
      last = now;
      raf = requestAnimationFrame(frame);
    };
    raf = requestAnimationFrame(frame);
  }

  render(
    body,
    h(
      'div',
      { class: 'ls' },
      h('div', { class: 'ls-controls' }, h('label', { class: 'ls-label', for: 'ls-load' }, t('loadsim-load')), range, shown, pause, reset),
      h(
        'p',
        { class: 'ls-about' },
        tn(pluralKey('loadsim-about', at(P.retries)), { base, mean: span(at(P.mean)), timeout: span(timeout), n: at(P.retries) }),
      ),
      h('div', { class: 'ls-panels' }, naiveEl, oursEl),
      h('p', { class: 'ls-note' }, t('loadsim-little-about')),
      h('p', { class: 'ls-note' }, t('loadsim-legend', { timeout: span(timeout), cap }), ' ', t('loadsim-try')),
    ),
  );
  setLoad();
  advance(0);
  say('loadsim-status', 'info');

  new IntersectionObserver((entries) => {
    onScreen = entries.some((e) => e.isIntersecting);
    run();
  }).observe(root);
  document.addEventListener('visibilitychange', run);
}
