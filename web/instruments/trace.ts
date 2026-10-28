// S6 · Follow one request. The visitor's own page request, hop by hop, from its Navigation Timing
// entry and our Server-Timing header (docs/CONTRACTS.md §4.5). "Send another request" times a
// fresh GET /_ping the same way, from its Resource Timing entry. The SVG timeline is decoration
// (aria-hidden); the ordered list below it carries the same facts for everyone.
// `hops()` is pure and tested in trace.test.ts.

import { $, h, render, s } from '../shared/dom.ts';
import { reducedMotion } from '../shared/env.ts';
import { bytes, duration, type Formatted } from '../shared/format.ts';
import { t, tn } from '../shared/i18n.ts';
import { navEntry, serverTiming } from '../shared/timing.ts';

export type HopId = 'dns' | 'tcp' | 'tls' | 'net' | 'app' | 'ttfb' | 'down';
/** Why a hop took no time — shown, never hidden. */
export type Zero = 'reused' | 'cached' | 'plain' | 'quic';

export interface Hop {
  id: HopId;
  /** Duration, ms. */
  ms: number;
  /** Extent on the timeline, ms after fetchStart. */
  from: number;
  to: number;
  zero?: Zero | undefined;
}

export interface Trace {
  hops: Hop[];
  /** fetchStart → responseEnd, ms. */
  total: number;
  /** Time not inside any hop (cache lookup, queueing), ms. */
  gap: number;
  reused: boolean;
}

export type Timing = Pick<
  PerformanceResourceTiming,
  | 'fetchStart'
  | 'domainLookupStart'
  | 'domainLookupEnd'
  | 'connectStart'
  | 'secureConnectionStart'
  | 'connectEnd'
  | 'requestStart'
  | 'responseStart'
  | 'responseEnd'
  | 'nextHopProtocol'
>;

/**
 * The hops of one request. `app` is Server-Timing `app` (ms): when present, request → first byte
 * is split into the network round trip and our server's share, drawn in the middle of it.
 */
export function hops(e: Timing, app: number | undefined, secure: boolean): Trace {
  const o = e.fetchStart;
  const reused = e.connectEnd === e.connectStart && e.domainLookupEnd === e.domainLookupStart;
  const quic = e.nextHopProtocol === 'h3';
  const tlsAt =
    e.secureConnectionStart > 0 ? Math.min(Math.max(e.secureConnectionStart, e.connectStart), e.connectEnd) : e.connectEnd;
  const hop = (id: HopId, a: number, b: number, zero?: Zero): Hop => {
    const ms = Math.max(0, b - a);
    return { id, ms, from: a - o, to: a - o + ms, zero: ms > 0 ? undefined : zero };
  };
  const ttfb = Math.max(0, e.responseStart - e.requestStart);
  const list = [
    hop('dns', e.domainLookupStart, e.domainLookupEnd, reused ? 'reused' : 'cached'),
    hop('tcp', e.connectStart, tlsAt, reused ? 'reused' : quic ? 'quic' : undefined),
    hop('tls', tlsAt, e.connectEnd, !secure ? 'plain' : reused ? 'reused' : quic ? 'quic' : undefined),
  ];
  if (app === undefined) {
    list.push(hop('ttfb', e.requestStart, e.responseStart));
  } else {
    const server = Math.min(Math.max(0, app), ttfb);
    const mid = (e.requestStart + e.responseStart) / 2 - o;
    list.push({ ...hop('net', e.requestStart, e.responseStart), ms: ttfb - server });
    list.push({ id: 'app', ms: server, from: mid - server / 2, to: mid + server / 2 });
  }
  list.push(hop('down', e.responseStart, e.responseEnd));
  const total = Math.max(0, e.responseEnd - o);
  const inside = list.reduce((sum, x) => sum + x.ms, 0);
  return { hops: list, total, gap: Math.max(0, total - inside), reused };
}

/** Server-Timing metrics of one response (ms). */
interface Server {
  app?: number | undefined;
  route?: number | undefined;
  render?: number | undefined;
  tls?: number | undefined;
  krtt?: number | undefined;
}

interface View {
  trace: Trace;
  server: Server;
  proto: string;
  size: number;
  /** GET /_ping (204, no body) rather than the page itself: some explainers differ. */
  ping: boolean;
}

const PROTO: Record<string, string> = { h2: 'HTTP/2', h3: 'HTTP/3', 'http/1.1': 'HTTP/1.1', 'http/1.0': 'HTTP/1.0' };

const SOURCE: Record<HopId, string> = {
  dns: 'domainLookupEnd − domainLookupStart',
  tcp: 'secureConnectionStart − connectStart',
  tls: 'connectEnd − secureConnectionStart',
  net: 'responseStart − requestStart − Server-Timing app',
  app: 'Server-Timing: app, route, render',
  ttfb: 'responseStart − requestStart',
  down: 'responseEnd − responseStart',
};

const fmt = (ms: number): Formatted => (ms === 0 ? { value: '0', unit: 'ms' } : duration(ms));
const text = (ms: number): string => {
  const f = fmt(ms);
  return `${f.value} ${f.unit}`;
};
const num = (f: Formatted): HTMLElement => h('span', { class: 'num' }, f.value, f.unit && h('span', { class: 'num__unit' }, f.unit));

