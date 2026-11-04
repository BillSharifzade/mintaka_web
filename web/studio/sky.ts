/**
 * The hero's sky, on one canvas.
 *
 * 1. Construction (once, ~3.8 s): a starfield fades in; Orion's stars brighten at their real
 *    positions and are named; the classical figure is drawn between them; the stars glide onto the
 *    logo's construction points (the belt settles onto its 1:2 slope, the corners mirror across
 *    Alnilam) while the guides show it; the hourglass fills from Alnilam outwards; Betelgeuse and
 *    Mintaka land. The result is pixel-for-pixel the brand mark in the page's figure, which the
 *    canvas replaces (the figure stays in the page for its name and as the no-script view).
 * 2. A live star chart: over the sky, a reticle follows the pointer and reads out the right
 *    ascension and declination beneath it; near one of Orion's stars it snaps to it and gives its
 *    name, catalogue position and one fact.
 * 3. On scroll, the page zooms into the mark's upper body until it fills the screen — the next
 *    chapter's Starlight.
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
/** The point the page zooms into: the middle of the mark's upper body, in mark units. */
const DOOR: Pt = [-2, -62];

const WHITE = '244,245,247';
const COOL = '143,179,255';
const WARM = '242,154,126';
const STARLIGHT = '#f4f5f7';
/** Sky colour → brand colour for the two named stars (Glow → Betelgeuse, Sky → Mintaka). */
const TINTS: Record<string, [number[], number[]]> = {
  betelgeuse: [[242, 154, 126], [226, 85, 45]],
  mintaka: [[143, 179, 255], [61, 123, 255]],
};

const FIELD = 8; // floats per background star: x, y, radius, alpha, speed, phase, depth, birth

