/**
 * The hero's sky, on one canvas.
 *
 * 1. Construction (once, ~3.8 s): a starfield fades in; Orion's stars brighten at their real
 *    positions and are named; the classical figure is drawn between them; the stars glide onto the
 *    logo's construction points (the belt settles onto its 1:2 slope, the corners mirror across
 *    Alnilam) while the guides show it; the hourglass fills from Alnilam outwards, and the three
 *    corner stars sink into it, staying faintly visible; Betelgeuse and Mintaka land. The result is
 *    the brand mark in the page's figure, which the canvas replaces (the figure stays in the page
 *    for its name and as the no-script view, with the same faint stars).
 * 2. A live star chart: over the sky, a reticle follows the pointer and reads out the right
 *    ascension and declination beneath it; it takes the colour of what is under it (Void over the
 *    mark's Starlight body), and anywhere on or near one of Orion's stars it locks on and gives its
 *    name, catalogue position and one fact.
 * 3. On scroll, the camera flies into the mark. The sky has depth: some of its stars lie between us
 *    and the mark and stream past in streaks (as long as the scroll is fast), the far ones barely
 *    move, the camera banks a little, and the mark grows until its body is all there is. Inside the
 *    body its own faint stars rush past last, and what remains is the next chapter's Starlight.
 *
 * Geometry is in the mark's own units (the brand file's viewBox), placed exactly over the figure;
 * the star data (real positions fitted onto the logo's frame, names, facts, the fit itself) comes
 * from the page's `data-sky`, computed by the site generator from the same catalogue as the lab.
 */

import { formatRaDec, unproject } from './astro.ts';
import { clamp, easeInOut, easeOut, easeOutBack, lerp, span } from './math.ts';

type Pt = [number, number];

interface Star {
  id: string;
  /** Name, fact and catalogue position, in the page's language. */
  n: string;
  f: string;
  c: string;
  /** Real position and logo construction point, in mark units (no logo point for Meissa). */
  s: Pt;
  l: Pt | null;
  /** Brightness on the brand board (Betelgeuse 23 … Meissa 4.5). */
  r: number;
}

interface Data {
  /** Sky → mark: one rotation and scale, as the complex number a + bi. */
  fit: [number, number];
  /** The projection centre (Alnilam), degrees. */
  c0: [number, number];
  stars: Star[];
}

export interface Sky {
  /** The hero's scroll progress: 0 at rest, 1 at the end of its pinned track. */
  zoom(p: number): void;
  /** True once the mark's body covers the screen. */
  covered(): boolean;
  /** Stop or restart everything that moves on its own (the reader's pause control). */
  setPaused(paused: boolean): void;
}

/** The classical stick figure, as on the brand board. */
const FIGURE: [string, string][] = [
  ['betelgeuse', 'meissa'],
  ['meissa', 'bellatrix'],
  ['betelgeuse', 'alnitak'],
  ['bellatrix', 'mintaka'],
  ['alnitak', 'alnilam'],
  ['alnilam', 'mintaka'],
  ['alnitak', 'saiph'],
  ['mintaka', 'rigel'],
];
/** The hourglass's top and bottom edges, which the classical figure lacks. */
const CLOSE: [string, string][] = [
  ['betelgeuse', 'bellatrix'],
  ['saiph', 'rigel'],
];
/** The point the camera flies into: the middle of the mark's upper body, in mark units. */
const DOOR: Pt = [-2, -62];
/** How far the camera banks on the way in, at most (radians). */
const ROLL = 0.05;
/** A streak runs back this many frames' worth of motion. */
const STREAK = 1.5;

const WHITE = '244,245,247';
const VOID = '10,11,13';
const COOL = '143,179,255';
const WARM = '242,154,126';
const STARLIGHT = '#f4f5f7';
const RGB_VOID = `rgb(${VOID})`;
/** Sky colour → brand colour for the two named stars (Glow → Betelgeuse, Sky → Mintaka). */
const TINTS: Record<string, [number[], number[]]> = {
  betelgeuse: [[242, 154, 126], [226, 85, 45]],
  mintaka: [[143, 179, 255], [61, 123, 255]],
};
const RGB_WHITE = `rgb(${WHITE})`;

