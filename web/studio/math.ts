/** Small numeric helpers for the studio page's motion. Curves follow the page's CSS tokens. */

export const clamp = (v: number, lo = 0, hi = 1): number => (v < lo ? lo : v > hi ? hi : v);
export const lerp = (a: number, b: number, t: number): number => a + (b - a) * t;
/** Where `t` sits between `a` and `b`, clamped to 0..1. */
export const span = (t: number, a: number, b: number): number => clamp((t - a) / (b - a));

export const easeOut = (t: number): number => 1 - (1 - t) ** 3;
export const easeIn = (t: number): number => t * t * t;
export const easeInOut = (t: number): number => (t < 0.5 ? 4 * t * t * t : 1 - (-2 * t + 2) ** 3 / 2);
/** A small overshoot for things that land (the mark's dots). */
export const easeOutBack = (t: number): number => 1 + 2.2 * (t - 1) ** 3 + 1.2 * (t - 1) ** 2;
export const easeOutExpo = (t: number): number => (t >= 1 ? 1 : 1 - 2 ** (-10 * t));

/** A locale the browser can format numbers in: Tajik writes numbers as Russian does. */
export function numberLocale(lang: string): string {
  if (Intl.NumberFormat.supportedLocalesOf(lang).length > 0) return lang;
  return lang === 'tg' ? 'ru' : 'en';
}
