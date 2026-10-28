// S4 · Route across Dushanbe. Three algorithms — Dijkstra, A* and Contraction Hierarchies —
// solve the same route on the same road graph, compiled from Rust to WebAssembly, in this
// browser. "Watch" replays what each one explored at the same pace, so the difference in work
// is visible; the numbers are medians of repeated runs, timed here with performance.now().

import init, { Router } from '../../wasm/routing/routing.js';
import { assetUrl } from '../../shared/assets.ts';
import { $, h, num, render } from '../../shared/dom.ts';
import { reducedMotion } from '../../shared/env.ts';
import { duration } from '../../shared/format.ts';
import { lang, t, tn } from '../../shared/i18n.ts';
import { median } from './stats.ts';

const ALGOS = [
  { code: 0, key: 'dijkstra' },
  { code: 1, key: 'astar' },
  { code: 2, key: 'ch' },
] as const;

/** Places from OpenStreetMap (2026-10-28 extract): the railway station and the airport. */
const PRESETS = {
  station: [38.55688, 68.79968],
  airport: [38.54558, 68.8274],
} as const;

interface Panel {
  canvas: HTMLCanvasElement;
  ctx: CanvasRenderingContext2D;
  explored: Float32Array;
  marks: Uint32Array;
  shown: number; // settled nodes drawn so far
  path: Float32Array;
  settled: number;
  time: HTMLElement;
  count: HTMLElement;
  speed: HTMLElement;
  layer: HTMLCanvasElement; // what has been explored so far (drawn incrementally)
}

const css = (name: string) => getComputedStyle(document.documentElement).getPropertyValue(name).trim();

export function mount(root: HTMLElement): void {
  const body = $('.instrument__body', root);
  const status = $('.instrument__status', root);
  if (!body) return;
  if (typeof WebAssembly !== 'object') {
    if (status) status.textContent = t('routing-status-nowasm');
    return;
  }
  if (status) status.textContent = t('routing-status-loading');
  void boot(body, status).catch((err: unknown) => {
    console.error('[mintaka] routing', err);
    if (status) status.textContent = t('routing-status-failed');
  });
}

