// The A/B test in the proof console's "Real visitors" row (docs/CONTRACTS.md §17): page views and
// conversions per variant from GET /_rum, how far each variant is from the planned sample size, and
// a verdict only once every variant has reached it. The server sends none before that, and none is
// shown before it even if one were sent (no peeking). Imported by visitors.ts while a test runs.

import { h, num, render } from '../shared/dom.ts';
import { numberLocale, pluralKey, t, tn } from '../shared/i18n.ts';
import type { RumExperiment } from '../shared/rum.ts';

const T = (key: string, vars?: Record<string, string | number>): string => t('hud-ab-' + key, vars);

export interface Row {
  id: string;
  control: boolean;
  /** Page views counted, never more than the plan. */
  views: number;
  conversions: number;
  /** Conversions per page view, 0–1. */
  rate: number;
  /** Per secondary metric, in the test's order. */
  secondary: number[];
  /** Against the control, once every variant has its planned page views. */
  verdict?: { p: number; significant: boolean; better: boolean };
}

/** The rows to show, and the false-positive rate of each comparison. */
export function rows(e: RumExperiment): { rows: Row[]; done: boolean; alpha: number } {
  const n = e.plan.sample_size;
  const done = !!e.result && e.variants.every((v) => v.views >= n);
  return {
    done,
    alpha: e.plan.alpha / Math.max(1, e.variants.length - 1),
    rows: e.variants.map((v, i) => {
      const views = Math.min(v.views, n);
      const c = done ? e.result?.find((r) => r.variant === v.id) : undefined;
      return {
        id: v.id,
        control: i === 0,
        views,
        conversions: v.conversions,
        rate: views ? Math.min(1, v.conversions / views) : 0,
        secondary: e.secondary.map((k) => v.secondary[k] ?? 0),
        ...(c && { verdict: { p: c.p, significant: c.significant, better: c.z > 0 } }),
      };
    }),
  };
}

export function show(el: HTMLElement, e: RumExperiment): void {
  // Digits in the page language's style; Tajik borrows Russian's, which browsers have data for.
  const L = numberLocale();
  const { rows: list, done, alpha } = rows(e);
  // Planned numbers are plain text; measured ones are `.num` (mono means measured).
  const plain = (x: number) => new Intl.NumberFormat(L).format(x);
  const count = (x: number) => num({ value: plain(x), unit: '' });
  const p = (x: number) => num({ value: x < 0.001 ? '< ' + plain(0.001) : new Intl.NumberFormat(L, { maximumSignificantDigits: 2 }).format(x), unit: '' });
  const max = e.plan.sample_size;
  // The variant this page view shows (`<html data-variant>`), if any.
  const mine = document.documentElement.dataset['variant'];
  const mono = (s: string) => h('span', { class: 'mono' }, s);
  const verdict = (v: NonNullable<Row['verdict']>) => tn(v.significant ? (v.better ? 'hud-ab-better' : 'hud-ab-worse') : 'hud-ab-same', { p: p(v.p) });
  render(
    el,
    h('span', { class: 'hud-sub' }, ...tn('hud-ab-lead', { id: mono(e.id), metric: mono(e.metric), max: plain(max) })),
    h(
      'ul',
      { class: 'hud-list', role: 'list' },
      list.map((r) =>
        h(
          'li',
          null,
          mono(r.id),
          (r.control || r.id === mine) && [' — ', T(r.control ? 'control' : 'yours'), r.control && r.id === mine && [', ', T('yours')]],
          h('br'),
          tn(pluralKey('hud-ab-views', max), { n: count(r.views), max: plain(max) }),
          ' · ',
          tn('hud-ab-rate', { n: count(r.conversions), metric: mono(e.metric), rate: num({ value: new Intl.NumberFormat(L, { style: 'percent', maximumFractionDigits: 1 }).format(r.rate), unit: '' }) }),
          r.secondary.map((x, i) => [' · ', count(x), ' ', mono(e.secondary[i]!)]),
          r.verdict && [h('br'), verdict(r.verdict)],
        ),
      ),
    ),
    h('span', { class: 'hud-sub' }, done ? T('done', { alpha: plain(alpha), family: plain(e.plan.alpha), k: list.length - 1 }) : T('wait')),
  );
}