function view(entry: PerformanceResourceTiming, secure: boolean, ping = false): View {
  const st = (name: string) => serverTiming(name, entry);
  const server: Server = { app: st('app'), route: st('route'), render: st('render'), tls: st('tls'), krtt: st('krtt') };
  return {
    trace: hops(entry, server.app, secure),
    server,
    proto: PROTO[entry.nextHopProtocol] ?? entry.nextHopProtocol,
    size: entry.transferSize,
    ping,
  };
}

/** Time a fresh GET /_ping by its Resource Timing entry. Null when the browser reports none. */
async function ping(): Promise<PerformanceResourceTiming | null> {
  const url = new URL('/_ping', location.href).href;
  const since = performance.now() - 1;
  const pick = (list: PerformanceEntryList) =>
    (list.filter((e) => e.name === url && e.startTime >= since).pop() as PerformanceResourceTiming | undefined) ?? null;
  let done = () => {};
  const seen = new Promise<PerformanceResourceTiming | null>((resolve) => {
    const po = new PerformanceObserver((list) => {
      const e = pick(list.getEntries());
      if (e) resolve(e);
    });
    po.observe({ type: 'resource' });
    const timer = setTimeout(() => resolve(pick(performance.getEntriesByType('resource'))), 2000);
    done = () => {
      po.disconnect();
      clearTimeout(timer);
    };
  });
  try {
    const res = await fetch(url, { cache: 'no-store' });
    await res.arrayBuffer();
    return await seen;
  } finally {
    done();
  }
}

/** Draw the timeline for `v` at the plot's current width. Returns the travelling dot. */
function draw(svg: SVGSVGElement, v: View, width: number): SVGCircleElement {
  const W = Math.max(240, Math.round(width));
  const { hops: list, total } = v.trace;
  const k = total > 0 ? (W - 2) / total : 0;
  const slot = W / list.length;
  const named = slot >= 72;
  const dot = s('circle', { class: 'trace-dot', cx: 1, cy: 24, r: 4 });
  const kids: SVGElement[] = [
    s('text', { x: 0, y: 11 }, '0'),
    s('text', { x: W, y: 11, 'text-anchor': 'end' }, text(total)),
    s('rect', { class: 'trace-track', x: 0.5, y: 18.5, width: W - 1, height: 11 }),
  ];
  list.forEach((hop, i) => {
    const x0 = 1 + hop.from * k;
    const x1 = 1 + hop.to * k;
    const mid = (x0 + x1) / 2;
    const at = slot * (i + 0.5);
    if (hop.ms > 0) {
      kids.push(s('rect', { class: `trace-seg trace-seg--${hop.id}`, x: x0, y: 19, width: Math.max(1, x1 - x0), height: 10 }));
    }
    kids.push(
      s('polyline', { class: 'trace-lead', points: `${mid},31 ${mid},38 ${at},50 ${at},54` }),
      s('text', { x: at, y: 68, 'text-anchor': 'middle' }, `0${i + 1}${named ? ' ' + t(`trace-${hop.id}-short`) : ''}`),
      s('text', { class: 'trace-v', x: at, y: 84, 'text-anchor': 'middle' }, text(hop.ms)),
    );
  });
  kids.push(dot);
  svg.setAttribute('viewBox', `0 0 ${W} 92`);
  svg.setAttribute('width', String(W));
  svg.setAttribute('height', '92');
  render(svg, kids);
  return dot;
}

/** The ordered list mirroring the timeline, with an explainer per hop. */
function list(v: View, secure: boolean): HTMLElement[] {
  const { server } = v;
  const extra: Partial<Record<HopId, (Node | string)[] | false>> = {
    tls: server.tls !== undefined && tn('trace-tls-server', { ms: num(fmt(server.tls)) }),
    net: server.krtt !== undefined && tn('trace-krtt', { ms: num(fmt(server.krtt)) }),
    app:
      server.route !== undefined &&
      server.render !== undefined &&
      tn('trace-app-split', { route: num(fmt(server.route)), render: num(fmt(server.render)) }),
  };
  return v.trace.hops.map((hop, i) => {
    const note = hop.zero && t(`trace-zero-${hop.zero}`);
    const more = extra[hop.id];
    const source = hop.id === 'tcp' && !secure ? 'connectEnd − connectStart' : SOURCE[hop.id];
    return h(
      'li',
      { class: 'trace-hop' },
      h(
        'details',
        null,
        h(
          'summary',
          null,
          // Spaces between the parts keep the accessible name readable; the grid ignores them.
          h('span', { class: 'trace-hop__n' }, `0${i + 1}`),
          ' ',
          h('span', { class: 'trace-hop__name' }, t(`trace-${hop.id}`)),
          ' ',
          num(fmt(hop.ms)),
          (note || more) && [' ', h('span', { class: 'trace-hop__x' }, note, note && more && ' · ', more)],
        ),
        h('p', { class: 'trace-hop__about' }, t(`trace-${hop.id}-about${v.ping && (hop.id === 'app' || hop.id === 'down') ? '-ping' : ''}`)),
        h('p', { class: 'source' }, source),
      ),
    );
  });
}

