// Real-user Web Vitals for this server (spec §12, docs/CONTRACTS.md §14). Loaded on idle by
// main.ts. Measures this page view with PerformanceObserver and Navigation Timing — no library —
// and, when the page is first hidden, sends one beacon with navigator.sendBeacon('/_rum'): the
// page's canonical route, TTFB, FCP, LCP, CLS and INP, and the counts of web/shared/rum.ts. If the
// visitor comes back and does more, the next hide sends only the new counts. Under Global Privacy
// Control or Do Not Track it measures and sends nothing.

import { lastValue, on } from '../shared/bus.ts';
import { declined, take, track } from '../shared/rum.ts';
import { navEntry, offlineCopy } from '../shared/timing.ts';
import { cls, type Shift } from '../hud/metrics.ts';
import { beacon, inp, route, worthSending } from './vitals.ts';

let started = false;

export function start(): void {
  if (started || declined()) return;
  started = true;

  const nav = navEntry();
  // A copy the service worker served offline says nothing about this server: counts only.
  const ours = !!nav && !offlineCopy(nav);
  const activation = (nav as (PerformanceNavigationTiming & { activationStart?: number }) | undefined)?.activationStart ?? 0;
  const since = (t: number) => Math.max(0, t - activation);
  // Paint metrics count only while the page has been visible from the start (as CrUX does).
  let hiddenAt = document.visibilityState === 'hidden' ? 0 : Infinity;
  for (const e of performance.getEntriesByType('visibility-state')) {
    if (e.name === 'hidden') hiddenAt = Math.min(hiddenAt, e.startTime);
  }

  const supported = PerformanceObserver.supportedEntryTypes ?? [];
  const flush: (() => void)[] = [];
  const watch = (type: string, fn: (entries: PerformanceEntryList) => void, extra?: object): boolean => {
    if (!supported.includes(type)) return false;
    try {
      const po = new PerformanceObserver((list) => fn(list.getEntries()));
      po.observe({ type, buffered: true, ...extra } as PerformanceObserverInit);
      flush.push(() => fn(po.takeRecords()));
      return true;
    } catch {
      return false;
    }
  };

  let fcp: number | undefined;
  let lcp: number | undefined;
  const shifts: Shift[] = [];
  const worst = new Map<number, number>();
  watch('paint', (entries) => {
    for (const e of entries) if (e.name === 'first-contentful-paint' && e.startTime < hiddenAt) fcp = since(e.startTime);
  });
  watch('largest-contentful-paint', (entries) => {
    const e = entries.at(-1);
    if (e && e.startTime < hiddenAt) lcp = since(e.startTime);
  });
  const layout = watch('layout-shift', (entries) => void shifts.push(...(entries as unknown as Shift[])));
  const interactions = (entries: PerformanceEntryList) => {
    for (const e of entries as PerformanceEventTiming[]) {
      if (e.interactionId) worst.set(e.interactionId, Math.max(worst.get(e.interactionId) ?? 0, e.duration));
    }
  };
  watch('event', interactions, { durationThreshold: 16 });
  watch('first-input', interactions);

  on('contact:sent', () => track('contact_submit'));

  let first = true;
  const send = () => {
    for (const f of flush) f();
    const view = first && ours;
    const counts = take();
    const tier = first ? lastValue('tier') : undefined;
    if (tier) counts[`render_tier:${tier}`] = 1;
    const count = (performance as Performance & { interactionCount?: number }).interactionCount;
    const metrics = view
      ? { ttfb: nav && nav.responseStart > 0 ? since(nav.responseStart) : undefined, fcp, lcp, cls: layout ? cls(shifts) : undefined, inp: inp(worst.values(), count) }
      : {};
    const canonical = document.querySelector<HTMLLinkElement>('link[rel="canonical"]')?.href;
    const b = beacon(route(canonical, location.pathname), view, metrics, counts);
    first = false;
    if (worthSending(b)) navigator.sendBeacon('/_rum', JSON.stringify(b));
  };
  // `pagehide` too: some browsers skip `visibilitychange` when the page is unloaded.
  document.addEventListener('visibilitychange', () => document.visibilityState === 'hidden' && send());
  addEventListener('pagehide', send);
}
