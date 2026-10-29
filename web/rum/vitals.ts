// Web Vitals from raw performance entries, as browsers and the Chrome UX Report define them, and
// the beacon built from them. Pure — no DOM — so Node tests import it; web/rum/index.ts feeds it.

import type { Metric, RumBeacon } from '../shared/rum.ts';

/** The beacon's event keys (crates/proof/src/rum.rs `EVENTS`): anything else would void the beacon. */
export const EVENT_KEYS: readonly string[] = [
  'cta_start_project',
  'proof_open',
  ...['start', 'complete'].flatMap((k) => ['physics', 'efficiency', 'security', 'highload', 'design'].map((p) => `instrument_${k}:${p}`)),
  'terminal_open',
  'ctf_stage:1',
  'ctf_stage:2',
  'ctf_stage:3',
  'ctf_stage:4',
  'contact_submit',
  'lang_switch',
  'render_tier:A',
  'render_tier:B',
  'render_tier:C',
  'render_tier:D',
];

/** The server's ranges: times up to 60 s, CLS up to 10. */
const MAX: Record<Metric, number> = { ttfb: 60_000, fcp: 60_000, lcp: 60_000, cls: 10, inp: 60_000 };

/**
 * Interaction to Next Paint: the slowest interaction, ignoring the slowest one of every 50 (so a
 * page with 120 interactions reports its third slowest). `worst` holds each interaction's longest
 * event duration; `count` is how many interactions there were (performance.interactionCount when
 * the browser has it). Undefined without interactions.
 */
export function inp(worst: Iterable<number>, count?: number): number | undefined {
  const sorted = [...worst].sort((a, b) => b - a);
  if (!sorted.length) return undefined;
  const n = Math.max(count ?? 0, sorted.length);
  return sorted[Math.min(Math.floor(n / 50), sorted.length - 1)];
}

/** The route a beacon names: the page's canonical path, or "404" for the not-found page. */
export function route(canonical: string | undefined, fallback: string): string {
  let path = fallback;
  try {
    if (canonical) path = new URL(canonical).pathname;
  } catch {
    /* keep the fallback */
  }
  return path.endsWith('/404') ? '404' : path;
}

/**
 * A beacon: times rounded to 0.1 ms and CLS to 0.0001, anything unmeasured or out of range left
 * out, events filtered to known keys with counts of 1–20. `view` marks the page view's first.
 */
export function beacon(path: string, view: boolean, metrics: Partial<Record<Metric, number | undefined>>, events: Record<string, number>): RumBeacon {
  const out: RumBeacon = { route: path };
  if (view) {
    out.view = true;
    const m: Partial<Record<Metric, number>> = {};
    for (const [name, value] of Object.entries(metrics) as [Metric, number | undefined][]) {
      if (value === undefined || !Number.isFinite(value) || !(name in MAX)) continue;
      const digits = name === 'cls' ? 1e4 : 10;
      const v = Math.round(Math.max(0, value) * digits) / digits;
      if (v <= MAX[name]) m[name] = v;
    }
    if (Object.keys(m).length) out.metrics = m;
  }
  const e: Record<string, number> = {};
  for (const [key, n] of Object.entries(events)) if (EVENT_KEYS.includes(key) && n >= 1) e[key] = Math.min(20, Math.round(n));
  if (Object.keys(e).length) out.events = e;
  return out;
}

/** Worth sending: a page view, or an event that happened since the last beacon. */
export const worthSending = (b: RumBeacon): boolean => b.view === true || b.events !== undefined;
