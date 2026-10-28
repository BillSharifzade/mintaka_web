// S2 · Edge of physics. Nothing happens until the visitor presses "Measure my distance". Then,
// all in the browser:
//   1. the measured round trip: the kernel's RTT for this connection (Server-Timing `krtt`), else
//      Navigation Timing, else the fastest of 5 requests to /_ping — minus our server's own time;
//   2. an upper bound on distance that needs no location: light can't go further in that time;
//   3. a floor estimated from the device's time zone → the zone's principal city (IANA tz
//      database, imported only now) → great-circle distance to the server's configured location.
// No IP lookup and no geolocation API: nothing about the visitor's location leaves the browser.

import { $, h, num, render } from '../shared/dom.ts';
import { lang, t, tn, tp } from '../shared/i18n.ts';
import { getProof } from '../shared/proof.ts';
import { navEntry, ping, serverTiming } from '../shared/timing.ts';
import { duration, km, type Formatted } from '../shared/format.ts';
import { locate, type Place } from './tz/tzdb.ts';

/** Light in optical fibre, km per ms: c ÷ 1.468 (group index of standard single-mode fibre). */
export const FIBRE = 299_792.458 / 1.468 / 1000;
/** Mean Earth radius (IUGG), km. */
const EARTH = 6371.0088;

export interface Point {
  lat: number;
  lon: number;
}

/** Great-circle distance on a sphere, km. */
export function haversine(a: Point, b: Point): number {
  const rad = Math.PI / 180;
  const x =
    Math.sin(((b.lat - a.lat) * rad) / 2) ** 2 +
    Math.cos(a.lat * rad) * Math.cos(b.lat * rad) * Math.sin(((b.lon - a.lon) * rad) / 2) ** 2;
  return 2 * EARTH * Math.asin(Math.min(1, Math.sqrt(x)));
}

/** The farthest the visitor can be from the server, km: light covers rtt/2 × FIBRE one way. */
export const reach = (rttMs: number): number => (rttMs / 2) * FIBRE;

/** The shortest round trip light in fibre allows over a distance, ms. */
export const lightFloor = (km: number): number => (2 * km) / FIBRE;

export type Source = 'kernel' | 'navigation' | 'ping';

export interface Trip {
  /** Network round trip, ms (our server's time already taken out). */
  rtt: number;
  /** Our server's time for that response (Server-Timing `app`), ms. */
  server: number | undefined;
  source: Source;
}

/** The round trip this page view already measured, from its navigation entry. */
export function fromNavigation(nav: PerformanceNavigationTiming | undefined): Trip | undefined {
  if (!nav) return undefined;
  const server = serverTiming('app', nav);
  const krtt = serverTiming('krtt', nav);
  if (krtt !== undefined && krtt > 0) return { rtt: krtt, server, source: 'kernel' };
  const wait = nav.responseStart - nav.requestStart - (server ?? 0);
  const fromCache = nav.transferSize === 0 && nav.decodedBodySize > 0;
  if (nav.requestStart > 0 && wait > 0 && !fromCache) return { rtt: wait, server, source: 'navigation' };
  return undefined;
}

const PINGS = 5;

/** The fastest of PINGS sequential requests to /_ping, server time taken out. */
async function fromPings(): Promise<Trip> {
  let best: Trip | undefined;
  for (let i = 0; i < PINGS; i++) {
    const p = await ping();
    const rtt = p.total - (p.app ?? 0);
    if (!best || rtt < best.rtt) best = { rtt, server: p.app, source: 'ping' };
  }
  return best!;
}

export type Verdict =
  | { kind: 'floor'; place: Place; distance: number; floor: number; detours: number }
  | { kind: 'faster'; place: Place; distance: number; floor: number }
  | { kind: 'no-proof' | 'no-server' | 'no-zone' };

/**
 * Compare the measured round trip with what light needs from the time zone's city.
 * `server` is the configured location: null when unconfigured, undefined when /_proof failed.
 */
