// Event counts for the self-hosted real-user measurements (POST /_rum, docs/CONTRACTS.md §10.2
// and §14). track() only adds one to a counter in this page view's memory, so it is safe to call
// at any time: before the collector (web/rum/index.ts, loaded on idle) exists, or when it never
// loads. The collector sends the counts with the page view's beacon when the page is hidden.
// Nothing leaves the browser before that, and nothing at all under Global Privacy Control or
// Do Not Track.

export type Pillar = 'efficiency' | 'security' | 'highload' | 'design';

/** Each event and the value it carries (`undefined`: none). Mirrors `EVENTS` in crates/proof/src/rum.rs. */
export interface RumEvents {
  /** A link to #contact ("Start a project") was followed. */
  cta_start_project: undefined;
  /** The proof console opened. */
  proof_open: undefined;
  /** The visitor started an instrument run: S2 physics as 'physics', the rest by pillar. */
  instrument_start: Pillar | 'physics';
  /** That run finished and showed its result. */
  instrument_complete: Pillar | 'physics';
  terminal_open: undefined;
  /** Stage n of the console challenge was solved. */
  ctf_stage: 1 | 2 | 3 | 4;
  /** The contact form was accepted by the server. */
  contact_submit: undefined;
  lang_switch: undefined;
}

/** The visitor's browser asks sites not to measure it (Global Privacy Control or Do Not Track). */
export function declined(): boolean {
  const n = navigator as Navigator & { globalPrivacyControl?: boolean };
  return n.globalPrivacyControl === true || n.doNotTrack === '1';
}

/** Most one beacon reports for one key (the server refuses more). */
export const MAX_COUNT = 20;

const counts = new Map<string, number>();

/** Count one event: `track('proof_open')`, `track('ctf_stage', 2)`, `track('instrument_start', 'security')`. */
export function track<K extends keyof RumEvents>(name: K, ...value: RumEvents[K] extends undefined ? [] : [RumEvents[K]]): void {
  const key = value.length ? `${name}:${String(value[0])}` : name;
  counts.set(key, Math.min(MAX_COUNT, (counts.get(key) ?? 0) + 1));
}

/** For the collector: everything counted since the last call, as a beacon's `events`. */
export function take(): Record<string, number> {
  const out = Object.fromEntries(counts);
  counts.clear();
  return out;
}

// ——— Wire shapes (crates/proof/src/rum.rs) ———

export type Metric = 'ttfb' | 'fcp' | 'lcp' | 'cls' | 'inp';

/** POST /_rum body. */
export interface RumBeacon {
  /** A page route (the page's canonical path), or "404". */
  route: string;
  /** First beacon of the page view: the only one with metrics. */
  view?: boolean;
  /** Milliseconds (CLS unitless). */
  metrics?: Partial<Record<Metric, number>>;
  events?: Record<string, number>;
}

/** GET /_rum. */
export interface RumSummary {
  /** Unix seconds: when this server process started counting. */
  since: number;
  now: number;
  /** Below this many samples `p75` is null. */
  min_n: number;
  start: RumWindow;
  /** The current hour and the 23 before it. */
  day: RumWindow;
  rejected: { invalid: number; rate_limited: number };
}

export interface RumWindow {
  from: number;
  views: number;
  routes: Record<string, number>;
  metrics: Partial<Record<Metric, { n: number; p75: number | null }>>;
  events: Record<string, number>;
}