function setStatus(root: HTMLElement, label: string, kind: string): void {
  const el = $('.instrument__status', root);
  if (!el) return;
  el.textContent = label;
  el.className = `instrument__status badge badge--${kind}`;
}

export function mount(root: HTMLElement): void {
  const body = $('.instrument__body', root) ?? root.appendChild(h('div', { class: 'instrument__body' }));
  const nav = navEntry();
  if (!nav || !('PerformanceObserver' in window)) {
    render(body, h('p', { class: 'instrument__fallback' }, t('trace-unsupported')));
    setStatus(root, t('trace-status-none'), 'plain');
    return;
  }
  const secure = location.protocol === 'https:';
  const cached = nav.transferSize === 0 && nav.decodedBodySize > 0;
  const views: (View | null)[] = [cached ? null : view(nav, secure)];
  let shown = 0;
  let sent = 1;
  let busy = false;
  let width = 0;
  let dot: SVGCircleElement | null = null;

  const svg = s('svg', { class: 'trace-svg', 'aria-hidden': 'true', focusable: 'false' });
  const plot = h('div', { class: 'trace-plot' }, svg);
  const meta = h('p', { class: 'trace-meta' });
  const hopList = h('ol', { class: 'trace-hops', role: 'list' });
  const gap = h('p', { class: 'trace-gap source' });
  const said = h('p', { class: 'trace-said', 'aria-live': 'polite' });
  const chips = [t('trace-page'), t('trace-latest')].map((label, i) =>
    h('button', { class: 'chip', type: 'button', 'aria-pressed': String(i === 0), hidden: i === 1, onclick: () => show(i) }, label),
  );
  const again = h('button', { class: 'btn btn--ghost btn--sm', type: 'button', onclick: () => void send() }, t('trace-again'));

  function show(i: number): void {
    shown = i;
    chips.forEach((c, j) => c.setAttribute('aria-pressed', String(i === j)));
    const v = views[i];
    plot.hidden = gap.hidden = !v;
    if (!v) {
      meta.textContent = t('trace-cached');
      render(hopList);
      return;
    }
    const parts = [
      v.proto && [v.proto],
      v.size > 0 && tn('trace-size', { size: num(bytes(v.size)) }),
      tn('trace-total', { total: num(fmt(v.trace.total)) }),
    ].filter((p) => p !== false && p !== '');
    render(meta, parts.map((p, i) => [i ? ' · ' : '', p]));
    dot = draw(svg, v, width || plot.clientWidth);
    render(hopList, list(v, secure));
    render(gap, ...(v.trace.gap >= 0.05 ? tn('trace-gap', { ms: num(fmt(v.trace.gap)) }) : []));
  }

  async function send(): Promise<void> {
    if (busy) return;
    busy = true;
    again.setAttribute('aria-disabled', 'true');
    try {
      const entry = await ping();
      if (!entry) throw new Error('no Resource Timing entry');
      const v = view(entry, secure, true);
      views[1] = v;
      sent++;
      chips[1]!.hidden = false;
      show(1);
      const app = v.server.app;
      said.textContent =
        app === undefined
          ? t('trace-done-short', { n: sent, total: text(v.trace.total) })
          : t('trace-done', { n: sent, total: text(v.trace.total), app: text(Math.min(app, v.trace.total)) });
      travel();
    } catch {
      said.textContent = t('trace-failed');
    } finally {
      busy = false;
      again.removeAttribute('aria-disabled');
    }
  }

  // A packet's journey along the path, left to right. Reduced motion: the numbers just update.
  function travel(): void {
    const d = dot;
    if (!d || reducedMotion()) return;
    const end = svg.viewBox.baseVal.width - 1;
    const t0 = performance.now();
    d.classList.add('is-on');
    const step = (now: number) => {
      const p = Math.min(1, (now - t0) / 900);
      const eased = p < 0.5 ? 2 * p * p : 1 - (2 - 2 * p) ** 2 / 2;
      d.setAttribute('cx', String(1 + eased * (end - 1)));
      if (p < 1) requestAnimationFrame(step);
      else d.classList.remove('is-on');
    };
    requestAnimationFrame(step);
  }

  render(
    body,
    h(
      'div',
      { class: 'trace' },
      h('div', { class: 'trace-head' }, h('div', { class: 'trace-pick' }, chips), meta),
      plot,
      hopList,
      gap,
      h('div', { class: 'trace-act' }, again, said),
    ),
  );
  setStatus(root, t('trace-status'), 'info');

  // Redraw at the plot's real width, so the mono labels stay at their CSS size on any screen.
  if ('ResizeObserver' in window) {
    new ResizeObserver((entries) => {
      const w = Math.round(entries[0]?.contentRect.width ?? 0);
      if (!w || w === width) return;
      width = w;
      const v = views[shown];
      if (v) dot = draw(svg, v, w);
    }).observe(plot);
  }
  show(0);
}
