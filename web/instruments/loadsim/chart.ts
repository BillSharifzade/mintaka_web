// History and SVG path arithmetic for the overload lab's charts. Pure — tested in chart.test.ts.
// (The same idea as live.ts's sparklines, copied rather than imported across islands.)

/** Windows kept per series: 60 s of 250 ms windows. */
export const HIST = 240;

/** The last HIST values of one series, oldest first; NaN marks "nothing to show". */
export class Series {
  readonly v = new Float64Array(HIST).fill(Number.NaN);

  push(x: number): void {
    this.v.copyWithin(0, 1);
    this.v[HIST - 1] = x;
  }

  clear(): void {
    this.v.fill(Number.NaN);
  }

  max(): number {
    let m = 0;
    for (const x of this.v) if (x > m) m = x;
    return m;
  }
}

/** A tidy scale maximum ≥ v: 1, 2 or 5 × 10^k (and at least `floor`). */
export function nice(v: number, floor = 1): number {
  const x = Math.max(v, floor);
  const e = 10 ** Math.floor(Math.log10(x));
  return [1, 2, 5, 10].map((m) => m * e).find((step) => step >= x - 1e-9) ?? 10 * e;
}

/** Maps a value to 0 (bottom) … 1 (top). */
export type Scale = (v: number) => number;
const clamp01 = (x: number) => Math.min(1, Math.max(0, x));
export const linear = (max: number): Scale => (v) => clamp01(v / max);
export const log = (lo: number, hi: number): Scale => (v) => clamp01(Math.log(v / lo) / Math.log(hi / lo));

const xy = (i: number, v: number, w: number, h: number, y: Scale) =>
  `${+((i * w) / (HIST - 1)).toFixed(1)} ${+(h - 1 - y(v) * (h - 2)).toFixed(1)}`;

/** A line through the series; NaN breaks it. */
export function line(v: ArrayLike<number>, w: number, h: number, y: Scale): string {
  let d = '';
  let pen = false;
  for (let i = 0; i < v.length; i++) {
    const x = v[i]!;
    if (Number.isNaN(x)) {
      pen = false;
      continue;
    }
    d += (pen ? 'L' : 'M') + xy(i, x, w, h, y);
    pen = true;
  }
  return d;
}

/** The area under the series down to the baseline, one closed shape per unbroken run. */
export function area(v: ArrayLike<number>, w: number, h: number, y: Scale): string {
  let d = '';
  let start = -1;
  const shut = (end: number) => {
    if (start < 0) return;
    d += `L${xy(end, 0, w, h, () => 0)}L${xy(start, 0, w, h, () => 0)}Z`;
    start = -1;
  };
  for (let i = 0; i < v.length; i++) {
    const x = v[i]!;
    if (Number.isNaN(x)) {
      shut(i - 1);
      continue;
    }
    d += (start < 0 ? 'M' : 'L') + xy(i, x, w, h, y);
    if (start < 0) start = i;
  }
  shut(v.length - 1);
  return d;
}

/** "×20 — 400" for a load multiplier and the base rate. */
export const loadRate = (x: number, base: number): number => Math.round(x * base);