export function judge(rtt: number, place: Place | null, server: Point | null | undefined): Verdict {
  if (server === undefined) return { kind: 'no-proof' };
  if (server === null) return { kind: 'no-server' };
  if (!place) return { kind: 'no-zone' };
  const distance = haversine(place, server);
  const floor = lightFloor(distance);
  if (floor > rtt) return { kind: 'faster', place, distance, floor };
  return { kind: 'floor', place, distance, floor, detours: rtt - floor };
}

/** Round an upper bound up to the precision km() shows, so the claim stays true. */
export function ceilKm(x: number): number {
  if (x < 10) return Math.ceil(x * 10 - 1e-9) / 10;
  return x < 1000 ? Math.ceil(x - 1e-9) : Math.ceil(x / 10 - 1e-9) * 10;
}

/**
 * Durations as this instrument shows them: µs below 1 ms, else ms with one decimal, so the parts
 * of the breakdown visibly add up to the round trip.
 */
export function msText(x: number, locale?: string): Formatted {
  if (x < 1) return duration(x, locale);
  return { value: new Intl.NumberFormat(locale, { minimumFractionDigits: 1, maximumFractionDigits: 1 }).format(x), unit: 'ms' };
}

/** Round a lower bound down to the precision msText() shows, so "at least" stays true. */
export function floorMs(x: number): number {
  const k = x < 0.01 ? 10_000 : x < 1 ? 1000 : 10; // 0.1 µs, 1 µs, 0.1 ms
  return Math.floor(x * k + 1e-9) / k;
}

// ——— The island ———

const T = (key: string, vars?: Record<string, string | number>): string => t('physics-' + key, vars);
const TN = (key: string, vars: Record<string, Node | string | number>) => tn('physics-' + key, vars);

