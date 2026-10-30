// Island strings. The site generator exports every message from `content/{lang}/islands/*.ftl`
// into `<script type="application/json" id="mintaka-i18n">` on each page. Fluent variables
// (`{ $ms }`) arrive as `{ms}` and are filled here. Island messages use plain variables only —
// no selectors — so this stays a lookup. Bundles in `islands/lazy/` are fetched with loadStrings().

import { assetUrl } from './assets.ts';

let dict: Record<string, string> | null = null;
const bundles = new Map<string, Promise<boolean>>();

function load(): Record<string, string> {
  if (dict) return dict;
  const el = typeof document !== 'undefined' ? document.getElementById('mintaka-i18n') : null;
  try {
    dict = el?.textContent ? (JSON.parse(el.textContent) as Record<string, string>) : {};
  } catch {
    dict = {};
  }
  return dict;
}

/** The page language (`<html lang>`), e.g. `en`, `ru`, `tg`. */
export function lang(): string {
  return (typeof document !== 'undefined' && document.documentElement.lang) || 'en';
}

// Languages whose numbers are written exactly like another's (CLDR: grouping by a no-break space,
// a decimal comma). Chrome ships no locale data for Tajik, so `Intl` would fall back to the
// browser's own language there (1,234.5, "Oct 30").
const SAME_NUMBERS: Record<string, string> = { tg: 'ru' };
const known = new Map<string, boolean>();
const supported = (l: string): boolean => {
  let ok = known.get(l);
  if (ok === undefined) known.set(l, (ok = Intl.NumberFormat.supportedLocalesOf(l).length > 0));
  return ok;
};

/**
 * The locale to format numbers in: the page language, or where this browser has no data for it,
 * a language that writes numbers the same way (Tajik → Russian rules: 1 234,5). Only digits and
 * separators are borrowed — never words, units or plural rules (those stay `lang()`'s).
 */
export function numberLocale(l: string = lang()): string {
  const base = l.split('-')[0] ?? l;
  const twin = SAME_NUMBERS[base];
  return twin && !supported(l) ? twin : l;
}

/** A date and time in the page language: "30 окт. 2026 г., 12:40"; in figures where the browser has no words for the language. */
export function dateTime(d: Date, l: string = lang()): string {
  if (supported(l)) return new Intl.DateTimeFormat(l, { dateStyle: 'medium', timeStyle: 'short' }).format(d);
  const numeric = { day: '2-digit', month: '2-digit', year: 'numeric', hour: '2-digit', minute: '2-digit' } as const;
  return new Intl.DateTimeFormat(numberLocale(l), numeric).format(d);
}

/** "a, b and c" in the page language; plain commas where the browser has no data for it. */
export function list(items: string[], l: string = lang()): string {
  return supported(l) ? new Intl.ListFormat(l, { type: 'conjunction' }).format(items) : items.join(', ');
}

/** Look up a message; `{name}` placeholders are replaced from `vars` (numbers are localised). */
export function t(key: string, vars?: Record<string, string | number>): string {
  const pattern = load()[key];
  if (pattern === undefined) {
    console.warn(`[mintaka] missing string "${key}"`);
    return key;
  }
  if (!vars) return pattern;
  return pattern.replace(/\{([a-z0-9_-]+)\}/gi, (whole, name: string) => {
    const v = vars[name];
    if (v === undefined) return whole;
    return typeof v === 'number' ? new Intl.NumberFormat(numberLocale()).format(v) : v;
  });
}

/**
 * Like t(), but a placeholder may be filled with a DOM node, so measured values keep their markup
 * inside a translated sentence: `h('p', null, ...tn('physics-took', { ms: num(duration(53)) }))`.
 */
export function tn(key: string, vars: Record<string, Node | string | number>): (Node | string)[] {
  return t(key)
    .split(/\{([a-z0-9_-]+)\}/gi)
    .map((part, i) => {
      if (i % 2 === 0) return part;
      const v = vars[part];
      if (v === undefined) return `{${part}}`;
      return typeof v === 'number' ? new Intl.NumberFormat(numberLocale()).format(v) : v;
    });
}

/**
 * A message that depends on a count: `tp('hud-requests', 3)` picks `hud-requests[3]`, then
 * `hud-requests[few]` (or whichever plural category the page language uses for 3), then the
 * default. `n` is also available to the pattern as `{n}`. Write the FTL as
 * `hud-requests = { $n -> [one] { $n } request *[other] { $n } requests }`.
 */
export function tp(key: string, n: number, vars: Record<string, string | number> = {}): string {
  return t(pluralKey(key, n), { n, ...vars });
}

// Chrome and Node ship no plural rules for Tajik, and Intl.PluralRules("tg") would quietly take
// the browser's own language instead (Russian on many machines in Tajikistan: few, many). Use the
// source language's one/other there, which Tajik messages are written for — a Tajik noun stays
// singular after a numeral, so most need no variant at all.
const pluralLocale = (l: string): string => (Intl.PluralRules.supportedLocalesOf(l).length > 0 ? l : 'en');

/** The message id tp() picks for `n`; with tn() a count can keep its markup: `tn(pluralKey(k, n), { n: num(…) })`. */
export function pluralKey(key: string, n: number): string {
  const d = load();
  const category = new Intl.PluralRules(pluralLocale(lang())).select(n);
  return [`${key}[${n}]`, `${key}[${category}]`].find((k) => k in d) ?? key;
}

/**
 * Fetch a lazy bundle (`content/{lang}/islands/lazy/<name>.ftl`) into the dictionary. Resolves
 * `false` if it couldn't be fetched (offline, say) — then its keys stay missing, so keep the line
 * that reports the failure inline — and tries again on the next call.
 */
export function loadStrings(name: string): Promise<boolean> {
  let job = bundles.get(name);
  if (!job) {
    job = fetch(assetUrl(`strings/${name}.${lang()}`))
      .then((res) => (res.ok ? (res.json() as Promise<Record<string, string>>) : Promise.reject(new Error(String(res.status)))))
      .then((strings) => {
        Object.assign(load(), strings);
        return true;
      })
      .catch(() => {
        bundles.delete(name);
        return false;
      });
    bundles.set(name, job);
  }
  return job;
}

/** For tests: install a dictionary without a DOM. */
export function __setStrings(strings: Record<string, string>): void {
  dict = strings;
}
