// SVG plots for the experiments, drawn in a fixed 300×190 box that CSS scales. Plots are pictures
// (aria-hidden); the same numbers are on the results card and in each "Your trials" table.

import { render, s } from '../../shared/dom.ts';
import { fmt } from './kit.ts';
import type { Fit, Pt } from './stats.ts';

const W = 300;
const H = 190;
const X0 = 44; // y axis
const X1 = 290;
const Y0 = 156; // x axis
const Y1 = 24; // top of the plot area

/** A tidy axis maximum ≥ v: 1, 2 or 5 × 10^k. */
export function nice(v: number): number {
  const e = 10 ** Math.floor(Math.log10(Math.max(v, 1e-9)));
  return [1, 2, 5, 10].map((m) => m * e).find((step) => step >= v - 1e-9) ?? 10 * e;
}

export const plotBox = (): SVGSVGElement =>
  s('svg', { class: 'laws-plot', viewBox: `0 0 ${W} ${H}`, 'aria-hidden': 'true', focusable: 'false' });

const scaleY = (yMax: number) => (y: number) => Y0 - (Math.min(Math.max(y, 0), yMax) / yMax) * (Y0 - Y1);

/** Axes, two grid lines and the y labels (unit on top). */
function axes(yMax: number, unit: string): SVGElement[] {
  const Y = scaleY(yMax);
  const out: SVGElement[] = [
    s('text', { x: X0 - 6, y: 12, 'text-anchor': 'end' }, unit),
    s('text', { x: X0 - 6, y: Y0 + 4, 'text-anchor': 'end' }, '0'),
  ];
  for (const f of [0.5, 1]) {
    out.push(
      s('path', { class: 'laws-grid', d: `M${X0} ${Y(yMax * f)}H${X1}` }),
      s('text', { x: X0 - 6, y: Y(yMax * f) + 4, 'text-anchor': 'end' }, fmt(yMax * f)),
    );
  }
  out.push(s('path', { class: 'laws-axis', d: `M${X0} ${Y1 - 8}V${Y0}H${X1}` }));
  return out;
}

export interface Scatter {
  pts: readonly Pt[];
  /** Larger square markers, e.g. a mean per condition. */
  means?: readonly Pt[];
  fit: Fit | null;
  xMax: number;
  x: string;
  y: string;
}

/** Points, optional means, and the least-squares line drawn over the range of the data only. */
export function scatter(svg: SVGSVGElement, p: Scatter): void {
  const yMax = nice(Math.max(100, ...p.pts.map((q) => q[1])));
  const X = (x: number) => X0 + (Math.min(x, p.xMax) / p.xMax) * (X1 - X0);
  const Y = scaleY(yMax);
  const kids = axes(yMax, p.y);
  for (let x = 0; x <= p.xMax; x++) kids.push(s('text', { x: X(x), y: Y0 + 16, 'text-anchor': 'middle' }, String(x)));
  kids.push(s('text', { x: (X0 + X1) / 2, y: H - 2, 'text-anchor': 'middle' }, p.x));
  if (p.fit && p.pts.length > 2) {
    const xs = p.pts.map((q) => q[0]);
    const lo = Math.min(...xs);
    const hi = Math.max(...xs);
    const { a, b } = p.fit;
    kids.push(s('line', { class: 'laws-fit', x1: X(lo), y1: Y(a + b * lo), x2: X(hi), y2: Y(a + b * hi) }));
  }
  for (const [x, y] of p.pts) kids.push(s('circle', { class: 'laws-pt', cx: X(x), cy: Y(y), r: 3 }));
  for (const [x, y] of p.means ?? []) kids.push(s('rect', { class: 'laws-mean', x: X(x) - 4.5, y: Y(y) - 4.5, width: 9, height: 9 }));
  render(svg, kids);
}

export interface Bar {
  label: string;
  value: number | null;
  /** A marker drawn above the label (shape and colour of the condition, never colour alone). */
  mark: (x: number, y: number) => SVGElement;
}

export function bars(svg: SVGSVGElement, rows: readonly Bar[], unit: string): void {
  const yMax = nice(Math.max(100, ...rows.map((r) => r.value ?? 0)));
  const Y = scaleY(yMax);
  const kids = axes(yMax, unit);
  const slot = (X1 - X0) / rows.length;
  rows.forEach((r, i) => {
    const cx = X0 + slot * (i + 0.5);
    if (r.value !== null) {
      const top = Y(r.value);
      kids.push(
        s('rect', { class: 'laws-bar', x: cx - slot * 0.3, y: top, width: slot * 0.6, height: Math.max(0, Y0 - top) }),
        s('text', { class: 'laws-v', x: cx, y: top - 6, 'text-anchor': 'middle' }, fmt(r.value)),
      );
    }
    kids.push(r.mark(cx, Y0 + 13), s('text', { x: cx, y: H - 2, 'text-anchor': 'middle' }, r.label));
  });
  render(svg, kids);
}
