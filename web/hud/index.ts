// Proof console: a pill with this page's vital signs and a panel saying where each number comes
// from. Also fills the footer's live line (#footer-stats). On mount it reads only what the browser
// already measured, plus GET /_proof; the HEAD request for headers and GET /_rum (other visitors'
// totals) wait for the panel to open.

import { $, h, num, render, s, type Child } from '../shared/dom.ts';
import { lang, pluralKey, t, tn, tp } from '../shared/i18n.ts';
import { getProof, type Proof } from '../shared/proof.ts';
import { offlineCopy, pageWeight, serverTiming } from '../shared/timing.ts';
import { bytes, duration } from '../shared/format.ts';
import { on } from '../shared/bus.ts';
import { offline } from '../shared/env.ts';
import { track } from '../shared/rum.ts';
import { cls, clsText, cookies, httpVersion, slowest, tlsVersion, transport, type Shift } from './metrics.ts';

const T = (key: string, vars?: Record<string, string | number>): string => t('hud-' + key, vars);
const WIDE = ['third', 'tls', 'headers', 'visitors'];

let pill: HTMLButtonElement | undefined;
let panel: HTMLElement;
let proof: Proof | null | undefined; // undefined: on its way · null: failed
let headed = false;
const cells: Record<string, HTMLElement> = {};
const rows: Record<string, HTMLElement> = {};
const parts: Record<string, HTMLElement> = {};
const L = lang();
const ms = (x: number) => num(duration(x, L));
const count = (n: number, cls?: string) => num({ value: new Intl.NumberFormat(L).format(n), unit: '' }, cls);
const tick = () => s('svg', { class: 'hud-tick', viewBox: '0 0 12 12', 'aria-hidden': 'true' }, s('path', { d: 'M2.5 6.5 5 9l4.5-6' }));
const set = (key: string, ...value: Child[]) => render(cells[key]!, ...value);
const vh = (text: string) => h('span', { class: 'visually-hidden' }, text);

export function mount(): void {
  if (pill) return;
  const row = (key: string) => {
    cells[key] = h('dd', { class: 'hud-v' }, T(key === 'inp' ? 'inp-none' : key === 'tier' ? 'tier-none' : 'pending'));
    return (rows[key] = h(
      'div',
      { class: WIDE.includes(key) ? 'hud-row hud-row--wide' : 'hud-row' },
      h('dt', null, T('k-' + key)),
      cells[key],
      h('dd', { class: 'source' }, T('src-' + key)),
    ));
  };
  const raw = h('pre', { class: 'hud-raw', id: 'hud-raw', hidden: true });
  const rawButton: HTMLButtonElement = h(
    'button',
    {
      type: 'button',
      class: 'btn btn--quiet btn--sm',
      'aria-expanded': 'false',
      'aria-controls': 'hud-raw',
      onclick: () => rawButton.setAttribute('aria-expanded', String(!(raw.hidden = !raw.hidden))),
    },
    T('raw'),
  );
  cells['raw'] = raw;
  for (const key of ['time', 'weight', 'tls']) parts[key] = h('span', { class: `hud-part hud-part--${key}`, hidden: true });
  pill = h(
    'button',
    { type: 'button', class: 'hud-pill', 'aria-expanded': 'false', 'aria-controls': 'hud-panel', onclick: () => toggle() },
    h('span', { class: 'hud-dot', 'aria-hidden': 'true' }),
    vh(T('pill-label')),
    parts['time'],
    parts['weight'],
    parts['tls'],
  );
  panel = h(
    'div',
    { class: 'hud-panel', id: 'hud-panel', role: 'dialog', 'aria-labelledby': 'hud-id hud-title', tabindex: '-1', hidden: true },
    h(
      'div',
      { class: 'hud-bar' },
      h('span', { class: 'hud-id', id: 'hud-id' }, T('id')),
      h('h2', { class: 'hud-title', id: 'hud-title' }, T('title')),
      h('button', { type: 'button', class: 'btn btn--quiet btn--sm hud-close', onclick: () => toggle(false) }, T('close')),
    ),
    h('dl', { class: 'hud-rows' }, ['server', 'weight', 'third', 'cookies', 'tls', 'headers', 'lcp', 'inp', 'cls', 'tier', 'visitors'].map(row)),
    h('div', { class: 'hud-foot' }, rawButton, raw),
  );
  const onkeydown = (e: KeyboardEvent) => {
    if (e.key === 'Escape' && !panel.hidden) {
      e.preventDefault();
      toggle(false);
    }
  };
  document.body.append(h('aside', { class: 'hud', 'aria-label': T('region'), onkeydown }, pill, panel));

  local();
  vitals();
  on('tier', (tier) => set('tier', T('tier-' + tier.toLowerCase())), { replay: true });
  getProof().then(
    (p) => ((proof = p), local()),
    () => ((proof = null), local()),
  );
  let frame = 0;
  try {
    // Islands load lazily; keep the weight current without polling.
    new PerformanceObserver(() => (frame ||= requestAnimationFrame(() => ((frame = 0), local())))).observe({ type: 'resource' });
  } catch {
    /* no Resource Timing observer: the numbers refresh when the panel opens */
  }
}

/** Open the panel (idempotent; mounts first if needed). */
export function open(): void {
  mount();
  toggle(true);
}

function toggle(show = panel.hidden): void {
  if (show !== panel.hidden) {
    if (show) panel.focus();
    return;
  }
  panel.hidden = !show;
  pill!.setAttribute('aria-expanded', String(show));
  if (!show) return pill!.focus();
  track('proof_open');
  if (proof === null) getProof().then((p) => ((proof = p), local()), () => {}); // retry once per opening
  local();
  headers();
  // Other visitors' totals change while the page is open: asked again on every opening.
  import('./visitors.ts').then((m) => m.show(cells['visitors']!), () => set('visitors', T(offline() ? 'offline' : 'pending')));
  panel.focus();
}

