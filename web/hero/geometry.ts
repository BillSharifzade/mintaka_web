// The hourglass, in the logo's own units (brand board 10): Alnilam at 0,0, y down. The filled
// shape is two quads — Betelgeuse·Bellatrix·Mintaka·Alnitak on top, Alnitak·Mintaka·Rigel·Saiph
// below — grown by 9 units, with the belt cutting a channel 22 units wide through the waist.
// Everything here is pure and shared by the WebGL2 shaders (as constants) and the Canvas tier.

export type V2 = readonly [number, number];

export const LOGO: Record<string, V2> = {
  betelgeuse: [-64, -104],
  bellatrix: [56, -90],
  mintaka: [26, -13],
  alnilam: [0, 0],
  alnitak: [-26, 13],
  saiph: [-56, 104],
  rigel: [64, 90],
};

/** The logo's viewBox: x from −84, y from −124, 157.03 × 237.01 units. */
export const VIEWBOX = { x: -84, y: -124, w: 157.03, h: 237.01 };
export const GROW = 9;
/** Particle kinds by seed: field stars (the background sky), threats (2 %), then the flow. */
export const FIELD_SHARE = 0.16;
export const THREAT_SHARE = 0.02;
export const HALF_CHANNEL = 11;

/** Along the belt, towards Mintaka. */
export const U: V2 = [2 / Math.sqrt(5), -1 / Math.sqrt(5)];
/** Across the belt, towards the bottom bulb. */
export const NB: V2 = [1 / Math.sqrt(5), 2 / Math.sqrt(5)];
/** Where Mintaka sits along the belt. */
export const MINTAKA_S = LOGO.mintaka![0] * U[0] + LOGO.mintaka![1] * U[1];

/** A half-plane `nx·x + ny·y − c`, positive outside. */
export type Edge = readonly [number, number, number];

function edge(a: V2, b: V2, inside: V2): Edge {
  const ex = b[0] - a[0];
  const ey = b[1] - a[1];
  const len = Math.hypot(ex, ey);
  let nx = ey / len;
  let ny = -ex / len;
  if (nx * (inside[0] - a[0]) + ny * (inside[1] - a[1]) > 0) {
    nx = -nx;
    ny = -ny;
  }
  return [nx, ny, nx * a[0] + ny * a[1]];
}

const L = LOGO as Record<string, V2>;
const IN_TOP: V2 = [-2, -48.5];
const IN_BOTTOM: V2 = [2, 48.5];

export const EDGES = {
  top: edge(L['betelgeuse']!, L['bellatrix']!, IN_TOP),
  rightT: edge(L['bellatrix']!, L['mintaka']!, IN_TOP),
  leftT: edge(L['alnitak']!, L['betelgeuse']!, IN_TOP),
  leftB: edge(L['saiph']!, L['alnitak']!, IN_BOTTOM),
  rightB: edge(L['mintaka']!, L['rigel']!, IN_BOTTOM),
  bottom: edge(L['rigel']!, L['saiph']!, IN_BOTTOM),
};

const dist = (e: Edge, x: number, y: number) => e[0] * x + e[1] * y - e[2];
export const beltD = (x: number, y: number) => NB[0] * x + NB[1] * y;
export const beltS = (x: number, y: number) => U[0] * x + U[1] * y;

/** Signed distance (approx.) to the grown top quad, its belt side included. */
export function sdTop(x: number, y: number): number {
  return Math.max(dist(EDGES.top, x, y), dist(EDGES.rightT, x, y), dist(EDGES.leftT, x, y), beltD(x, y)) - GROW;
}

/** Signed distance (approx.) to the grown bottom quad; `open` leaves the bottom edge open. */
export function sdBottom(x: number, y: number, open = false): number {
  const sides = Math.max(dist(EDGES.leftB, x, y), dist(EDGES.rightB, x, y), -beltD(x, y));
  return (open ? sides : Math.max(sides, dist(EDGES.bottom, x, y))) - GROW;
}

export function sdVessel(x: number, y: number, openBottom = false): number {
  return Math.min(sdTop(x, y), sdBottom(x, y, openBottom));
}

/** y of the grown bottom edge at x (particles leave the vessel below it). */
export function bottomEdgeY(x: number): number {
  const [nx, ny, c] = EDGES.bottom;
  return (c + GROW - nx * x) / ny;
}

/** A point inside a bulb for 0 ≤ u, v ≤ 1, spread evenly: bilinear over the quad's corners,
 *  pulled 18 % towards its centre and kept clear of the belt channel. */
export function fillTarget(bulb: 'top' | 'bottom', u: number, v: number): [number, number] {
  const q = bulb === 'top'
    ? [LOGO['betelgeuse']!, LOGO['bellatrix']!, LOGO['mintaka']!, LOGO['alnitak']!]
    : [LOGO['alnitak']!, LOGO['mintaka']!, LOGO['rigel']!, LOGO['saiph']!];
  const [cx, cy] = bulb === 'top' ? [-2, -48.5] : [2, 48.5];
  const vv = bulb === 'top' ? v * 0.78 : 0.22 + v * 0.74;
  const [a, b, c, d] = q.map((p): V2 => [cx + (p[0] - cx) * 0.82, cy + (p[1] - cy) * 0.82]) as [V2, V2, V2, V2];
  const top = [a[0] + (b[0] - a[0]) * u, a[1] + (b[1] - a[1]) * u] as const;
  const bot = [d[0] + (c[0] - d[0]) * u, d[1] + (c[1] - d[1]) * u] as const;
  return [top[0] + (bot[0] - top[0]) * vv, top[1] + (bot[1] - top[1]) * vv];
}

/** The four output lanes under the bottom bulb, x in logo units. */
export const LANES = [-36, -9, 18, 45] as const;

export type Region = 'top' | 'channel' | 'bottom' | 'outside';

export function region(x: number, y: number): Region {
  const d = beltD(x, y);
  if (d < -HALF_CHANNEL) return sdTop(x, y) < 0 ? 'top' : 'outside';
  if (d > HALF_CHANNEL) return sdBottom(x, y) < 0 ? 'bottom' : 'outside';
  return sdVessel(x, y) < 0 ? 'channel' : 'outside';
}

/** Scroll progress through S1–S3: 0 at the top, 1 at the light ray, 2 at Orion, 3 when gone. */
export function progress(scrollY: number, keys: readonly [number, number, number, number]): number {
  const [k0, k1, k2, k3] = keys;
  if (scrollY <= k0) return 0;
  if (scrollY < k1) return (scrollY - k0) / Math.max(1, k1 - k0);
  if (scrollY < k2) return 1 + (scrollY - k1) / Math.max(1, k2 - k1);
  if (scrollY < k3) return 2 + (scrollY - k2) / Math.max(1, k3 - k2);
  return 3;
}

export const smoothstep = (a: number, b: number, x: number): number => {
  const t = Math.min(1, Math.max(0, (x - a) / (b - a)));
  return t * t * (3 - 2 * t);
};

export interface Weights {
  flow: number;
  ray: number;
  orion: number;
  /** Fades everything out as Orion leaves the screen. */
  alpha: number;
  /** Progress of the logo's stars towards their places in the sky. */
  morph: number;
}

export function weights(t: number): Weights {
  const flow = 1 - smoothstep(0.12, 0.8, t);
  const ray = smoothstep(0.35, 0.95, t) * (1 - smoothstep(1.25, 1.75, t));
  const orion = smoothstep(1.35, 1.95, t);
  return { flow, ray, orion, alpha: 1 - smoothstep(2.45, 2.95, t), morph: smoothstep(0.3, 1.9, t) };
}