async function boot(body: HTMLElement, status: HTMLElement | null): Promise<void> {
  const [, bytes] = await Promise.all([
    init({ module_or_path: assetUrl('routing.wasm') }),
    fetch(assetUrl('dushanbe.bin')).then((r) => {
      if (!r.ok) throw new Error(`graph: ${r.status}`);
      return r.arrayBuffer();
    }),
  ]);
  const router = new Router(new Uint8Array(bytes));
  const bounds = router.bounds();
  const [bx0, by0, bx1, by1] = [bounds[0]!, bounds[1]!, bounds[2]!, bounds[3]!];
  const roads = router.roads();
  const aspect = (by1 - by0) / (bx1 - bx0);

  let watch = !reducedMotion();
  let start: number | null = null;
  let end: number | null = null;
  let raf = 0;
  const colours = { road: css('--graphite'), explored: css('--mist'), jump: css('--sky'), path: css('--mintaka'), dot: css('--starlight'), bg: css('--void') };

  // ── DOM ────────────────────────────────────────────────────────────────
  const live = h('p', { class: 'rt-live', 'aria-live': 'polite' });
  const summary = h('p', { class: 'rt-summary' });
  const modeWatch = h('button', { type: 'button', class: 'chip', 'aria-pressed': String(watch), onclick: () => setMode(true) }, t('routing-mode-watch'));
  const modeRace = h('button', { type: 'button', class: 'chip', 'aria-pressed': String(!watch), onclick: () => setMode(false) }, t('routing-mode-race'));
  const presetStation = h('button', { type: 'button', class: 'chip', onclick: () => fromPreset() }, t('routing-preset-airport'));
  const presetRandom = h('button', { type: 'button', class: 'chip', onclick: () => randomPair() }, t('routing-preset-random'));

  const panels: Panel[] = ALGOS.map((a) => {
    const canvas = h('canvas', { class: 'rt-canvas', 'aria-hidden': 'true' });
    const count = h('dd', { class: 'rt-v' }, '—');
    const time = h('dd', { class: 'rt-v' }, '—');
    const speed = h('dd', { class: 'rt-v' }, '—');
    return {
      canvas,
      ctx: canvas.getContext('2d')!,
      explored: new Float32Array(),
      marks: new Uint32Array(),
      shown: 0,
      path: new Float32Array(),
      settled: 0,
      time,
      count,
      speed,
      layer: document.createElement('canvas'),
      // the figure is assembled below
      el: h(
        'figure',
        { class: `rt-panel rt-panel--${a.key}` },
        h('figcaption', { class: 'rt-head' }, h('span', { class: 'rt-name' }, t(`routing-${a.key}`)), h('span', { class: 'rt-what' }, t(`routing-${a.key}-what`))),
        h('div', { class: 'rt-map' }, canvas, h('span', { class: 'rt-osm' }, '© OpenStreetMap')),
        h('dl', { class: 'rt-stats' },
          h('div', null, h('dt', null, t('routing-explored')), count),
          h('div', null, h('dt', null, t('routing-time')), time),
          h('div', null, h('dt', null, t('routing-speedup')), speed),
        ),
      ),
    } as Panel & { el: HTMLElement };
  });

  render(
    body,
    h('div', { class: 'rt-controls' },
      h('div', { class: 'rt-group', role: 'group', 'aria-label': t('routing-route') }, presetStation, presetRandom),
      h('div', { class: 'rt-group', role: 'group', 'aria-label': t('routing-mode') }, modeWatch, modeRace),
    ),
    h('p', { class: 'rt-hint' }, t('routing-hint')),
    h('div', { class: 'rt-panels' }, ...panels.map((p) => (p as Panel & { el: HTMLElement }).el)),
    summary,
    live,
    h('p', { class: 'rt-graph source' }, ...tn('routing-graph', {
      n: num({ value: router.nodes().toLocaleString(lang()), unit: '' }),
      arcs: num({ value: router.arcs().toLocaleString(lang()), unit: '' }),
      shortcuts: num({ value: router.shortcuts().toLocaleString(lang()), unit: '' }),
    })),
  );
  if (status) {
    status.textContent = t('routing-status-ready');
    status.className = 'instrument__status badge badge--info';
  }

  // ── drawing ────────────────────────────────────────────────────────────
  let scale = 1;
  let dpr = 1;
  const toPx = (x: number, y: number): [number, number] => [(x - bx0) * scale, (y - by0) * scale];

  function size(): void {
    const w = panels[0]!.canvas.parentElement!.clientWidth || 300;
    dpr = Math.min(devicePixelRatio || 1, 2);
    scale = w / (bx1 - bx0);
    for (const p of panels) {
      p.canvas.width = Math.round(w * dpr);
      p.canvas.height = Math.round(w * aspect * dpr);
      p.canvas.style.setProperty('aspect-ratio', `${(bx1 - bx0) / (by1 - by0)}`);
      p.layer.width = p.canvas.width;
      p.layer.height = p.canvas.height;
      p.shown = 0;
      p.layer.getContext('2d')!.clearRect(0, 0, p.layer.width, p.layer.height);
    }
    drawBase();
    for (const p of panels) reveal(p, p.marks.length);
    paint();
  }

  const base = document.createElement('canvas');
  function drawBase(): void {
    base.width = panels[0]!.canvas.width;
    base.height = panels[0]!.canvas.height;
    const c = base.getContext('2d')!;
    c.setTransform(dpr, 0, 0, dpr, 0, 0);
    c.strokeStyle = colours.road;
    c.lineWidth = 0.7;
    c.beginPath();
    for (let i = 0; i < roads.length; i += 4) {
      const [x1, y1] = toPx(roads[i]!, roads[i + 1]!);
      const [x2, y2] = toPx(roads[i + 2]!, roads[i + 3]!);
      c.moveTo(x1, y1);
      c.lineTo(x2, y2);
    }
    c.stroke();
  }

  /** Draw explored lines for settled nodes up to `upto` onto the panel's layer. */
  function reveal(p: Panel, upto: number): void {
    if (upto <= p.shown) return;
    const c = p.layer.getContext('2d')!;
    c.setTransform(dpr, 0, 0, dpr, 0, 0);
    const jump = p === panels[2];
    c.strokeStyle = jump ? colours.jump : colours.explored;
    c.globalAlpha = jump ? 0.85 : 0.5;
    c.lineWidth = jump ? 1 : 1.2;
    c.beginPath();
    const from = p.shown === 0 ? 0 : p.marks[p.shown - 1]!;
    const to = p.marks[upto - 1] ?? from;
    for (let i = from; i < to; i += 4) {
      const [x1, y1] = toPx(p.explored[i]!, p.explored[i + 1]!);
      const [x2, y2] = toPx(p.explored[i + 2]!, p.explored[i + 3]!);
      c.moveTo(x1, y1);
      c.lineTo(x2, y2);
    }
    c.stroke();
    p.shown = upto;
  }

  function paint(): void {
    for (const p of panels) {
      const c = p.ctx;
      c.setTransform(1, 0, 0, 1, 0, 0);
      c.clearRect(0, 0, p.canvas.width, p.canvas.height);
      c.drawImage(base, 0, 0);
      c.drawImage(p.layer, 0, 0);
      c.setTransform(dpr, 0, 0, dpr, 0, 0);
      const done = p.shown >= p.marks.length;
      if (done && p.path.length) {
        c.strokeStyle = colours.path;
        c.lineWidth = 2.6;
        c.lineCap = 'round';
        c.beginPath();
        for (let i = 0; i < p.path.length; i += 4) {
          const [x1, y1] = toPx(p.path[i]!, p.path[i + 1]!);
          const [x2, y2] = toPx(p.path[i + 2]!, p.path[i + 3]!);
          c.moveTo(x1, y1);
          c.lineTo(x2, y2);
        }
        c.stroke();
      }
      // Start: a filled dot. Destination: a ring. (Rust is kept for Betelgeuse and the CTA.)
      for (const [node, ring] of [[start, false], [end, true]] as const) {
        if (node === null) continue;
        const xy = router.node_xy(node);
        const [x, y] = toPx(xy[0]!, xy[1]!);
        c.beginPath();
        c.arc(x, y, ring ? 5 : 4, 0, Math.PI * 2);
        c.fillStyle = ring ? colours.bg : colours.dot;
        c.fill();
        if (ring) {
          c.strokeStyle = colours.dot;
          c.lineWidth = 2;
          c.stroke();
        }
      }
    }
  }

  // ── running ────────────────────────────────────────────────────────────
  function run(): void {
    if (start === null || end === null) return;
    cancelAnimationFrame(raf);
    for (const [i, a] of ALGOS.entries()) {
      const p = panels[i]!;
      p.settled = router.run(a.code, start, end, true);
      p.explored = router.explored();
      p.marks = router.explored_marks();
      p.path = router.path();
      p.shown = 0;
      p.layer.getContext('2d')!.clearRect(0, 0, p.layer.width, p.layer.height);
    }
    const metres = router.distance() / 10;
    measure();
    if (!watch) {
      for (const p of panels) reveal(p, p.marks.length);
      paint();
      announce(metres);
      return;
    }
    // Same pace for everyone: the slowest finishes in ~2.5 s, the others when their work is done.
    const most = Math.max(...panels.map((p) => p.marks.length));
    const perMs = most / 2500;
    const t0 = performance.now();
    const frame = (now: number) => {
      const n = Math.ceil((now - t0) * perMs);
      for (const p of panels) reveal(p, Math.min(p.marks.length, n));
      paint();
      if (panels.some((p) => p.shown < p.marks.length)) raf = requestAnimationFrame(frame);
      else announce(metres);
    };
    raf = requestAnimationFrame(frame);
  }

  /** Median time per query in this browser: CH queries are batched to beat timer resolution. */
  function measure(): void {
    const ms: number[] = [];
    for (const [i, a] of ALGOS.entries()) {
      const batch = a.code === 2 ? 50 : a.code === 1 ? 5 : 1;
      const samples: number[] = [];
      for (let k = 0; k < 9; k++) {
        const t0 = performance.now();
        for (let j = 0; j < batch; j++) router.run(a.code, start!, end!, false);
        samples.push((performance.now() - t0) / batch);
      }
      ms[i] = median(samples);
      const p = panels[i]!;
      render(p.count, num({ value: p.settled.toLocaleString(lang()), unit: '' }));
      render(p.time, num(duration(ms[i]!)));
    }
    for (const [i, p] of panels.entries()) {
      const x = ms[0]! / ms[i]!;
      render(p.speed, i === 0 ? t('routing-baseline') : num({ value: x >= 10 ? Math.round(x).toLocaleString(lang()) : x.toFixed(1), unit: '×' }));
    }
    const less = panels[0]!.settled / Math.max(1, panels[2]!.settled);
    render(summary, ...tn('routing-summary', { less: num({ value: Math.round(less).toLocaleString(lang()), unit: '×' }) }));
  }

  function announce(metres: number): void {
    live.textContent = t('routing-announce', {
      km: (metres / 1000).toLocaleString(lang(), { maximumFractionDigits: 1 }),
      d: panels[0]!.settled,
      c: panels[2]!.settled,
    });
  }

  function fromPreset(): void {
    const a = router.project(PRESETS.station[0], PRESETS.station[1]);
    const b = router.project(PRESETS.airport[0], PRESETS.airport[1]);
    start = router.nearest(a[0]!, a[1]!);
    end = router.nearest(b[0]!, b[1]!);
    run();
  }

  let seed = 0x9e3779b9;
  function randomPair(): void {
    const n = router.nodes();
    const next = () => ((seed = (seed * 1664525 + 1013904223) >>> 0), seed % n);
    start = next();
    end = next();
    run();
  }

  function setMode(w: boolean): void {
    watch = w;
    modeWatch.setAttribute('aria-pressed', String(w));
    modeRace.setAttribute('aria-pressed', String(!w));
    run();
  }

  // Tap a map: first tap sets the start, the second the end.
  for (const p of panels) {
    p.canvas.addEventListener('click', (e) => {
      const r = p.canvas.getBoundingClientRect();
      const x = bx0 + ((e.clientX - r.left) / r.width) * (bx1 - bx0);
      const y = by0 + ((e.clientY - r.top) / r.height) * (by1 - by0);
      const node = router.nearest(x, y);
      if (start === null || end !== null) {
        start = node;
        end = null;
        for (const q of panels) {
          q.marks = new Uint32Array();
          q.path = new Float32Array();
          q.shown = 0;
          q.layer.getContext('2d')!.clearRect(0, 0, q.layer.width, q.layer.height);
        }
        paint();
      } else {
        end = node;
        run();
      }
    });
  }

  new ResizeObserver(() => size()).observe(panels[0]!.canvas.parentElement!);
  size();
  fromPreset();
}
