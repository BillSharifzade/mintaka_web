// Number formatting for measured values. Units follow Chrome DevTools: 1 kB = 1000 bytes.
// Pure functions — safe to import from Node tests. `locale` defaults to the page language.

import { lang, numberLocale } from './i18n.ts';

const nf = (digits: number, locale?: string) =>
  new Intl.NumberFormat(numberLocale(locale), { maximumFractionDigits: digits, minimumFractionDigits: 0 });

// Unit symbols as each language writes them: SI symbols in Cyrillic for Russian (CLDR's) and
// Tajik (the same). Keep in step with unit_symbol() in crates/site/src/i18n.rs.
const CYRILLIC: Record<string, string> = { 'µs': 'мкс', ms: 'мс', s: 'с', B: 'Б', kB: 'кБ', MB: 'МБ', km: 'км' };
const UNITS: Record<string, Record<string, string>> = { ru: CYRILLIC, tg: CYRILLIC };

/** A unit symbol in the page language: `unit('ms')` is "ms" in English, "мс" in Russian and Tajik. */
export function unit(symbol: string, locale: string = lang()): string {
  return UNITS[locale.split('-')[0] ?? '']?.[symbol] ?? symbol;
}

export interface Formatted {
  value: string;
  unit: string;
}

/** A duration given in milliseconds, as µs below 1 ms, ms below 10 s, otherwise s. */
export function duration(ms: number, locale?: string): Formatted {
  if (!Number.isFinite(ms) || ms < 0) return { value: '—', unit: '' };
  if (ms < 1) {
    const us = ms * 1000;
    return { value: nf(us < 10 ? 1 : 0, locale).format(us), unit: unit('µs', locale) };
  }
  if (ms < 10_000) return { value: nf(ms < 10 ? 1 : 0, locale).format(ms), unit: unit('ms', locale) };
  return { value: nf(1, locale).format(ms / 1000), unit: unit('s', locale) };
}

/** A byte count as B, kB or MB (base 1000). */
export function bytes(n: number, locale?: string): Formatted {
  if (!Number.isFinite(n) || n < 0) return { value: '—', unit: '' };
  if (n < 1000) return { value: nf(0, locale).format(n), unit: unit('B', locale) };
  if (n < 1_000_000) return { value: nf(n < 10_000 ? 1 : 0, locale).format(n / 1000), unit: unit('kB', locale) };
  return { value: nf(1, locale).format(n / 1_000_000), unit: unit('MB', locale) };
}

/** A distance in kilometres. */
export function km(n: number, locale?: string): Formatted {
  if (!Number.isFinite(n) || n < 0) return { value: '—', unit: '' };
  return { value: nf(n < 10 ? 1 : 0, locale).format(n), unit: unit('km', locale) };
}

export const join = (f: Formatted): string => (f.unit ? `${f.value} ${f.unit}` : f.value);
