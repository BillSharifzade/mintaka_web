// The coverage map, drawn as Orion. Each of the parser's coverage points is a star placed along
// the constellation's lines, roughly in the order the fuzzer reaches them: the magic bytes at the
// head (Meissa), the header at the shoulders, star records down the body, sections at the belt,
// and the name section running down to Saiph — "the sword", where the bug lives.

import { s } from '../../shared/dom.ts';

export const SKY = { width: 320, height: 440 } as const;

/** `COUNT` in crates/fuzzer/src/points.rs; the module reports it too and the tests compare. */
export const POINTS = 39;

/** Coverage point ids the island refers to by name. */
export const P = { nameLen: 35, nameOk: 36 } as const;

type NodeId = number | 'lead' | 'saiph';

export interface SkyPoint {
  id: number;
  x: number;
  y: number;
  /** Set for the points that light a named star of Orion. */
  star?: string;
}

export interface SkyLink {
  a: NodeId;
  b: NodeId;
  x1: number;
  y1: number;
  x2: number;
  y2: number;
}

interface Star {
  at: [number, number];
  point: NodeId;
  label: 'left' | 'right' | 'below';
}

const T = (x: number, y: number): [number, number] => [(x - 30) * 1.3 + 74, (y - 30) * 1.3 + 56];

/** Named stars, and the coverage point that lights each one. Saiph is the crash site. */
export const STARS = {
  Meissa: { at: T(99, 30), point: 2, label: 'right' },
  Betelgeuse: { at: T(30, 65), point: 27, label: 'left' },
  Bellatrix: { at: T(134, 80), point: 8, label: 'right' },
  Mintaka: { at: T(114, 170), point: 28, label: 'right' },
  Alnilam: { at: T(96, 183), point: 12, label: 'below' },
  Alnitak: { at: T(78, 195), point: 13, label: 'left' },
  Rigel: { at: T(170, 281), point: 32, label: 'right' },
  Saiph: { at: T(54, 302), point: 'saiph', label: 'left' },
} satisfies Record<string, Star>;

const LEAD: [number, number] = [STARS.Meissa.at[0], 10];

/** Constellation segments, and the minor points spread along each. */
const SEGMENTS: ReadonlyArray<{ from: NodeId; to: NodeId; points: number[] }> = [
  { from: 'lead', to: 2, points: [0, 1] },
  { from: 2, to: 8, points: [3, 4, 5, 6, 7] },
  { from: 8, to: 28, points: [9, 10, 14, 15, 16, 17] },
  { from: 28, to: 12, points: [11] },
  { from: 12, to: 13, points: [] },
  { from: 2, to: 27, points: [22, 23, 24, 25, 26] },
  { from: 27, to: 13, points: [18, 19, 20, 21, 29, 30] },
  { from: 28, to: 32, points: [31, 33] },
  { from: 13, to: 'saiph', points: [34, 35, 36, 37] },
];

/** The one point off the lines: a chart named MINTAKA sits alone, near Mintaka. */
const LONE: Record<number, [number, number]> = { 38: [226, 250] };

export function layout(): { points: SkyPoint[]; links: SkyLink[] } {
  const where = new Map<NodeId, [number, number]>([['lead', LEAD]]);
  const starOf = new Map<NodeId, string>();
  for (const [name, star] of Object.entries(STARS)) {
    where.set(star.point, star.at);
    starOf.set(star.point, name);
  }
  for (const seg of SEGMENTS) {
    const [x1, y1] = where.get(seg.from)!;
    const [x2, y2] = where.get(seg.to)!;
    seg.points.forEach((p, k) => {
      const f = (k + 1) / (seg.points.length + 1);
      where.set(p, [x1 + (x2 - x1) * f, y1 + (y2 - y1) * f]);
    });
  }
  for (const [id, at] of Object.entries(LONE)) where.set(Number(id), at);

  const points: SkyPoint[] = [];
  for (let id = 0; id < POINTS; id++) {
    const at = where.get(id);
    if (!at) throw new Error(`coverage point ${id} has no place in the sky`);
    const star = starOf.get(id);
    points.push(star ? { id, x: at[0], y: at[1], star } : { id, x: at[0], y: at[1] });
  }

  const links: SkyLink[] = [];
  for (const seg of SEGMENTS) {
    const nodes: NodeId[] = [seg.from, ...seg.points, seg.to];
    for (let i = 0; i + 1 < nodes.length; i++) {
      const a = nodes[i]!;
      const b = nodes[i + 1]!;
      const [x1, y1] = where.get(a)!;
      const [x2, y2] = where.get(b)!;
      links.push({ a, b, x1, y1, x2, y2 });
    }
  }
  return { points, links };
}

function add<K extends keyof SVGElementTagNameMap>(parent: Element, tag: K, attrs: Record<string, string | number>): SVGElementTagNameMap[K] {
  const node = s(tag, attrs);
  parent.append(node);
  return node;
}

/** The live sky. Pure DOM, so it works under Trusted Types. */
export class Sky {
  readonly svg: SVGSVGElement;
  private readonly dots = new Map<number, SVGCircleElement>();
  private readonly labels = new Map<NodeId, SVGTextElement>();
  private readonly lines: Array<{ link: SkyLink; node: SVGLineElement }> = [];
  private readonly saiph: SVGCircleElement;
  private readonly pulses: SVGGElement;
  private crashLine: SVGLineElement | null = null;
  private readonly lit = new Set<NodeId>(['lead']);
  private readonly motion: boolean;

