// S6 · Live load: this server's own traffic, one tick per second over Server-Sent Events from
// GET /_telemetry (docs/CONTRACTS.md §6) — aggregates only. The stream is open only while the
// instrument is on screen and the tab is visible; it reconnects with backoff, and the status
// badge says honestly which of connecting / live / reconnecting / offline / paused it is.
// `Ring` and `sparkPath` are pure and tested in live.test.ts.

import { $, h, render, s } from '../shared/dom.ts';
import { reducedMotion } from '../shared/env.ts';
import { bytes, duration, type Formatted } from '../shared/format.ts';
import { lang, t, tp } from '../shared/i18n.ts';
import type { TelemetryHello, TelemetryTick } from '../shared/proof.ts';

/** The last `cap` values of one metric, oldest first; null marks a second without data. */
export class Ring {
  readonly cap: number;
  #buf: (number | null)[];
  #next = 0;
  #size = 0;

  constructor(cap: number) {
    this.cap = cap;
    this.#buf = new Array<number | null>(cap).fill(null);
  }

  push(value: number | null): void {
    this.#buf[this.#next] = value;
    this.#next = (this.#next + 1) % this.cap;
    this.#size = Math.min(this.#size + 1, this.cap);
  }

  get size(): number {
    return this.#size;
  }