/**
 * Floats per star: x, y, radius, alpha, twinkle speed, phase, parallax, birth, distance. The body's
 * own stars use x, y (in a unit disc about the door), radius, alpha and distance.
 */
const FIELD = 9;

export function sky(pin: HTMLElement, canvas: HTMLCanvasElement, fig: HTMLElement, opts: { intro: boolean; fine: boolean }): Sky {
  const ctx = canvas.getContext('2d');
  const svg = fig.querySelector('svg');
  const raw = canvas.dataset['sky'];
  if (!ctx || !svg || !raw) return { zoom() {}, covered: () => false, setPaused() {} };
  const g = ctx;
  const data = JSON.parse(raw) as Data;
  const stars = data.stars;
  const vb = svg.viewBox.baseVal;
  const body = new Path2D(svg.querySelector('path')?.getAttribute('d') ?? '');
  // The mark's own dots (Betelgeuse 20, Mintaka 14, the two belt stars 6) and the corner stars'
  // faint ones (`lg-f`), by position.
  const dotR = new Map<string, number>();
  const faintR = new Map<string, number>();
  for (const st of stars) {
    if (!st.l) continue;
    for (const c of svg.querySelectorAll('circle')) {
      if (Math.abs(Number(c.getAttribute('cx')) - st.l[0]) < 0.5 && Math.abs(Number(c.getAttribute('cy')) - st.l[1]) < 0.5) {
        (c.classList.contains('lg-f') ? faintR : dotR).set(st.id, Number(c.getAttribute('r')));
      }
    }
  }
  const index = new Map(stars.map((st, i) => [st.id, i]));
  // Per star, this frame: position in mark units and drawn radius (reused, never reallocated).
  const pos: Pt[] = stars.map(() => [0, 0]);
  const shownR = new Float32Array(stars.length);

  let W = 0;
  let H = 0;
  let dpr = 1;
  // Mark units → CSS pixels at rest: x = ox + k·u.
  let k = 1;
  let ox = 0;
  let oy = 0;
  let zmax = 20;
  let field = new Float32Array(0);
  let inner = new Float32Array(0);
  // Where each star was drawn last frame (for its streak), and whether that is known.
  let trail = new Float32Array(0);
  let trailIn = new Float32Array(0);
  let fresh = true;
  let tints: string[] = [];

  // The sky's own clock: it runs only while the sky is allowed to move (not when paused).
  let clock = 0;
  let t0 = opts.intro ? 0 : -1e6;
  let paused = false;
  let progress = 0;
  // The zoom: where the scroll says (zq) and where the camera is (ze, which follows with a little
  // weight, so a wheel's steps become one flight).
  let zq = 0;
  let ze = 0;
  let flow = 0;
  // Pointer: normalised target and eased value for parallax; raw and eased position for the reticle.
  let tx = 0;
  let ty = 0;
  let px = 0;
  let py = 0;
  let mx = -1;
  let my = -1;
  let rx = -1;
  let ry = -1;
  let over = false;
  let moved = 0;
  let ink = 0;
  let snapped: Star | null = null;
  let factLines: string[] = [];
  let drawn = performance.now();
  let raf = 0;
  let visible = true;

  // The few fonts the canvas uses, built once; a font is assigned only when it changes, because
  // every assignment makes the browser resolve it again.
  const mono = (size: number, weight = 400) => `${weight} ${size}px "IBM Plex Mono", ui-monospace, monospace`;
  const F_NAME = mono(10.5, 500);
  const F_RATIO = mono(11, 500);
  const F_STAR = mono(12.5, 500);
  const F_SMALL = mono(10.5);
  const F_FACT = mono(11);
  let fontNow = '';
  const font = (f: string) => {
    if (f !== fontNow) g.font = fontNow = f;
  };

  /** A deterministic starfield (mulberry32), so the sky is the same on every visit. */
  function makeField() {
    let seed = 0x4d696e74;
    const rnd = () => {
      seed = (seed + 0x6d2b79f5) >>> 0;
      let t = seed;
      t = Math.imul(t ^ (t >>> 15), t | 1);
      t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
      return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
    };
    const n = Math.round(clamp((W * H) / 2000, 220, 950));
    const list: { tint: number; v: number[] }[] = [];
    for (let i = 0; i < n; i++) {
      const m = rnd();
      const radius = m > 0.975 ? 1.45 + rnd() * 0.65 : m > 0.82 ? 0.9 + rnd() * 0.4 : 0.45 + rnd() * 0.35;
      const t = rnd();
      const v = [rnd() * W, rnd() * H, radius, 0.3 + rnd() * 0.62, 0.5 + rnd() * 1.7, rnd() * 6.283, 0.15 + rnd() * 0.85, rnd(), 0];
      // Distance (the mark is at 1), derived from the star's own numbers so the sky at rest is
      // unchanged: two in five lie between us and the mark, mostly just in front of it.
      const u1 = (v[5]! * 1.5471) % 1;
      const u2 = (v[7]! * 13.37) % 1;
      v[8] = u1 < 0.4 ? 1 - 0.7 * u2 * u2 : 1.25 + 4 * u2 ** 1.4;
      list.push({ tint: t > 0.93 ? 1 : t > 0.88 ? 2 : 0, v });
    }
    // Grouped by colour, so the fill style changes three times a frame, not a thousand.
    list.sort((a, b) => a.tint - b.tint);
    field = new Float32Array(n * FIELD);
    tints = list.map((s) => `rgb(${s.tint === 1 ? COOL : s.tint === 2 ? WARM : WHITE})`);
    list.forEach((s, i) => field.set(s.v, i * FIELD));
    // The stars inside the mark: Void on Starlight, met only on the way in.
    const m = Math.round(n * 0.3);
    inner = new Float32Array(m * FIELD);
    for (let i = 0; i < m; i++) {
      const a = rnd() * 6.283;
      const d = Math.sqrt(rnd());
      inner.set([Math.cos(a) * d, Math.sin(a) * d, 0.55 + rnd() * 0.95, 0.14 + rnd() * 0.3, 0, 0, 0, 0, 0.3 + rnd() * 1.3], i * FIELD);
    }
    trail = new Float32Array(n * 2);
    trailIn = new Float32Array(m * 2);
    fresh = true;
  }

  function layout() {
    const pr = pin.getBoundingClientRect();
    const fr = fig.getBoundingClientRect();
    if (!pr.width || !fr.width) return;
    const resized = Math.abs(pr.width - W) > 1 || Math.abs(pr.height - H) > 1;
    W = pr.width;
    H = pr.height;
    dpr = Math.min(window.devicePixelRatio || 1, 2);
    canvas.width = Math.round(W * dpr);
    canvas.height = Math.round(H * dpr);
    // Resizing the canvas resets its state, the font included.
    fontNow = '';
    k = fr.width / vb.width;
    ox = fr.left - pr.left - vb.x * k;
    oy = fr.top - pr.top - vb.y * k;
    // Enough zoom for the body around the door (≈36 units of clear Starlight) to cover the corners.
    zmax = (Math.hypot(W / 2, H / 2) / (36 * k)) * 1.3;
    if (resized || field.length === 0) makeField();
    fresh = true;
  }

  // The transform of the mark this frame: screen = T + S·R·u, R the camera's roll.
  let S = 1;
  let Tx = 0;
  let Ty = 0;
  let Rc = 1;
  let Rs = 0;
  // The vanishing point: where the door is on the screen.
  let vx = 0;
  let vy = 0;

  function frame(now: number) {
    raf = 0;
    const settled = ze === zq && flow === 0;
    // Idle — only the twinkle moving, each star over seconds: twenty frames a second look the
    // same and cost a third.
    if (settled && clock - t0 > 4200 && now - moved > 600 && now - drawn < 48) {
      raf = requestAnimationFrame(frame);
      return;
    }
    const dt = Math.min(64, now - drawn);
    drawn = now;
    if (!paused) clock += dt;
    const T = clock - t0;
    // Ease the pointer: parallax slowly (held still while paused), the reticle a little faster.
    const ep = paused ? 0 : 1 - Math.exp(-dt / 140);
    px += (tx - px) * ep;
    py += (ty - py) * ep;
    const er = paused ? 1 : 1 - Math.exp(-dt / 60);
    rx = rx < 0 ? mx : rx + (mx - rx) * er;
    ry = ry < 0 ? my : ry + (my - ry) * er;
    // The camera follows the scroll with a little weight; `flow` is how fast it is moving.
    const was = ze;
    ze += (zq - ze) * (1 - Math.exp(-dt / 110));
    if (Math.abs(zq - ze) < 0.0004) ze = zq;
    flow += (clamp(Math.abs(ze - was) / Math.max(1, dt) / 0.0005) - flow) * (1 - Math.exp(-dt / 150));
    if (flow < 0.01) flow = 0;

    // Linear in log-space: a steady push through the mark, not a sudden lurch.
    const Z = Math.exp(ze * Math.log(Math.max(1, zmax)));
    const pull = easeInOut(ze);
    const roll = -ROLL * Math.sin(Math.PI * ze);
    Rc = Math.cos(roll);
    Rs = Math.sin(roll);
    vx = lerp(ox + DOOR[0] * k, W / 2, pull) + px * 7 * (1 - ze);
    vy = lerp(oy + DOOR[1] * k, H / 2, pull) + py * 5 * (1 - ze);
    S = k * Z;
    Tx = vx - S * (Rc * DOOR[0] - Rs * DOOR[1]);
    Ty = vy - S * (Rs * DOOR[0] + Rc * DOOR[1]);

    g.setTransform(dpr, 0, 0, dpr, 0, 0);
    g.clearRect(0, 0, W, H);
    g.lineCap = 'round';
    if (ze < 0.985) {
      layer(field, trail, 1 - 1 / Z, 1 - span(ze, 0.74, 0.97), false, span(T, 0, 1100));
      const fill = drawOrion(T);
      // Inside the body, its own stars rush past — only while the camera is moving.
      const a = span(ze, 0.42, 0.6) * (1 - span(ze, 0.84, 0.97)) * flow;
      if (a > 0) {
        g.save();
        toMarkSpace();
        g.clip(body);
        g.setTransform(dpr, 0, 0, dpr, 0, 0);
        layer(inner, trailIn, span(ze, 0.38, 1) * 0.985, a, true, 1);
        g.restore();
      } else trailIn.fill(NaN);
      if (opts.fine && over && mx >= 0 && T > 1300 && ze < 0.02) drawReticle(T, fill, dt);
    } else {
      g.fillStyle = STARLIGHT;
      g.fillRect(0, 0, W, H);
    }
    fresh = false;
    // Keep going while anything moves: the twinkle (unless paused), or the camera still settling.
    // Once the body covers the screen nothing shows, so it rests until the scroll moves again.
    const still = ze === zq && flow === 0;
    if (visible && !document.hidden && !(still && ze >= 0.985) && (!paused || !still)) raf = requestAnimationFrame(frame);
  }

  /**
   * One layer of stars in depth. Each lies at distance D (the mark is at 1): as the camera closes
   * in by `c` it grows away from the vanishing point, and while it moves fast it is drawn as a
   * streak of its last frame's motion. The sky's stars twinkle, are born during the construction
   * and lean with the pointer; the body's own (`inside`) lie in a disc about the door, in Void.
   */
  function layer(list: Float32Array, tr: Float32Array, c: number, alpha: number, inside: boolean, appear: number) {
    if (alpha <= 0) {
      tr.fill(NaN);
      return;
    }
    const R0 = Math.hypot(W, H) * 0.62;
    const lean = 1 - ze;
    let tint = '';
    for (let i = 0, n = 0; i < list.length; i += FIELD, n++) {
      const born = inside ? 1 : clamp((appear - list[i + 7]! * 0.72) / 0.28);
      const D = list[i + 8]!;
      // Not yet born, or already behind the camera.
      if (born <= 0 || D - c < 0.02) {
        tr[n * 2] = NaN;
        continue;
      }
      const s = D / (D - c);
      const ux = inside ? list[i]! * R0 : list[i]! + px * 18 * list[i + 6]! * lean - vx;
      const uy = inside ? list[i + 1]! * R0 : list[i + 1]! + py * 12 * list[i + 6]! * lean - vy;
      const x = vx + (Rc * ux - Rs * uy) * s;
      const y = vy + (Rs * ux + Rc * uy) * s;
      const ddx = x - tr[n * 2]!;
      const ddy = y - tr[n * 2 + 1]!;
      tr[n * 2] = x;
      tr[n * 2 + 1] = y;
      if (x < -60 || x > W + 60 || y < -60 || y > H + 60) continue;
      const twinkle = inside ? 1 : 0.68 + 0.32 * Math.sin(clock * 0.001 * list[i + 4]! + list[i + 5]!);
      // Close stars grow a little, and fade as they pass.
      g.globalAlpha = list[i + 3]! * twinkle * born * alpha * (s > 10 ? clamp((22 - s) / 12) : 1);
      const t = inside ? RGB_VOID : tints[n]!;
      if (t !== tint) {
        tint = t;
        g.fillStyle = g.strokeStyle = t;
      }
      const r = list[i + 2]! * Math.min(3, 1 + (s - 1) * 0.18);
      if (!fresh && ddx * ddx + ddy * ddy > 2.25) {
        g.lineWidth = r * 2;
        g.beginPath();
        g.moveTo(x - ddx * STREAK, y - ddy * STREAK);
        g.lineTo(x, y);
        g.stroke();
      } else if (r < 1) {
        g.fillRect(x - r, y - r, r * 2, r * 2);
      } else {
        g.beginPath();
        g.arc(x, y, r, 0, 6.2832);
        g.fill();
      }
    }
    g.globalAlpha = 1;
  }

  /** Draw in the mark's units from here on. */
  function toMarkSpace() {
    g.setTransform(dpr * S * Rc, dpr * S * Rs, -dpr * S * Rs, dpr * S * Rc, dpr * Tx, dpr * Ty);
  }

  function screen(p: Pt): Pt {
    return [Tx + S * (Rc * p[0] - Rs * p[1]), Ty + S * (Rs * p[0] + Rc * p[1])];
  }

  /** A point on the screen, in mark units. */
  function toMark(sx: number, sy: number): Pt {
    const u = (sx - Tx) / S;
    const v = (sy - Ty) / S;
    return [Rc * u + Rs * v, -Rs * u + Rc * v];
  }

  /** Draws the constellation and the mark; returns how far the body's fill has spread (0..1). */
  function drawOrion(T: number): number {
    const lit = span(T, 250, 1100);
    const labels = span(T, 600, 1300) * (1 - span(T, 2450, 2900));
    const morph = easeInOut(span(T, 1850, 2900));
    const guides = span(T, 2050, 2500) * (1 - span(T, 2950, 3350));
    const fill = easeOut(span(T, 2750, 3500));
    const pop = span(T, 3150, 3850);
    const linesOut = 1 - span(T, 3000, 3500);
    const meissa = 1 - morph;
    for (let j = 0; j < stars.length; j++) {
      const st = stars[j]!;
      const l = st.l ?? st.s;
      pos[j]![0] = lerp(st.s[0], l[0], morph);
      pos[j]![1] = lerp(st.s[1], l[1], morph);
    }
    const P = (id: string): Pt => pos[index.get(id) ?? 0]!;

    toMarkSpace();
    const hair = 1 / S;

    // The classical figure, drawn line by line. (Fades are alpha, not new colour strings.)
    if (linesOut > 0) {
      g.lineWidth = hair;
      g.strokeStyle = `rgb(${WHITE})`;
      FIGURE.forEach(([a, b], i) => {
        const draw = easeOut(span(T, 900 + i * 95, 1500 + i * 95));
        if (draw <= 0) return;
        const [ax, ay] = P(a);
        const [bx, by] = P(b);
        g.globalAlpha = 0.4 * linesOut * (a === 'meissa' || b === 'meissa' ? meissa : 1);
        g.beginPath();
        g.moveTo(ax, ay);
        g.lineTo(lerp(ax, bx, draw), lerp(ay, by, draw));
        g.stroke();
      });
      // The hourglass closes as the stars settle.
      g.globalAlpha = 0.4 * morph * linesOut;
      g.beginPath();
      for (const [a, b] of CLOSE) {
        g.moveTo(...P(a));
        g.lineTo(...P(b));
      }
      g.stroke();
    }

    // Construction guides: the belt's 1:2 line run on, and the diagonals mirrored across Alnilam.
    if (guides > 0) {
      g.strokeStyle = `rgb(${COOL})`;
      g.globalAlpha = 0.6 * guides;
      g.lineWidth = hair;
      g.setLineDash([3, 4]);
      g.beginPath();
      g.moveTo(-26 - 80, 13 + 40);
      g.lineTo(26 + 80, -13 - 40);
      for (const [a, b] of [
        ['betelgeuse', 'rigel'],
        ['bellatrix', 'saiph'],
      ] as const) {
        g.moveTo(...P(a));
        g.lineTo(...P(b));
      }
      g.stroke();
      g.setLineDash([]);
    }
    g.globalAlpha = 1;

    // The body fills from Alnilam outwards (once filled, without the clip).
    g.fillStyle = STARLIGHT;
    if (fill >= 1) {
      g.fill(body);
    } else if (fill > 0) {
      g.save();
      g.beginPath();
      g.arc(0, 0, fill * 175, 0, 6.2832);
      g.clip();
      g.fill(body);
      g.restore();
    }

    // The stars: small and bright in the sky, then the mark's own dots — or, for the corners, the
    // faint dots they leave in its body.
    for (let j = 0; j < stars.length; j++) {
      const st = stars[j]!;
      const [x, y] = pos[j]!;
      const skyR = (1.4 + st.r * 0.11) / k;
      const final = dotR.get(st.id);
      const faint = faintR.get(st.id);
      let alpha = lit;
      let r = skyR;
      let colour = RGB_WHITE;
      let halo = 1 - pop;
      if (final !== undefined) {
        r = lerp(skyR, final, easeOutBack(pop));
        const tint = TINTS[st.id];
        if (tint) {
          const [from, to] = tint;
          colour = `rgb(${(pop >= 1 ? to : from.map((v, i) => Math.round(lerp(v, to[i]!, pop)))).join(',')})`;
        }
      } else if (faint !== undefined) {
        // It sinks into the body as the fill passes it, and comes back as a faint Void dot that
        // twinkles with the sky (and holds still with it).
        const sink = span(fill, Math.hypot(x, y) / 175 - 0.02, Math.hypot(x, y) / 175 + 0.22);
        r = lerp(skyR, faint, sink);
        halo = 1 - sink;
        if (sink < 0.5) {
          alpha = lit * (1 - sink * 2);
        } else {
          colour = RGB_VOID;
          alpha = (sink - 0.5) * 2 * 0.2 * (0.8 + 0.2 * Math.sin(clock * 0.0011 + j * 1.7));
        }
      } else {
        // Meissa, which the logo does not use, leaves.
        alpha *= meissa;
      }
      shownR[j] = r;
      if (alpha <= 0.002) continue;
      g.fillStyle = colour;
      if (halo > 0 && colour !== RGB_VOID) {
        g.globalAlpha = 0.16 * alpha * halo;
        g.beginPath();
        g.arc(x, y, r * 2.8, 0, 6.2832);
        g.fill();
      }
      g.globalAlpha = alpha;
      g.beginPath();
      g.arc(x, y, r, 0, 6.2832);
      g.fill();
    }
    g.globalAlpha = 1;

    // Names in the sky; the slope's ratio while the guides show.
    g.setTransform(dpr, 0, 0, dpr, 0, 0);
    if (labels > 0) {
      font(F_NAME);
      g.fillStyle = 'rgb(201,206,216)';
      for (let j = 0; j < stars.length; j++) {
        const st = stars[j]!;
        const [x, y] = screen(pos[j]!);
        g.globalAlpha = 0.9 * labels * (st.id === 'meissa' ? meissa : 1);
        g.fillText(st.n.toLocaleUpperCase(), x + 11, y + 4);
      }
    }
    if (guides > 0) {
      const [x, y] = screen([26 + 60, -13 - 30]);
      font(F_RATIO);
      g.fillStyle = `rgb(${COOL})`;
      g.globalAlpha = guides;
      g.fillText('1 : 2', x + 8, y - 6);
    }
    g.globalAlpha = 1;
    return fill;
  }

  /** Right ascension and declination under a point on the screen. */
  function radec(sx: number, sy: number): string {
    // Screen → mark units → the sky's projection plane (undoing the fit) → the sphere.
    const [u, v] = toMark(sx, sy);
    const [a, b] = data.fit;
    const d = a * a + b * b;
    const [ra, dec] = unproject((a * u + b * v) / d, (a * v - b * u) / d, data.c0);
    return formatRaDec(ra, dec);
  }

  function wrap(text: string, max: number): string[] {
    const out: string[] = [];
    let line = '';
    for (const word of text.split(' ')) {
      const next = line ? `${line} ${word}` : word;
      if (line && g.measureText(next).width > max) {
        out.push(line);
        line = word;
      } else {
        line = next;
      }
    }
    if (line) out.push(line);
    return out.slice(0, 4);
  }

  function drawReticle(T: number, fill: number, dt: number) {
    const morph = easeInOut(span(T, 1850, 2900));
    // Lock onto the star whose reach the pointer is deepest in: anywhere on its dot, or near it.
    let best: Star | null = null;
    let bestQ = 1;
    for (let j = 0; j < stars.length; j++) {
      const st = stars[j]!;
      if (st.id === 'meissa' && morph > 0.5) continue;
      const [x, y] = screen(pos[j]!);
      const reach = Math.max(30, shownR[j]! * S + 12);
      const q = Math.hypot(x - mx, y - my) / reach;
      if (q < bestQ) {
        bestQ = q;
        best = st;
      }
    }
    if (best !== snapped) {
      snapped = best;
      font(F_FACT);
      factLines = best ? wrap(best.f, 236) : [];
    }
    const j = snapped ? (index.get(snapped.id) ?? 0) : -1;
    const target = j >= 0 ? screen(pos[j]!) : null;
    const x = target ? lerp(rx, target[0], 0.55) : rx;
    const y = target ? lerp(ry, target[1], 0.55) : ry;
    const R = j >= 0 ? Math.max(17, shownR[j]! * S + 7) : 11;

    // Over the mark's Starlight body (once filled there, and not on a coloured dot) the reticle
    // turns Void, so it never vanishes white on white.
    const [u, v] = toMark(mx, my);
    let dark = false;
    if (fill > 0 && Math.hypot(u, v) < fill * 175) {
      g.setTransform(1, 0, 0, 1, 0, 0);
      dark = g.isPointInPath(body, u, v);
      for (const [id, dr] of dotR) {
        const p = pos[index.get(id) ?? 0]!;
        if (dr > 8 && Math.hypot(u - p[0], v - p[1]) < dr) dark = false;
      }
    }
    ink += ((dark ? 1 : 0) - ink) * (1 - Math.exp(-dt / 70));
    // Starlight (244,245,247) to Void (10,11,13): one step for all three channels.
    const c = Math.round(lerp(244, 10, ink));

    g.setTransform(dpr, 0, 0, dpr, 0, 0);
    g.strokeStyle = `rgba(${c},${c + 1},${c + 3},0.8)`;
    g.lineWidth = 1;
    g.beginPath();
    g.arc(x, y, R, 0, 6.2832);
    for (const [dx, dy] of [
      [1, 0],
      [-1, 0],
      [0, 1],
      [0, -1],
    ] as const) {
      g.moveTo(x + dx * (R + 4), y + dy * (R + 4));
      g.lineTo(x + dx * (R + 11), y + dy * (R + 11));
    }
    g.stroke();

    const lines: [string, string, string][] = snapped
      ? [
          [snapped.n, F_STAR, STARLIGHT],
          [snapped.c, F_SMALL, 'rgba(201,206,216,0.95)'],
          ...factLines.map((l): [string, string, string] => [l, F_FACT, 'rgba(201,206,216,0.95)']),
        ]
      : [[radec(mx, my), F_SMALL, 'rgba(201,206,216,0.95)']];
    let wMax = 0;
    for (const [text, f] of lines) {
      font(f);
      wMax = Math.max(wMax, g.measureText(text).width);
    }
    const lh = 17;
    const boxW = wMax + 20;
    const boxH = lines.length * lh + 12;
    let bx = x + R + 16;
    if (bx + boxW > W - 12) bx = x - R - 16 - boxW;
    const by = clamp(y - 14, 12, H - boxH - 12);
    g.fillStyle = 'rgba(10,11,13,0.9)';
    g.strokeStyle = 'rgba(244,245,247,0.14)';
    g.beginPath();
    g.roundRect(bx + 0.5, by + 0.5, boxW, boxH, 8);
    g.fill();
    g.stroke();
    lines.forEach(([text, f, colour], i) => {
      font(f);
      g.fillStyle = colour;
      g.fillText(text, bx + 10, by + 18 + i * lh);
    });
  }

  function kick() {
    if (!raf && visible && !document.hidden) {
      drawn = performance.now();
      raf = requestAnimationFrame(frame);
    }
  }

  // Pointer: parallax anywhere on the hero; the reticle only over open sky, never over the words.
  pin.addEventListener('pointermove', (e) => {
    const r = pin.getBoundingClientRect();
    mx = e.clientX - r.left;
    my = e.clientY - r.top;
    tx = (mx / r.width) * 2 - 1;
    ty = (my / r.height) * 2 - 1;
    over = e.pointerType === 'mouse' && !(e.target as Element).closest('a, button, h1, p, .hero__cta');
    moved = performance.now();
    if (paused) kick();
  });
  pin.addEventListener('pointerleave', () => {
    over = false;
    tx = 0;
    ty = 0;
    moved = performance.now();
  });

  new IntersectionObserver(([entry]) => {
    visible = !!entry?.isIntersecting;
    fresh = true;
    kick();
  }).observe(pin);
  document.addEventListener('visibilitychange', kick);
  let pending = 0;
  new ResizeObserver(() => {
    cancelAnimationFrame(pending);
    pending = requestAnimationFrame(() => {
      layout();
      kick();
    });
  }).observe(pin);
  layout();
  kick();

  return {
    zoom(p: number) {
      progress = p;
      zq = span(p, 0.06, 0.84);
      // Scrolling during the construction finishes it at once: the zoom needs the whole mark.
      if (p > 0.02 && clock - t0 < 3900) t0 = clock - 3900;
      kick();
    },
    covered: () => progress > 0 && zq > 0.8,
    setPaused(next: boolean) {
      paused = next;
      kick();
    },
  };
}