  constructor(parent: HTMLElement, starNames: Record<string, string>, motion: boolean) {
    this.motion = motion;
    const { points, links } = layout();
    this.svg = s('svg', { viewBox: `0 0 ${SKY.width} ${SKY.height}`, class: 'fz-sky', 'aria-hidden': 'true', focusable: 'false' });
    const guides = add(this.svg, 'g', { class: 'fz-sky__guides' });
    const litLines = add(this.svg, 'g', { class: 'fz-sky__links' });
    this.pulses = add(this.svg, 'g', { class: 'fz-sky__pulses' });
    const dots = add(this.svg, 'g', { class: 'fz-sky__dots' });
    const names = add(this.svg, 'g', { class: 'fz-sky__names' });

    for (const link of links) {
      const xy = { x1: link.x1, y1: link.y1, x2: link.x2, y2: link.y2 };
      add(guides, 'line', xy);
      this.lines.push({ link, node: add(litLines, 'line', { ...xy, class: 'fz-sky__link' }) });
    }
    for (const p of points) {
      const cls = p.star ? `fz-sky__dot fz-sky__dot--star fz-sky__dot--${p.star.toLowerCase()}` : 'fz-sky__dot';
      this.dots.set(p.id, add(dots, 'circle', { cx: p.x, cy: p.y, r: p.star ? 5.5 : 2.6, class: cls }));
    }
    const [sx, sy] = STARS.Saiph.at;
    this.saiph = add(dots, 'circle', { cx: sx, cy: sy, r: 5.5, class: 'fz-sky__dot fz-sky__dot--star fz-sky__dot--saiph' });

    for (const [name, star] of Object.entries(STARS)) {
      const [x, y] = star.at;
      const place =
        star.label === 'left' ? { x: x - 10, y: y + 3.5, a: 'end' } :
        star.label === 'below' ? { x: x + 4, y: y + 17, a: 'start' } :
        { x: x + 10, y: y + 3.5, a: 'start' };
      const text = add(names, 'text', { x: place.x, y: place.y, 'text-anchor': place.a, class: 'fz-sky__name' });
      text.textContent = starNames[name] ?? name;
      this.labels.set(star.point, text);
    }
    parent.append(this.svg);
  }

  reset(): void {
    this.lit.clear();
    this.lit.add('lead');
    for (const d of this.dots.values()) d.classList.remove('is-lit', 'is-you');
    this.saiph.classList.remove('is-lit', 'is-crash', 'is-you');
    for (const { node } of this.lines) node.classList.remove('is-lit');
    for (const label of this.labels.values()) label.classList.remove('is-lit', 'is-crash');
    this.pulses.replaceChildren();
    this.crashLine?.remove();
    this.crashLine = null;
  }

  /** Lights newly reached points. */
  light(points: number[], pulse = true): void {
    for (const id of points) {
      if (this.lit.has(id)) continue;
      this.lit.add(id);
      this.dots.get(id)?.classList.add('is-lit');
      this.labels.get(id)?.classList.add('is-lit');
      if (pulse) this.pulse(id);
    }
    // The name was read safely: the sword holds.
    if (this.lit.has(P.nameOk)) this.setSaiph('is-lit');
    this.refreshLinks();
  }

  /** The crash: Saiph flares, and a red line runs from the last point reached (`from`) to it. */
  crash(from: number): void {
    this.setSaiph('is-crash');
    this.labels.get('saiph')?.classList.add('is-crash');
    const start = this.dots.get(from);
    if (start) {
      this.crashLine?.remove();
      this.crashLine = s('line', {
        x1: start.getAttribute('cx')!,
        y1: start.getAttribute('cy')!,
        x2: this.saiph.getAttribute('cx')!,
        y2: this.saiph.getAttribute('cy')!,
        class: 'fz-sky__link fz-sky__link--crash is-lit',
      });
      // Under the dots, over the guides.
      this.svg.insertBefore(this.crashLine, this.pulses);
    }
    this.pulse('saiph', true);
  }

  /** Rings the points a visitor's own input reached (empty to clear). */
  mark(points: number[], crashed = false): void {
    for (const [id, d] of this.dots) d.classList.toggle('is-you', points.includes(id));
    this.saiph.classList.toggle('is-you', crashed);
  }

  private setSaiph(state: 'is-lit' | 'is-crash'): void {
    if (this.saiph.classList.contains('is-crash')) return;
    this.lit.add('saiph');
    this.saiph.classList.add(state);
    this.labels.get('saiph')?.classList.add('is-lit');
    this.refreshLinks();
  }

  private refreshLinks(): void {
    for (const { link, node } of this.lines) {
      if (this.lit.has(link.a) && this.lit.has(link.b)) node.classList.add('is-lit');
    }
  }

  private pulse(id: number | 'saiph', big = false): void {
    // At full speed dozens of paths land in one frame; a few rings say it, more just clutter.
    if (!this.motion || (!big && this.pulses.childElementCount >= 6)) return;
    const target = id === 'saiph' ? this.saiph : this.dots.get(id);
    if (!target) return;
    const ring = add(this.pulses, 'circle', {
      cx: target.getAttribute('cx')!,
      cy: target.getAttribute('cy')!,
      r: big ? 8 : 4,
      class: big ? 'fz-sky__pulse fz-sky__pulse--crash' : 'fz-sky__pulse',
    });
    ring.addEventListener('animationend', () => ring.remove(), { once: true });
    setTimeout(() => ring.remove(), 2000);
  }
}