export function sky(pin: HTMLElement, canvas: HTMLCanvasElement, fig: HTMLElement, opts: { intro: boolean; fine: boolean }): Sky {
  const ctx = canvas.getContext('2d');
  const svg = fig.querySelector('svg');
  const raw = canvas.dataset['sky'];
  if (!ctx || !svg || !raw) return { zoom() {}, covered: () => false, setPaused() {} };
  const data = JSON.parse(raw) as Data;
  const vb = svg.viewBox.baseVal;
  const body = new Path2D(svg.querySelector('path')?.getAttribute('d') ?? '');
  // The mark's own dots (Betelgeuse 20, Mintaka 14, the two belt stars 6), by position.
  const dotR = new Map<string, number>();
  for (const st of data.stars) {
    if (!st.l) continue;
    for (const c of svg.querySelectorAll('circle')) {
      if (Math.abs(Number(c.getAttribute('cx')) - st.l[0]) < 0.5 && Math.abs(Number(c.getAttribute('cy')) - st.l[1]) < 0.5) {
        dotR.set(st.id, Number(c.getAttribute('r')));
      }
    }
  }
  const byId = new Map(data.stars.map((st) => [st.id, st]));

  let W = 0;
  let H = 0;
  let dpr = 1;
  // Mark units → CSS pixels at rest: x = ox + k·u.
  let k = 1;
  let ox = 0;
  let oy = 0;
  let zmax = 20;
  let field = new Float32Array(0);
  let tints: string[] = [];

  // The sky's own clock: it runs only while the sky is allowed to move (not when paused).
  let clock = 0;
  let t0 = opts.intro ? 0 : -1e6;
  let paused = false;
  let progress = 0;
  let zq = 0;
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
  let snapped: Star | null = null;
  let factLines: string[] = [];
  let last = performance.now();
  let raf = 0;
  let visible = true;

  const mono = (size: number, weight = 400) => `${weight} ${size}px "IBM Plex Mono", ui-monospace, monospace`;

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
    const stars: { tint: number; v: number[] }[] = [];
    for (let i = 0; i < n; i++) {
      const m = rnd();
      const radius = m > 0.975 ? 1.45 + rnd() * 0.65 : m > 0.82 ? 0.9 + rnd() * 0.4 : 0.45 + rnd() * 0.35;
      const t = rnd();
      stars.push({
        tint: t > 0.93 ? 1 : t > 0.88 ? 2 : 0,
        v: [rnd() * W, rnd() * H, radius, 0.3 + rnd() * 0.62, 0.5 + rnd() * 1.7, rnd() * 6.283, 0.15 + rnd() * 0.85, rnd()],
      });
    }
    // Grouped by colour, so the fill style changes three times a frame, not a thousand.
    stars.sort((a, b) => a.tint - b.tint);
    field = new Float32Array(n * FIELD);
    tints = stars.map((s) => (s.tint === 1 ? COOL : s.tint === 2 ? WARM : WHITE));
    stars.forEach((s, i) => field.set(s.v, i * FIELD));
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
    k = fr.width / vb.width;
    ox = fr.left - pr.left - vb.x * k;
    oy = fr.top - pr.top - vb.y * k;
    // Enough zoom for the body around the door (≈36 units of clear Starlight) to cover the corners.
    zmax = (Math.hypot(W / 2, H / 2) / (36 * k)) * 1.3;
    if (resized || field.length === 0) makeField();
  }

  // The transform of the mark this frame: screen = T + S·u.
  let S = 1;
  let Tx = 0;
  let Ty = 0;

  function frame(now: number) {
    raf = 0;
    const dt = Math.min(64, now - last);
    last = now;
    if (!paused) clock += dt;
    const T = clock - t0;
    // Ease the pointer: parallax slowly (held still while paused), the reticle a little faster.
    const ep = paused ? 0 : 1 - Math.exp(-dt / 140);
    px += (tx - px) * ep;
    py += (ty - py) * ep;
    const er = paused ? 1 : 1 - Math.exp(-dt / 60);
    rx = rx < 0 ? mx : rx + (mx - rx) * er;
    ry = ry < 0 ? my : ry + (my - ry) * er;

    // Linear in log-space: the zoom feels like a steady push through the mark, not a sudden lurch.
    const Z = Math.exp(zq * Math.log(Math.max(1, zmax)));
    const pull = easeInOut(zq);
    const doorX = ox + DOOR[0] * k;
    const doorY = oy + DOOR[1] * k;
    S = k * Z;
    Tx = lerp(doorX, W / 2, pull) - DOOR[0] * S + px * 7 * (1 - zq);
    Ty = lerp(doorY, H / 2, pull) - DOOR[1] * S + py * 5 * (1 - zq);

    ctx!.setTransform(dpr, 0, 0, dpr, 0, 0);
    ctx!.clearRect(0, 0, W, H);
    drawField(clock, span(T, 0, 1100), Z);
    drawOrion(T);
    if (zq > 0.985) {
      ctx!.setTransform(dpr, 0, 0, dpr, 0, 0);
      ctx!.fillStyle = STARLIGHT;
      ctx!.fillRect(0, 0, W, H);
    }
    if (opts.fine && over && mx >= 0 && T > 1300 && zq < 0.02) drawReticle(T);
    if (visible && !document.hidden && !paused) raf = requestAnimationFrame(frame);
  }

  function drawField(now: number, appear: number, Z: number) {
    const c = ctx!;
    const cx = W / 2;
    const cy = H / 2;
    // On the zoom the stars stream outwards, nearer ones faster.
    const warp = (Z - 1) * 0.045;
    const fade = 1 - zq * 0.8;
    let tint = '';
    for (let i = 0, n = 0; i < field.length; i += FIELD, n++) {
      const born = clamp((appear - field[i + 7]! * 0.72) / 0.28);
      if (born <= 0) continue;
      const d = field[i + 6]!;
      let x = field[i]! + px * 18 * d;
      let y = field[i + 1]! + py * 12 * d;
      if (warp > 0) {
        const w = 1 + warp * d;
        x = cx + (x - cx) * w;
        y = cy + (y - cy) * w;
      }
      const twinkle = 0.68 + 0.32 * Math.sin(now * 0.001 * field[i + 4]! + field[i + 5]!);
      if (tints[n] !== tint) {
        tint = tints[n]!;
        c.fillStyle = `rgb(${tint})`;
      }
      c.globalAlpha = field[i + 3]! * twinkle * born * fade;
      const r = field[i + 2]!;
      if (r < 1) {
        c.fillRect(x - r, y - r, r * 2, r * 2);
      } else {
        c.beginPath();
        c.arc(x, y, r, 0, 6.2832);
        c.fill();
      }
    }
    c.globalAlpha = 1;
  }

  /** A star's position this frame, in mark units: from the sky towards its construction point. */
  function at(st: Star, morph: number): Pt {
    const l = st.l ?? st.s;
    return [lerp(st.s[0], l[0], morph), lerp(st.s[1], l[1], morph)];
  }

  function screen(p: Pt): Pt {
    return [Tx + p[0] * S, Ty + p[1] * S];
  }

  function drawOrion(T: number) {
    const c = ctx!;
    const lit = span(T, 250, 1100);
    const labels = span(T, 600, 1300) * (1 - span(T, 2450, 2900));
    const morph = easeInOut(span(T, 1850, 2900));
    const guides = span(T, 2050, 2500) * (1 - span(T, 2950, 3350));
    const fill = easeOut(span(T, 2750, 3500));
    const pop = span(T, 3150, 3850);
    const linesOut = 1 - span(T, 3000, 3500);
    const meissa = 1 - morph;
    const pos = new Map(data.stars.map((st) => [st.id, at(st, morph)]));
    const P = (id: string): Pt => pos.get(id) ?? [0, 0];

    c.setTransform(dpr * S, 0, 0, dpr * S, dpr * Tx, dpr * Ty);
    c.lineCap = 'round';
    const hair = 1 / S;

    // The classical figure, drawn line by line.
    if (linesOut > 0) {
      c.lineWidth = hair;
      FIGURE.forEach(([a, b], i) => {
        const draw = easeOut(span(T, 900 + i * 95, 1500 + i * 95));
        if (draw <= 0) return;
        const head = a === 'meissa' || b === 'meissa' ? meissa : 1;
        const [ax, ay] = P(a);
        const [bx, by] = P(b);
        c.strokeStyle = `rgba(${WHITE},${(0.4 * linesOut * head).toFixed(3)})`;
        c.beginPath();
        c.moveTo(ax, ay);
        c.lineTo(lerp(ax, bx, draw), lerp(ay, by, draw));
        c.stroke();
      });
      // The hourglass closes as the stars settle.
      c.strokeStyle = `rgba(${WHITE},${(0.4 * morph * linesOut).toFixed(3)})`;
      for (const [a, b] of CLOSE) {
        c.beginPath();
        c.moveTo(...P(a));
        c.lineTo(...P(b));
        c.stroke();
      }
    }

    // Construction guides: the belt's 1:2 line run on, and the diagonals mirrored across Alnilam.
    if (guides > 0) {
      c.strokeStyle = `rgba(${COOL},${(0.6 * guides).toFixed(3)})`;
      c.lineWidth = hair;
      c.setLineDash([3, 4]);
      c.beginPath();
      c.moveTo(-26 - 80, 13 + 40);
      c.lineTo(26 + 80, -13 - 40);
      for (const [a, b] of [
        ['betelgeuse', 'rigel'],
        ['bellatrix', 'saiph'],
      ] as const) {
        c.moveTo(...P(a));
        c.lineTo(...P(b));
      }
      c.stroke();
      c.setLineDash([]);
    }

    // The body fills from Alnilam outwards.
    if (fill > 0) {
      c.save();
      c.beginPath();
      c.arc(0, 0, fill * 175, 0, 6.2832);
      c.clip();
      c.fillStyle = STARLIGHT;
      c.fill(body);
      c.restore();
    }

    // The stars: small and bright in the sky, then the mark's own dots (or its corners).
    for (const st of data.stars) {
      const [x, y] = P(st.id);
      const skyR = (1.4 + st.r * 0.11) / k;
      const final = dotR.get(st.id);
      let alpha = lit;
      let r = skyR;
      let colour = WHITE;
      if (final !== undefined) {
        r = lerp(skyR, final, easeOutBack(pop));
        const tint = TINTS[st.id];
        if (tint) {
          const [from, to] = tint;
          colour = from.map((v, i) => Math.round(lerp(v, to[i]!, pop))).join(',');
        }
      } else {
        // Bellatrix, Saiph and Rigel become the hourglass's corners; Meissa leaves.
        alpha *= (1 - fill) * (st.id === 'meissa' ? meissa : 1);
      }
      if (alpha <= 0.002) continue;
      if (pop < 1) {
        c.globalAlpha = 0.16 * alpha * (1 - pop);
        c.fillStyle = `rgb(${colour})`;
        c.beginPath();
        c.arc(x, y, r * 2.8, 0, 6.2832);
        c.fill();
      }
      c.globalAlpha = alpha;
      c.fillStyle = `rgb(${colour})`;
      c.beginPath();
      c.arc(x, y, r, 0, 6.2832);
      c.fill();
    }
    c.globalAlpha = 1;

    // Names in the sky; the slope's ratio while the guides show.
    c.setTransform(dpr, 0, 0, dpr, 0, 0);
    if (labels > 0) {
      c.font = mono(10.5, 500);
      c.fillStyle = `rgba(201,206,216,${(0.9 * labels).toFixed(3)})`;
      for (const st of data.stars) {
        const head = st.id === 'meissa' ? meissa : 1;
        if (head <= 0) continue;
        const [x, y] = screen(P(st.id));
        c.globalAlpha = head;
        c.fillText(st.n.toLocaleUpperCase(), x + 11, y + 4);
      }
      c.globalAlpha = 1;
    }
    if (guides > 0) {
      const [x, y] = screen([26 + 60, -13 - 30]);
      c.font = mono(11, 500);
      c.fillStyle = `rgba(${COOL},${guides.toFixed(3)})`;
      c.fillText('1 : 2', x + 8, y - 6);
    }
  }

  /** Right ascension and declination under a point on the screen. */
  function radec(sx: number, sy: number): string {
    // Screen → mark units → the sky's projection plane (undoing the fit) → the sphere.
    const u = (sx - Tx) / S;
    const v = (sy - Ty) / S;
    const [a, b] = data.fit;
    const d = a * a + b * b;
    const [ra, dec] = unproject((a * u + b * v) / d, (a * v - b * u) / d, data.c0);
    return formatRaDec(ra, dec);
  }

  function wrap(text: string, max: number): string[] {
    const c = ctx!;
    const out: string[] = [];
    let line = '';
    for (const word of text.split(' ')) {
      const next = line ? `${line} ${word}` : word;
      if (line && c.measureText(next).width > max) {
        out.push(line);
        line = word;
      } else {
        line = next;
      }
    }
    if (line) out.push(line);
    return out.slice(0, 4);
  }

  function drawReticle(T: number) {
    const c = ctx!;
    const morph = easeInOut(span(T, 1850, 2900));
    // Snap to one of Orion's stars near the pointer.
    let best: Star | null = null;
    let bestD = 30;
    for (const st of data.stars) {
      if (st.id === 'meissa' && morph > 0.5) continue;
      const [x, y] = screen(at(st, morph));
      const d = Math.hypot(x - mx, y - my);
      if (d < bestD) {
        bestD = d;
        best = st;
      }
    }
    if (best !== snapped) {
      snapped = best;
      c.font = mono(11);
      factLines = best ? wrap(best.f, 236) : [];
    }
    const target = snapped ? screen(at(snapped, morph)) : null;
    const x = target ? lerp(rx, target[0], 0.55) : rx;
    const y = target ? lerp(ry, target[1], 0.55) : ry;
    const R = snapped ? 17 : 11;

    c.setTransform(dpr, 0, 0, dpr, 0, 0);
    c.strokeStyle = 'rgba(244,245,247,0.75)';
    c.lineWidth = 1;
    c.beginPath();
    c.arc(x, y, R, 0, 6.2832);
    for (const [dx, dy] of [
      [1, 0],
      [-1, 0],
      [0, 1],
      [0, -1],
    ] as const) {
      c.moveTo(x + dx * (R + 4), y + dy * (R + 4));
      c.lineTo(x + dx * (R + 11), y + dy * (R + 11));
    }
    c.stroke();

    const lines: [string, string, string][] = snapped
      ? [
          [snapped.n, mono(12.5, 500), STARLIGHT],
          [snapped.c, mono(10.5), 'rgba(201,206,216,0.95)'],
          ...factLines.map((l): [string, string, string] => [l, mono(11), 'rgba(201,206,216,0.95)']),
        ]
      : [[radec(mx, my), mono(10.5), 'rgba(201,206,216,0.95)']];
    let wMax = 0;
    for (const [text, font] of lines) {
      c.font = font;
      wMax = Math.max(wMax, c.measureText(text).width);
    }
    const lh = 17;
    const boxW = wMax + 20;
    const boxH = lines.length * lh + 12;
    let bx = x + R + 18;
    if (bx + boxW > W - 12) bx = x - R - 18 - boxW;
    const by = clamp(y - 14, 12, H - boxH - 12);
    c.fillStyle = 'rgba(10,11,13,0.78)';
    c.beginPath();
    c.roundRect(bx, by, boxW, boxH, 8);
    c.fill();
    lines.forEach(([text, font, colour], i) => {
      c.font = font;
      c.fillStyle = colour;
      c.fillText(text, bx + 10, by + 18 + i * lh);
    });
  }

  function kick() {
    if (!raf && visible && !document.hidden) {
      last = performance.now();
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
    if (paused) kick();
  });
  pin.addEventListener('pointerleave', () => {
    over = false;
    tx = 0;
    ty = 0;
  });

  new IntersectionObserver(([entry]) => {
    visible = !!entry?.isIntersecting;
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
