// The arithmetic of "Laws you can feel": least squares and the two laws' information measures.
// Pure functions, computed on the visitor's device — tested in stats.test.ts.

export type Pt = readonly [x: number, y: number];

export interface Fit {
  /** Intercept, in the units of y. */
  a: number;
  /** Slope: units of y per unit of x. */
  b: number;
  /** Coefficient of determination, 0…1. */
  r2: number;
  n: number;
}

export const mean = (xs: readonly number[]): number => xs.reduce((sum, x) => sum + x, 0) / xs.length;

/** Ordinary least squares, y = a + b·x. Null below two points or when every x is the same. */
export function fit(pts: readonly Pt[]): Fit | null {
  const n = pts.length;
  if (n < 2) return null;
  const mx = mean(pts.map((p) => p[0]));
  const my = mean(pts.map((p) => p[1]));
  let sxx = 0;
  let sxy = 0;
  let syy = 0;
  for (const [x, y] of pts) {
    sxx += (x - mx) * (x - mx);
    sxy += (x - mx) * (y - my);
    syy += (y - my) * (y - my);
  }
  if (sxx === 0) return null;
  const b = sxy / sxx;
  // For one predictor R² = r² = Sxy² / (Sxx·Syy); a flat, exact line explains everything.
  return { a: my - b * mx, b, r2: syy === 0 ? 1 : (sxy * sxy) / (sxx * syy), n };
}

/** Fitts's index of difficulty, Shannon form: ID = log2(D/W + 1) bits. */
export const fittsId = (distance: number, width: number): number => Math.log2(distance / width + 1);

/** Throughput in bits/s: the mean of ID/MT over trials given as [ID bits, MT ms]. */
export const throughput = (trials: readonly Pt[]): number => mean(trials.map(([id, ms]) => id / (ms / 1000)));

/** Hick–Hyman: information in a choice among n equally likely options, log2(n + 1) bits. */
export const hickBits = (n: number): number => Math.log2(n + 1);

/** Fisher–Yates, in place. */
export function shuffle<T>(xs: T[], random: () => number = Math.random): T[] {
  for (let i = xs.length - 1; i > 0; i--) {
    const j = Math.floor(random() * (i + 1));
    [xs[i], xs[j]] = [xs[j] as T, xs[i] as T];
  }
  return xs;
}