/** Everything the browser already measured, plus /_proof once it arrives. Cheap; no requests. */
function local(): void {
  const app = serverTiming('app');
  const w = pageWeight();
  const jar = cookies(document.cookie, proof?.privacy.cookies_set);
  const third = w.thirdPartyOrigins;
  const weight = () => (w.transferred > 0 ? num(bytes(w.transferred, L)) : null);
  const mode = transport(proof);

  pillPart('time', app === undefined ? null : [vh(T('pill-server')), ms(app)]);
  pillPart('weight', w.transferred > 0 && [vh(T('pill-weight')), weight()]);
  pillPart(
    'tls',
    mode === 'pq'
      ? ['PQ', tick(), vh(T('pill-pq'))]
      : mode === 'dev'
        ? T('pill-dev')
        : mode,
  );

  // Blame the browser only when the API itself is missing; otherwise the server didn't report it.
  set('server', app !== undefined ? ms(app) : T(offlineCopy() ? 'offline-copy' : 'PerformanceServerTiming' in globalThis ? 'not-reported' : 'na'));
  set(
    'weight',
    'transferSize' in PerformanceResourceTiming.prototype ? num(bytes(w.transferred, L)) : T('na'),
    ' ',
    h('span', { class: 'hud-sub' }, tp('hud-requests', w.requests, { cached: w.fromCache })),
  );
  set('third', count(third.length, third.length ? 'hud-bad' : ''), third.length > 0 && h('ul', { class: 'hud-list', role: 'list' }, third.map((o) => h('li', null, o))));
  set(
    'cookies',
    count(jar.count, jar.ours ? 'hud-bad' : ''),
    jar.count > 0 && jar.ours !== undefined && h('span', { class: 'hud-sub' }, T(jar.ours ? 'cookies-ours' : 'cookies-foreign')),
  );
  if (proof === null) set('tls', T(offline() ? 'offline' : 'err-proof'));
  else if (proof) {
    const http = httpVersion(proof.http.version);
    const tls = proof.tls;
    // A fact the server did not report (null) is left out, never filled in.
    const facts = tls && [tlsVersion(tls.version), tls.cipher, tls.group, http].filter((f) => f !== null);
    set('tls', facts ? h('span', { class: 'mono' }, facts.join(' · ')) : T('plain', { http }));
  }

  const footer = $('#footer-stats');
  if (!footer) return;
  const bits: Child[][] = [];
  if (w.transferred > 0) bits.push([weight()]);
  if (app !== undefined) bits.push(tn('hud-foot-server', { value: ms(app) }));
  // The site's own cookies: what /_proof reports, or what scripts see when that is nothing at all.
  if (jar.ours !== undefined) bits.push(tn(pluralKey('hud-foot-cookies', jar.ours ? jar.count : 0), { n: count(jar.ours ? jar.count : 0) }));
  bits.push(tn(pluralKey('hud-foot-third', third.length), { n: count(third.length) }));
  if (proof) bits.push([proof.tls ? (proof.tls.post_quantum ? T('foot-pq', { tls: tlsVersion(proof.tls.version) }) : tlsVersion(proof.tls.version)) : T('foot-dev')]);
  render(footer, T('foot-lead'), ' ', bits.map((bit, i) => [i ? ' · ' : '', ...bit]));
}

function pillPart(key: string, content: Child): void {
  const el = parts[key]!;
  el.hidden = content === null || content === undefined || content === false;
  render(el, content);
}

/** LCP, INP (slowest interaction so far) and CLS, where the browser supports them. */
function vitals(): void {
  const supported = PerformanceObserver.supportedEntryTypes ?? [];
  const watch = (type: string, fn: (entries: PerformanceEntry[]) => void, extra?: object): boolean => {
    if (!supported.includes(type)) return false;
    try {
      new PerformanceObserver((list) => fn(list.getEntries())).observe({ type, buffered: true, ...extra } as PerformanceObserverInit);
      return true;
    } catch {
      return false;
    }
  };
  rows['lcp']!.hidden = !watch('largest-contentful-paint', (entries) => {
    const last = entries.at(-1);
    if (last) set('lcp', ms(last.startTime));
  });
  const shifts: Shift[] = [];
  const showCls = () => set('cls', num({ value: clsText(cls(shifts), L), unit: '' }));
  rows['cls']!.hidden = !watch('layout-shift', (entries) => {
    shifts.push(...(entries as unknown as Shift[]));
    showCls();
  });
  if (!rows['cls']!.hidden) showCls();
  let worst = 0;
  const onEvents = (entries: PerformanceEntry[]) => {
    worst = slowest(entries as PerformanceEventTiming[], worst);
    if (worst > 0) set('inp', ms(worst));
  };
  // INP groups events by interactionId; without it there is nothing honest to show.
  const interactions = typeof PerformanceEventTiming !== 'undefined' && 'interactionId' in PerformanceEventTiming.prototype;
  rows['inp']!.hidden = !(interactions && watch('event', onEvents, { durationThreshold: 16 }));
  if (interactions) watch('first-input', onEvents);
}

/** The security-header row and raw view: a lazily imported chunk, loaded on first opening. */
function headers(): void {
  if (headed) return;
  headed = true;
  import('./headers.ts')
    .then((m) => m.scan(cells['headers']!, cells['raw']!))
    .catch((err: unknown) => {
      headed = false; // try again next time the panel opens
      set('headers', offline() ? T('offline') : T('err-head', { reason: err instanceof Error ? err.message : String(err) }));
    });
}
