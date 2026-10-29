// The proof console's "Real visitors" row: what visitors' browsers measured on this server, from
// GET /_rum (docs/CONTRACTS.md §14). Imported when the panel opens, like headers.ts, and asked
// again on every opening. Too few samples for a percentile are said in words, never as a number.
// Imports only shared modules (never ./index.ts), so the console stays a single chunk.

import { h, num, render } from '../shared/dom.ts';
import { duration } from '../shared/format.ts';
import { offline } from '../shared/env.ts';
import { lang, pluralKey, t, tn } from '../shared/i18n.ts';
import { declined, type Metric, type RumSummary, type RumWindow } from '../shared/rum.ts';
import { clsText } from './metrics.ts';

const T = (key: string, vars?: Record<string, string | number>): string => t('hud-visitors-' + key, vars);
const ORDER: Metric[] = ['lcp', 'inp', 'cls', 'fcp', 'ttfb'];

export interface Line {
  metric: Metric;
  n: number;
  /** null: fewer than `min` samples, so no percentile is shown. */
  p75: number | null;
}

/** The metrics in display order (Core Web Vitals first), with the server's sample rule enforced again. */
export function lines(w: RumWindow, min: number): Line[] {
  return ORDER.map((metric) => {
    const m = w.metrics[metric] ?? { n: 0, p75: null };
    return { metric, n: m.n, p75: m.n >= min ? m.p75 : null };
  });
}

export async function show(cell: HTMLElement): Promise<void> {
  const L = lang();
  const count = (n: number) => num({ value: new Intl.NumberFormat(L).format(n), unit: '' });
  let s: RumSummary;
  try {
    const res = await fetch('/_rum', { cache: 'no-store', headers: { accept: 'application/json' } });
    if (!res.ok) throw new Error(`/_rum answered ${res.status}`);
    s = (await res.json()) as RumSummary;
  } catch (err) {
    render(cell, offline() ? t('hud-offline') : T('error', { reason: err instanceof Error ? err.message : String(err) }));
    return;
  }
  const date = new Intl.DateTimeFormat(L, { dateStyle: 'medium', timeStyle: 'short' }).format(new Date(s.since * 1000));
  const w = s.start;
  const value = (l: Line) =>
    l.p75 === null
      ? h('span', null, ...tn(pluralKey('hud-visitors-few', l.n), { n: count(l.n), min: count(s.min_n) }))
      : [num(l.metric === 'cls' ? { value: clsText(l.p75, L), unit: '' } : duration(l.p75, L)), ' ', h('span', null, ...tn('hud-visitors-p75', { n: count(l.n) }))];
  render(
    cell,
    h('span', { class: 'hud-sub' }, ...tn(pluralKey('hud-visitors-since', w.views), { date, n: count(w.views) })),
    w.views > 0 && h('ul', { class: 'hud-list', role: 'list' }, lines(w, s.min_n).map((l) => h('li', null, h('span', { class: 'mono' }, l.metric.toUpperCase()), ' ', value(l)))),
    declined() && h('span', { class: 'hud-sub' }, T('declined')),
  );
}