  values(): (number | null)[] {
    const out: (number | null)[] = [];
    for (let i = this.#size; i > 0; i--) out.push(this.#buf[(this.#next - i + this.cap) % this.cap] ?? null);
    return out;
  }
}

/** The largest value, or 0. */
export const peak = (values: readonly (number | null)[]): number =>
  values.reduce<number>((m, v) => (v !== null && v > m ? v : m), 0);

/**
 * Path data for a sparkline in a `w`×`h` box: one step per second with the newest value at the
 * right edge, y from 0 (bottom) to `top` (top), 1 unit of margin; a null breaks the line.
 */
export function sparkPath(values: readonly (number | null)[], cap: number, w: number, h: number, top = peak(values)): string {
  const dx = w / (cap - 1);
  const skip = cap - values.length;
  let d = '';
  let pen = false;
  values.forEach((v, i) => {
    if (v === null) {
      pen = false;
      return;
    }
    const y = top > 0 ? h - 1 - (Math.min(Math.max(v, 0), top) / top) * (h - 2) : h - 1;
    d += `${pen ? 'L' : 'M'}${+((skip + i) * dx).toFixed(1)} ${+y.toFixed(1)}`;
    pen = true;
  });
  return d;
}

/** Requests per dot so the stream stays readable: 1, 2, 5, 10, 20, 50… (≤ `most` dots/s). */
export function perDot(rps: number, most = 40): number {
  if (rps <= most) return 1;
  const want = rps / most;
  const e = 10 ** Math.floor(Math.log10(want));
  return [1, 2, 5, 10].map((m) => m * e).find((step) => step >= want) ?? 10 * e;
}

const CAP = 60;
const SPARK_W = 120;
const SPARK_H = 24;
const TRAVEL_S = 2.4;

type State = 'connecting' | 'live' | 'reconnecting' | 'offline' | 'paused';
const BADGE: Record<State, string> = { connecting: 'plain', live: 'live', reconnecting: 'warn', offline: 'bad', paused: 'plain' };

const count = (v: number): Formatted => ({ value: new Intl.NumberFormat(lang()).format(Math.round(v)), unit: '' });
const micro = (v: number): Formatted => duration(v / 1000);
const pct = (v: number): Formatted => ({ value: new Intl.NumberFormat(lang(), { maximumFractionDigits: 1 }).format(v), unit: '%' });
const said = (f: Formatted): string => (f.unit === '%' ? f.value + f.unit : f.unit ? `${f.value} ${f.unit}` : f.value);

interface Metric {
  id: string;
  get(k: TelemetryTick): number | null;
  format(v: number): Formatted;
  spark: boolean;
}

const METRICS: Metric[] = [
  { id: 'rps', get: (k) => k.rps, format: count, spark: true },
  { id: 'p50', get: (k) => k.p50_us, format: micro, spark: true },
  { id: 'p99', get: (k) => k.p99_us, format: micro, spark: true },
  { id: 'p999', get: (k) => k.p999_us, format: micro, spark: true },
  { id: 'cpu', get: (k) => k.cpu_pct, format: pct, spark: true },
  { id: 'rss', get: (k) => k.rss_bytes, format: bytes, spark: true },
  { id: 'conns', get: (k) => k.conns, format: count, spark: true },
  { id: 'total', get: (k) => k.req_total, format: count, spark: false },
];

interface Row {
  m: Metric;
  ring: Ring;
  value: HTMLElement;
  path: SVGPathElement | null;
  range: HTMLElement | null;
}

/** Dots flowing left → right along a thin line, at the rate requests arrive. */
function stream(canvas: HTMLCanvasElement) {
  const ctx = canvas.getContext('2d');
  const css = getComputedStyle(document.documentElement);
  const ink = css.getPropertyValue('--starlight').trim();
  const rule = css.getPropertyValue('--graphite').trim();
  const dpr = Math.min(2, devicePixelRatio || 1);
  let dots: number[] = [];
  let rate = 0;
  let wait = 0;
  let raf = 0;
  let last = 0;
  let on = false;

  const paint = () => {
    if (!ctx) return;
    const { width: w, height: hh } = canvas;
    const lw = Math.max(1, Math.round(dpr));
    const y = Math.floor(hh / 2);
    const r = 1.75 * dpr;
    ctx.clearRect(0, 0, w, hh);
    ctx.fillStyle = rule;
    ctx.fillRect(0, y, w, lw);
    ctx.fillStyle = ink;
    ctx.beginPath();
    for (const x of dots) {
      ctx.moveTo(x * w + r, y + lw / 2);
      ctx.arc(x * w, y + lw / 2, r, 0, 2 * Math.PI);
    }
    ctx.fill();
  };
  // Size the backing store from a ResizeObserver, so the animation loop never reads layout.
  if ('ResizeObserver' in window) {
    new ResizeObserver(([e]) => {
      if (!e) return;
      canvas.width = Math.round(e.contentRect.width * dpr);
      canvas.height = Math.round(e.contentRect.height * dpr);
      paint();
    }).observe(canvas);
  }
  const gap = () => -Math.log(1 - Math.random()) / rate; // Poisson arrivals, in seconds
  const frame = (now: number) => {
    const dt = Math.min(0.1, (now - last) / 1000);
    last = now;
    dots = dots.map((x) => x + dt / TRAVEL_S).filter((x) => x <= 1);
    if (rate > 0) {
      for (wait -= dt; wait <= 0; wait += gap()) dots.push((-wait / TRAVEL_S) % 1);
    }
    paint();
    // No traffic and an empty line: sleep until the next tick brings requests.
    raf = rate > 0 || dots.length ? requestAnimationFrame(frame) : 0;
  };
  const wake = () => {
    if (!on || raf || reducedMotion()) return;
    last = performance.now();
    raf = requestAnimationFrame(frame);
  };

  return {
    rate(dotsPerS: number): void {
      rate = dotsPerS;
      if (rate > 0) wait = gap(); // arrivals are memoryless: a fresh gap at the new rate is exact
      if (reducedMotion()) {
        // A still snapshot: as many dots as would be on the line at this rate, evenly spaced.
        const n = Math.round(rate * TRAVEL_S);
        dots = Array.from({ length: n }, (_, i) => (i + 0.5) / n);
        paint();
      } else wake();
    },
    start(): void {
      on = true;
      wake();
    },
    stop(): void {
      on = false;
      cancelAnimationFrame(raf);
      raf = 0;
    },
  };
}

export function mount(root: HTMLElement): void {
  const body = $('.instrument__body', root) ?? root.appendChild(h('div', { class: 'instrument__body' }));
  const badge = $('.instrument__status', root);
  const foot = $('.instrument__foot', root) ?? root.appendChild(h('p', { class: 'instrument__foot' }));
  foot.textContent = t('live-foot');

  const setBadge = (label: string, kind: string) => {
    if (!badge) return;
    badge.textContent = label;
    badge.className = `instrument__status badge badge--${kind}`;
  };

  if (!('EventSource' in window)) {
    render(body, h('p', { class: 'instrument__fallback' }, t('live-unsupported')));
    setBadge(t('live-offline'), 'plain');
    return;
  }

  const rows: Row[] = METRICS.map((m) => ({
    m,
    ring: new Ring(CAP),
    value: h('span', { class: 'num' }, '—'),
    path: m.spark ? s('path') : null,
    range: m.spark ? h('span', { class: 'visually-hidden' }) : null,
  }));
  const canvas = h('canvas', { class: 'live-canvas', 'aria-hidden': 'true' });
  const scale = h('p', { class: 'source' });
  const server = h('p', { class: 'source' });
  const announce = h('p', { class: 'visually-hidden', 'aria-live': 'polite' });
  const flow = stream(canvas);

  render(
    body,
    h(
      'div',
      { class: 'live' },
      h(
        'dl',
        { class: 'kv live-kv' },
        rows.map((r) =>
          h(
            'div',
            null,
            h('dt', null, t(`live-${r.m.id}`)),
            h(
              'dd',
              null,
              r.path &&
                s(
                  'svg',
                  { class: 'live-spark', viewBox: `0 0 ${SPARK_W} ${SPARK_H}`, preserveAspectRatio: 'none', 'aria-hidden': 'true' },
                  s('line', { class: 'live-base', x1: 0, y1: SPARK_H - 0.5, x2: SPARK_W, y2: SPARK_H - 0.5 }),
                  r.path,
                ),
              r.value,
              r.range && [' ', r.range],
            ),
          ),
        ),
      ),
      h('div', { class: 'live-stream' }, canvas, scale),
      h('p', { class: 'source' }, t('live-window')),
      server,
      announce,
    ),
  );

  let es: EventSource | null = null;
  let state: State | null = null;
  let wasLive = false;
  let lost = false;
  let tries = 0;
  let retry = 0;
  let watchdog = 0;
  let lastT = 0;
  let inView = false;

  // Announce only losses and recoveries — not every scroll that pauses the stream.
  const setState = (next: State) => {
    if (next === state) return;
    let news = '';
    if (next === 'live') {
      if (lost) news = 'live-said-live';
      lost = false;
      wasLive = true;
    } else if (wasLive && !lost && (next === 'reconnecting' || next === 'offline')) {
      news = next === 'offline' ? 'live-said-offline' : 'live-said-lost';
      lost = true;
    }
    if (news) announce.textContent = t(news);
    state = next;
    setBadge(t(`live-${next}`), BADGE[next]);
  };

  const onTick = (k: TelemetryTick) => {
    // Seconds we missed (paused, reconnecting) stay visible as gaps in the lines.
    if (lastT && k.t > lastT + 1) {
      for (let i = Math.min(k.t - lastT - 1, CAP); i > 0; i--) for (const r of rows) r.ring.push(null);
    }
    lastT = Math.max(lastT, k.t);
    for (const r of rows) {
      const v = r.m.get(k);
      r.ring.push(v);
      if (v === null) {
        r.value.className = 'live-idle';
        r.value.textContent = t('live-idle');
      } else {
        const f = r.m.format(v);
        r.value.className = 'num';
        render(r.value, f.value, f.unit && h('span', { class: 'num__unit' }, f.unit));
      }
      if (r.path && r.range) {
        const vals = r.ring.values();
        r.path.setAttribute('d', sparkPath(vals, CAP, SPARK_W, SPARK_H));
        const seen = vals.filter((x): x is number => x !== null);
        r.range.textContent = seen.length
          ? t('live-range', { min: said(r.m.format(Math.min(...seen))), max: said(r.m.format(peak(seen))) })
          : '';
      }
    }
    const per = perDot(k.rps);
    scale.textContent = tp('live-dots', per);
    flow.rate(k.rps / per);
  };

  const close = () => {
    es?.close();
    es = null;
    clearTimeout(watchdog);
  };

  // No tick for 5 s means the stream is stuck, even if the socket looks open.
  const arm = () => {
    clearTimeout(watchdog);
    watchdog = window.setTimeout(fail, 5000);
  };

  function fail(): void {
    close();
    tries++;
    if (!active()) return;
    setState(tries > 3 || !navigator.onLine ? 'offline' : 'reconnecting');
    const delay = Math.min(30, 2 ** (tries - 1)) * (0.8 + Math.random() * 0.4) * 1000;
    retry = window.setTimeout(open, delay);
  }

  function open(): void {
    clearTimeout(retry);
    if (es || !active()) return;
    if (!navigator.onLine) return setState('offline'); // the 'online' event brings us back
    setState(tries ? 'reconnecting' : 'connecting');
    const source = new EventSource('/_telemetry');
    es = source;
    source.addEventListener('hello', (e) => {
      try {
        const hello = JSON.parse((e as MessageEvent<string>).data) as TelemetryHello;
        server.textContent = tp('live-server', hello.cores, { region: hello.region, build: hello.build });
      } catch {
        /* a malformed hello only costs us the server line */
      }
    });
    source.addEventListener('tick', (e) => {
      let k: TelemetryTick;
      try {
        k = JSON.parse((e as MessageEvent<string>).data) as TelemetryTick;
      } catch {
        return;
      }
      tries = 0;
      setState('live');
      arm();
      onTick(k);
    });
    source.onerror = fail;
    arm();
  }

  const active = () => inView && document.visibilityState === 'visible';

  const update = () => {
    if (active()) {
      flow.start();
      open();
    } else {
      flow.stop();
      close();
      clearTimeout(retry);
      setState('paused');
    }
  };

  setState('connecting');
  new IntersectionObserver((entries) => {
    inView = entries.some((e) => e.isIntersecting);
    update();
  }).observe(root);
  document.addEventListener('visibilitychange', update);
  window.addEventListener('online', () => {
    tries = 0;
    open();
  });
}
