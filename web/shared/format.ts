// Number formatting for measured values. Units follow Chrome DevTools: 1 kB = 1000 bytes.
// Pure functions — safe to import from Node tests.

const nf = (digits: number, locale?: string) =>
  new Intl.NumberFormat(locale, { maximumFractionDigits: digits, minimumFractionDigits: 0 });

export interface Formatted {
  value: string;
  unit: string;
}

/** A duration given in milliseconds, as µs below 1 ms, ms below 10 s, otherwise s. */
export function duration(ms: number, locale?: string): Formatted {
  if (!Number.isFinite(ms) || ms < 0) return { value: '—', unit: '' };
  if (ms < 1) {
    const us = ms * 1000;
    return { value: nf(us < 10 ? 1 : 0, locale).format(us), unit: 'µs' };
  }
  if (ms < 10_000) return { value: nf(ms < 10 ? 1 : 0, locale).format(ms), unit: 'ms' };
  return { value: nf(1, locale).format(ms / 1000), unit: 's' };
}

/** A byte count as B, kB or MB (base 1000). */
export function bytes(n: number, locale?: string): Formatted {
  if (!Number.isFinite(n) || n < 0) return { value: '—', unit: '' };
  if (n < 1000) return { value: nf(0, locale).format(n), unit: 'B' };
  if (n < 1_000_000) return { value: nf(n < 10_000 ? 1 : 0, locale).format(n / 1000), unit: 'kB' };
  return { value: nf(1, locale).format(n / 1_000_000), unit: 'MB' };
}

/** A distance in kilometres. */
export function km(n: number, locale?: string): Formatted {
  if (!Number.isFinite(n) || n < 0) return { value: '—', unit: '' };
  return { value: nf(n < 10 ? 1 : 0, locale).format(n), unit: 'km' };
}

export const join = (f: Formatted): string => (f.unit ? `${f.value} ${f.unit}` : f.value);