export function mount(root: HTMLElement): void {
  const body = $('.instrument__body', root) ?? root.appendChild(h('div', { class: 'instrument__body' }));
  const status = $('.instrument__status', root);
  const L = lang();
  const ms = (x: number) => num(msText(x, L));
  const dist = (x: number) => num(km(x, L));

  const button = h('button', { class: 'btn btn--ghost', type: 'button' }, T('measure'));
  const figs = h('div');
  const say = h('div', { class: 'phys-say', 'aria-live': 'polite' });
  const more = h('div');
  const navServer = serverTiming('app');

  render(
    body,
    h(
      'div',
      { class: 'phys' },
      navServer !== undefined && h('p', { class: 'phys-intro' }, ...TN('intro', { server: ms(navServer) })),
      h('div', { class: 'phys-act' }, button, h('p', { class: 'phys-hint' }, T('hint'))),
      figs,
      say,
      more,
    ),
  );
  if (status) status.textContent = T('status-ready');

  let measured = false;
  button.addEventListener('click', async () => {
    button.disabled = true;
    button.textContent = T('measuring');
    try {
      // This page view's own measurement first (kernel, then browser); fresh pings on "again".
      const tz = Intl.DateTimeFormat().resolvedOptions().timeZone ?? '';
      const proof = getProof().catch(() => null);
      const data = import('./tz/data.ts');
      const known = measured ? undefined : fromNavigation(navEntry());
      // Lay the result out now, inside the click's 500 ms input window, and fill it in place:
      // late growth would count as layout shift, and this page reports its own CLS.
      show(known, null, tz, '');
      const trip = known ?? (await fromPings());
      const where = await proof.then((p) => (p ? p.server.location : undefined));
      const place = tz ? locate(tz, (await data).zones) : null;
      show(trip, judge(trip.rtt, place, where), tz, where?.city ?? '');
      measured = true;
      if (status) {
        status.textContent = T('status-measured');
        status.classList.add('badge--info');
      }
    } catch (err) {
      render(figs);
      render(more);
      render(say, h('p', null, T('error', { reason: err instanceof Error ? err.message : String(err) })));
    } finally {
      button.disabled = false;
      button.textContent = T(measured ? 'again' : 'measure');
    }
  });

  /** The whole result. `trip` undefined: still measuring; `v` null: the floor is still coming. */
  function show(trip: Trip | undefined, v: Verdict | null, tz: string, serverCity: string): void {
    const located = v?.kind === 'floor' || v?.kind === 'faster';
    const floor = located ? floorMs(v.floor) : undefined;
    const bound = trip && ceilKm(reach(trip.rtt));
    const fig = (label: string, value: Node | string, source: (Node | string)[]) =>
      h('div', null, h('dt', { class: 'phys-k' }, label), h('dd', { class: 'phys-v' }, value), h('dd', { class: 'source' }, ...source));

    render(
      figs,
      h(
        'dl',
        { class: 'phys-figs' },
        fig(T('rtt-label'), trip ? ms(trip.rtt) : '—', [!trip ? T('measuring') : trip.source === 'ping' ? tp('physics-src-ping', PINGS) : T('src-' + trip.source)]),
        fig(T('bound-label'), bound !== undefined ? dist(bound) : '—', [T('bound-src')]),
        fig(
          T('floor-label'),
          floor !== undefined ? ms(floor) : '—',
          located ? TN('floor-src', { zone: tz, city: v.place.city, km: dist(v.distance) }) : [T(v ? 'floor-none' : 'measuring')],
        ),
      ),
    );

    const lines: (Node | string)[][] = [];
    if (v?.kind === 'faster') lines.push(TN('faster', { city: v.place.city, km: dist(bound!) }));
    else if (trip) lines.push(TN('bound', { rtt: ms(trip.rtt), km: dist(bound!) }));
    if (!v || !trip) lines.push([T('measuring')]);
    else if (v.kind === 'floor') lines.push(TN('floor', { floor: ms(floor!), city: v.place.city, server_city: serverCity, rtt: ms(trip.rtt) }));
    else if (!located) lines.push([T(v.kind, { zone: tz })]);
    if (v?.kind === 'floor' && trip) lines.push([...TN('gap', { detours: ms(trip.rtt - floor!) }), ...(trip.server === undefined ? [] : [' ', ...TN('server', { server: ms(trip.server) })])]);
    else if (v && trip && trip.server !== undefined) lines.push(TN('server', { server: ms(trip.server) }));
    render(say, ...lines.map((line) => h('p', null, ...line)));

    // [physics floor][network detours][our server], or [round trip][our server] without a floor.
    const parts: [string, number | undefined][] =
      v?.kind === 'floor' && trip
        ? [['floor', floor], ['net', trip.rtt - floor!], ['srv', trip.server]]
        : v && trip
          ? [['rtt', trip.rtt], ['srv', trip.server]]
          : [['floor', undefined], ['net', undefined], ['srv', undefined]];
    render(more, bar(parts), h('p', { class: 'phys-foot source' }, T('foot')));
  }

  function bar(parts: [string, number | undefined][]): HTMLElement {
    const known = parts.filter(([, x]) => x !== undefined) as [string, number][];
    const total = known.reduce((sum, [, x]) => sum + x, 0);
    const track = h('div', { class: 'phys-bar', 'aria-hidden': 'true' });
    const legend = h('ul', { class: 'phys-legend', role: 'list' });
    for (const [key, x] of parts) {
      if (x !== undefined && total > 0) {
        const seg = h('span', { class: `phys-seg phys-seg--${key}` });
        seg.style.setProperty('--phys-w', String(x / total));
        track.append(seg);
      }
      if (key === 'srv' && x === undefined && known.length) continue; // no Server-Timing: no server part
      legend.append(h('li', null, h('span', { class: `phys-sw phys-seg--${key}`, 'aria-hidden': 'true' }), T('part-' + key), ' ', x === undefined ? '—' : ms(x)));
    }
    return h('figure', { class: 'phys-break' }, h('figcaption', { class: 'phys-k' }, ...TN('bar-label', { total: known.length ? ms(total) : '—' })), track, legend);
  }
}
