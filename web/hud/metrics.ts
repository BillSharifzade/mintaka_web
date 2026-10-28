// Pure helpers for the proof console (web/hud/index.ts). No DOM, so Node tests can import them.

import type { Proof } from '../shared/proof.ts';

export interface Shift {
  value: number;
  startTime: number;
  hadRecentInput: boolean;
}

/**
 * Cumulative Layout Shift as browsers define it: the largest session window of shifts, where a
 * window ends after a 1 s gap or at 5 s long. Shifts within 500 ms of input don't count.
 */
export function cls(shifts: readonly Shift[]): number {
  let max = 0;
  let sum = 0;
  let first = -Infinity;
  let last = -Infinity;
  for (const s of shifts) {
    if (s.hadRecentInput) continue;
    if (s.startTime - last < 1000 && s.startTime - first < 5000) sum += s.value;
    else {
      sum = s.value;
      first = s.startTime;
    }
    last = s.startTime;
    max = Math.max(max, sum);
  }
  return max;
}

/** The slowest interaction so far: the longest Event Timing entry that belongs to an interaction. */
export function slowest(entries: readonly { interactionId?: number; duration: number }[], before = 0): number {
  let worst = before;
  for (const e of entries) if ((e.interactionId ?? 0) > 0 && e.duration > worst) worst = e.duration;
  return worst;
}

/** CLS is unitless: "0", "0.004", "0.12". */
export function clsText(v: number, locale?: string): string {
  const digits = v === 0 ? 0 : v < 0.01 ? 3 : 2;
  return new Intl.NumberFormat(locale, { maximumFractionDigits: digits, minimumFractionDigits: digits }).format(v);
}

/** Cookies visible to scripts: `a=1; b=2` → 2. */
export const cookieCount = (jar: string): number => (jar.trim() ? jar.split(';').length : 0);

export interface Cookies {
  /** Cookies to show: those scripts can see, or those this site set, whichever is more. */
  count: number;
  /** true: this site set cookies · false: none of them are ours · undefined: can't tell without /_proof. */
  ours: boolean | undefined;
}

/**
 * Cookies are per host, not per port: on localhost the page also sees other apps' cookies. They
 * count as ours only when /_proof says the server set some (`serverSet`, undefined if unknown).
 */
export function cookies(jar: string, serverSet: number | undefined): Cookies {
  const visible = cookieCount(jar);
  if (serverSet === undefined) return { count: visible, ours: visible ? undefined : false };
  return serverSet > 0 ? { count: Math.max(visible, serverSet), ours: true } : { count: visible, ours: false };
}

/** "TLSv1.3" → "TLS 1.3". */
export const tlsVersion = (v: string): string => v.replace(/^TLSv?\s*/i, 'TLS ');

/** "HTTP/2.0" → "HTTP/2"; "HTTP/1.1" stays. */
export const httpVersion = (v: string): string => v.replace(/^(HTTP\/\d)\.0$/i, '$1');

/** What the pill says about the transport: `pq`, `dev` (plain HTTP), a TLS version, or null (unknown). */
export function transport(p: Proof | null | undefined): string | null {
  if (!p) return null;
  if (!p.tls) return 'dev';
  return p.tls.post_quantum ? 'pq' : tlsVersion(p.tls.version);
}
