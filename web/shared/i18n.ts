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
    return typeof v === 'number' ? new Intl.NumberFormat(lang()).format(v) : v;
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
      return typeof v === 'number' ? new Intl.NumberFormat(lang()).format(v) : v;
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

/** The message id tp() picks for `n`; with tn() a count can keep its markup: `tn(pluralKey(k, n), { n: num(…) })`. */
export function pluralKey(key: string, n: number): string {
  const d = load();
  const category = new Intl.PluralRules(lang()).select(n);
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
