// Pieces the three experiments share: strings, number formatting, timing and the answer slot.

import { h } from '../../shared/dom.ts';
import { lang, t, tn, tp } from '../../shared/i18n.ts';

export const L = (key: string, vars?: Record<string, string | number>): string => t(`laws-${key}`, vars);

/** A count: `P('misses', 3)` picks the plural variant of `laws-misses` for 3. */
export const P = (key: string, n: number): string => tp(`laws-${key}`, n);

/** One number on the results card: label, formatted value, unit. */
export type Item = readonly [label: string, value: string, unit: string];

export interface Report {
  items: Item[];
  /** One sentence in plain language with the visitor's own numbers. */
  plain?: (Node | string)[] | undefined;
  line: string;
  note?: string | undefined;
}

export interface Api {
  /** A run began (counted for /_rum; the measurements themselves never leave the browser). */
  started(): void;
  /** A finished run: show the visitor's numbers on the results card. */
  report(r: Report): void;
  /** Move to the next experiment (the Fitts test's way out for keyboard users). */
  next(): void;
}

/** An experiment builds its panel and returns `stop(reason?)`, which aborts a run in progress. */
export type Experiment = (panel: HTMLElement, api: Api) => (reason?: string) => void;

/** `v` with exactly `digits` decimals, in the page language (never "-0"). */
export const fmt = (v: number, digits = 0): string =>
  new Intl.NumberFormat(lang(), { minimumFractionDigits: digits, maximumFractionDigits: digits }).format(
    Math.round(v * 10 ** digits) / 10 ** digits || 0,
  );

/** A measured value: <span class="num">412<span class="num__unit">ms</span></span>. */
export const num = (value: string, unit = ''): HTMLElement =>
  h('span', { class: 'num' }, value, unit && h('span', { class: 'num__unit' }, unit));

/** A message whose `{name}` placeholders become nodes (measured values in .num). */
export const fill = (key: string, parts: Record<string, Node>): (Node | string)[] => tn(`laws-${key}`, parts);

/** The timestamp of the next animation frame — the frame that paints what was just changed. */
export const frame = (): Promise<number> => new Promise((resolve) => requestAnimationFrame(resolve));

export const sleep = (ms: number): Promise<void> => new Promise((resolve) => setTimeout(resolve, ms));

/** A random foreperiod, so the next stimulus can't be anticipated. */
export const foreperiod = (): Promise<void> => sleep(450 + Math.random() * 500);

/** A one-shot answer: `wait()` resolves with the first `give()` while it is open. */
export function slot<T>() {
  let take: ((v: T) => void) | null = null;
  return {
    wait: () => new Promise<T>((resolve) => (take = resolve)),
    give(v: T): void {
      const f = take;
      take = null;
      f?.(v);
    },
    close(): void {
      take = null;
    },
    get open(): boolean {
      return take !== null;
    },
  };
}

/** A mean that tolerates empty lists (null). */
export const avg = (xs: readonly number[]): number | null => (xs.length ? xs.reduce((a, b) => a + b, 0) / xs.length : null);

/** "Your trials" — the data behind a plot, as a table anyone can read. */
export function trials(cols: readonly string[], rows: readonly (readonly string[])[]): HTMLElement {
  return h(
    'details',
    { class: 'laws-data' },
    h('summary', null, L('data')),
    h(
      'table',
      null,
      h('thead', null, h('tr', null, cols.map((c) => h('th', { scope: 'col' }, c)))),
      h('tbody', null, rows.map((r) => h('tr', null, r.map((c) => h('td', null, c))))),
    ),
  );
}
