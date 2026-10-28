// Tier C: Canvas 2D. The same rules as the WebGL2 shaders (shaders.ts), on the CPU, for a few
// thousand particles. Keep the two in step when changing the physics.

import type { Frame, Renderer } from './frame.ts';
import { FIELD_SHARE, HALF_CHANNEL, LANES, MINTAKA_S, NB, THREAT_SHARE, U, bottomEdgeY, fillTarget, sdTop, sdVessel, smoothstep } from './geometry.ts';
import { STRIDE } from './particles.ts';

const h1 = (n: number): number => {
  const x = Math.sin(n * 91.3458 + 17.17) * 47453.5453;
  return x - Math.floor(x);
};

const THREAT = FIELD_SHARE + THREAT_SHARE;
const COL = {
  starlight: [244, 245, 247],
  mist: [201, 206, 216],
  steel: [110, 118, 134],
  blue: [61, 123, 255],
  sky: [143, 179, 255],
  ember: [178, 58, 23],
  rust: [226, 85, 45],
} as const;

function mixc(a: readonly number[], b: readonly number[], t: number): [number, number, number] {
  return [a[0]! + (b[0]! - a[0]!) * t, a[1]! + (b[1]! - a[1]!) * t, a[2]! + (b[2]! - a[2]!) * t];
}

function curl(x: number, y: number, t: number): [number, number] {
  const a = 0.045, b = 0.061, c = 0.033;
  const dpx = a * Math.cos(x * a + t * 0.7) * Math.cos(y * b - t * 0.5) + 0.6 * c * Math.cos(x * c - y * a + t * 0.9);
  const dpy = -b * Math.sin(x * a + t * 0.7) * Math.sin(y * b - t * 0.5) - 0.6 * a * Math.cos(x * c - y * a + t * 0.9);
  return [dpy * 22, -dpx * 22];
}

function spawn(seed: number, t: number): [number, number] {
  const r1 = h1(seed * 13.1 + Math.floor(t * 0.37));
  const r2 = h1(seed * 7.7 + Math.floor(t * 0.53));
  const ang = -2.6 + r1 * 2.05;
  const rad = 72 + r2 * 46;
  return [-4 + Math.cos(ang) * rad, -62 + Math.sin(ang) * rad];
}

export function createCanvas(canvas: HTMLCanvasElement, init: Float32Array): Renderer | null {
  const ctx = canvas.getContext('2d');
  if (!ctx) return null;
  const s = new Float32Array(init);
  const n = s.length / STRIDE;
  let dpr = 1;

  function step(f: Frame): void {
    const { time, dt, fig, w } = f;
    for (let i = 0; i < n; i++) {
      const o = i * STRIDE;
      const seed = s[o + 4]!;
      if (seed < FIELD_SHARE) continue;
      let P0 = s[o]!, P1 = s[o + 1]!, V0 = s[o + 2]!, V1 = s[o + 3]!;
      let phase = s[o + 5]!;
      let life = s[o + 6]! + dt;
      const threat = seed < THREAT;
      const lane = LANES[Math.floor(h1(seed * 3.7) * 4)] ?? 0;
      let ax = 0, ay = 0;

      if (w.flow > 0.001) {
        const sc = fig.scale;
        let px = (P0 - fig.ox) / sc, py = (P1 - fig.oy) / sc;
        let vx = V0 / sc, vy = V1 / sc;
        let a0 = 0, a1 = 0;
        const d = NB[0] * px + NB[1] * py;
        if (phase < 0.5) {
          const [tx, ty] = fillTarget('top', h1(seed * 1.7), h1(seed * 2.9));
          const [nx, ny] = curl(px, py, time);
          a0 = (tx - px) * 2.6 - vx * 2.2 + nx * 1.2;
          a1 = (ty - py) * 2.6 - vy * 2.2 + ny * 1.2;
          if (sdTop(px, py) < 0 && d < -HALF_CHANNEL && Math.hypot(tx - px, ty - py) < 9) { phase = 1; life = 0; vx *= 0.5; vy *= 0.5; }
        } else if (phase < 1.5) {
          const [nx, ny] = curl(px, py, time);
          a0 = nx * 2.2 - vx * 2.1;
          a1 = 13 + ny * 2.2 - vy * 2.1;
          if (d > -HALF_CHANNEL) {
            if (threat) { phase = 5; life = 0; vx *= 0.1; vy *= 0.1; }
            else { phase = 2; life = 0; }
          }
        } else if (phase < 2.5) {
          a0 = (U[0] * 112 - vx) * 7 - NB[0] * d * 30;
          a1 = (U[1] * 112 - vy) * 7 - NB[1] * d * 30;
          if (U[0] * px + U[1] * py > MINTAKA_S + 2) { phase = 3; life = 0; }
        } else if (phase < 3.5) {
          if (life < 1.5) {
            const [tx, ty] = fillTarget('bottom', h1(seed * 3.1), h1(seed * 4.3));
            a0 = (tx - px) * 3 - vx * 2.4;
            a1 = (ty - py) * 3 - vy * 2.4;
          } else {
            const [nx, ny] = curl(px, py, time * 0.6);
            a0 = nx * 0.8 - vx * 1.2 + (lane - px) * 4 * smoothstep(70, 104, py);
            a1 = 15 + ny * 0.8 - vy * 1.6;
          }
          if (py > bottomEdgeY(px) - 1.5) { phase = 4; life = 0; }
        } else if (phase < 4.5) {
          a0 = (lane - px) * 9 - vx * 5;
          a1 = 62 - vy * 0.4;
          if (py > bottomEdgeY(lane) + 120 || life > 3.6) { phase = 0; life = 0; [px, py] = spawn(seed, time); vx = vy = 0; }
        } else {
          vx *= 0.82;
          vy *= 0.82;
          if (life > 1.1) { phase = 0; life = 0; [px, py] = spawn(seed, time); vx = vy = 0; }
        }
        if (phase > 0.5 && phase < 3.5) {
          const open = phase > 2.5;
          const sd = sdVessel(px, py, open);
          if (sd > -2) {
            const e = 0.6;
            let gx = (sdVessel(px + e, py, open) - sdVessel(px - e, py, open)) / (2 * e);
            let gy = (sdVessel(px, py + e, open) - sdVessel(px, py - e, open)) / (2 * e);
            const gl = Math.hypot(gx, gy) + 1e-5;
            gx /= gl;
            gy /= gl;
            a0 -= gx * (sd + 2) * 160;
            a1 -= gy * (sd + 2) * 160;
          }
        }
        P0 = fig.ox + px * sc;
        P1 = fig.oy + py * sc;
        V0 = vx * sc;
        V1 = vy * sc;
        ax += a0 * sc * w.flow;
        ay += a1 * sc * w.flow;
      }

      const wt = w.ray + w.orion;
      if (wt > 0.001) {
        let tx = 0, ty = 0;
        if (w.ray > 0) {
          const along = (h1(seed * 5.3) + time * (0.06 + 0.1 * h1(seed * 9.1))) % 1;
          const lens = 1 - Math.abs(2 * along - 1);
          const rx = f.ray[0] + along * f.ray[2];
          const ry = f.ray[1] + (h1(seed * 2.9) - 0.5) * 14 * lens;
          if (along < 0.04 && P0 > f.ray[0] + f.ray[2] * 0.75 && w.ray > 0.5) { P0 = rx; P1 = ry; V0 = f.ray[2] * 0.1; V1 = 0; }
          tx += rx * w.ray;
          ty += ry * w.ray;
        }
        if (w.orion > 0) {
          let ox: number, oy: number;
          if (h1(seed * 4.1) < 0.12) {
            const li = Math.floor(h1(seed * 6.3) * 8) * 4;
            const m = h1(seed * 8.7);
            ox = f.lines[li]! + (f.lines[li + 2]! - f.lines[li]!) * m;
            oy = f.lines[li + 1]! + (f.lines[li + 3]! - f.lines[li + 1]!) * m;
          } else {
            const r = h1(seed * 7.9);
            let j = 7;
            for (let k = 0; k < 8; k++) if (r <= f.cdf[k]!) { j = k; break; }
            const ang = h1(seed * 3.3) * 6.2832 + time * (0.12 + 0.2 * h1(seed * 5.9));
            const rad = f.orion[j * 3 + 2]! * (0.35 + 2.6 * h1(seed * 4.7) ** 2);
            ox = f.orion[j * 3]! + Math.cos(ang) * rad;
            oy = f.orion[j * 3 + 1]! + Math.sin(ang) * rad;
          }
          tx += ox * w.orion;
          ty += oy * w.orion;
        }
        tx /= wt;
        ty /= wt;
        const k = Math.min(wt, 1);
        ax += ((tx - P0) * 26 - V0 * 7) * k;
        ay += ((ty - P1) * 26 - V1 * 7) * k;
      }

      if (f.ptr[2] > 0) {
        const dx = P0 - f.ptr[0], dy = P1 - f.ptr[1];
        const r = Math.hypot(dx, dy) + 1e-3;
        if (r < 170) {
          const g = f.ptr[2] * 1100 * (1 - r / 170) ** 2;
          ax -= (dx / r) * g;
          ay -= (dy / r) * g;
        }
      }
      if (f.tap[3] > 0.5) {
        const age = time - f.tap[2];
        const dx = P0 - f.tap[0], dy = P1 - f.tap[1];
        const r = Math.hypot(dx, dy) + 1e-3;
        const band = Math.exp(-(((r - age * 720) / 42) ** 2)) * 3200 * Math.max(0, 1 - age / 1.3);
        ax += (dx / r) * band;
        ay += (dy / r) * band;
      }

      V0 += ax * dt;
      V1 += ay * dt;
      s[o] = P0 + V0 * dt;
      s[o + 1] = P1 + V1 * dt;
      s[o + 2] = V0;
      s[o + 3] = V1;
      s[o + 5] = phase;
      s[o + 6] = life;
    }
  }

  function paint(f: Frame): void {
    const [vw, vh] = f.view;
    ctx!.setTransform(dpr, 0, 0, dpr, 0, 0);
    ctx!.clearRect(0, 0, vw, vh);
    ctx!.globalCompositeOperation = 'lighter';
    const { w, fig } = f;
    for (let i = 0; i < n; i++) {
      const o = i * STRIDE;
      const seed = s[o + 4]!;
      let x: number, y: number, a: number, size: number;
      let col: readonly number[];
      if (seed < FIELD_SHARE) {
        const depth = 0.08 + 0.3 * h1(seed * 2.7);
        x = h1(seed * 1.1) * vw + f.parallax[0] * depth * 18;
        y = (((h1(seed * 2.3) - (f.scroll[1] * depth) / vh) % 1) + 1) % 1 * vh + f.parallax[1] * depth * 18;
        const tw = 0.55 + 0.45 * Math.sin(f.time * (0.6 + h1(seed * 4.4) * 1.6) + seed * 60);
        col = mixc(COL.steel, COL.mist, h1(seed * 3.1));
        a = (0.18 + 0.5 * h1(seed * 5.5)) * tw;
        size = 1 + 1.4 * h1(seed * 6.6) ** 3;
      } else {
        x = s[o]! - f.scroll[0];
        y = s[o + 1]! - f.scroll[1];
        if (x < -4 || y < -4 || x > vw + 4 || y > vh + 4) continue;
        const phase = s[o + 5]!, life = s[o + 6]!;
        size = 1.6;
        if (phase < 0.5) { col = COL.mist; a = 0.4; }
        else if (phase < 1.5) { col = COL.starlight; a = 0.6; }
        else if (phase < 2.5) {
          const px = (s[o]! - fig.ox) / fig.scale, py = (s[o + 1]! - fig.oy) / fig.scale;
          col = mixc(COL.starlight, COL.blue, smoothstep(-30, MINTAKA_S, U[0] * px + U[1] * py));
          a = 0.85;
        } else if (phase < 3.5) { col = COL.blue; a = 0.7; }
        else if (phase < 4.5) { col = COL.sky; a = 0.75 * (1 - smoothstep(0.3, 2.6, life)); }
        else { col = COL.ember; a = 0.9 * (1 - smoothstep(0.5, 1.1, life)); size += 2.5 * (1 - smoothstep(0.05, 0.6, life)); }
        if (seed < THREAT && phase < 1.5) col = COL.ember;
        a *= f.flowAlpha + (1 - f.flowAlpha) * (1 - w.flow);
        if (w.ray > 0) col = mixc(col, COL.sky, w.ray);
        if (w.orion > 0) col = mixc(col, COL.starlight, w.orion);
      }
      a *= w.alpha;
      if (a < 0.02) continue;
      ctx!.globalAlpha = Math.min(1, a);
      ctx!.fillStyle = `rgb(${col[0]! | 0},${col[1]! | 0},${col[2]! | 0})`;
      ctx!.fillRect(x - size / 2, y - size / 2, size, size);
    }
    ctx!.globalCompositeOperation = 'source-over';
    for (let i = 0; i < 8; i++) {
      const r = f.stars[i * 4 + 2]!, a = f.stars[i * 4 + 3]!;
      if (a < 0.01 || r < 0.5) continue;
      ctx!.globalAlpha = a;
      ctx!.fillStyle = `rgb(${(f.starCol[i * 3]! * 255) | 0},${(f.starCol[i * 3 + 1]! * 255) | 0},${(f.starCol[i * 3 + 2]! * 255) | 0})`;
      ctx!.beginPath();
      ctx!.arc(f.stars[i * 4]!, f.stars[i * 4 + 1]!, r, 0, Math.PI * 2);
      ctx!.fill();
    }
    ctx!.globalAlpha = 1;
  }

  const r: Renderer = {
    tier: 'C',
    sample: null,
    resize(width, height, d) {
      dpr = d;
      if (canvas.width !== width || canvas.height !== height) {
        canvas.width = width;
        canvas.height = height;
      }
    },
    draw(f) {
      const t0 = performance.now();
      step(f);
      paint(f);
      r.sample?.(performance.now() - t0);
    },
    clear() {
      ctx.setTransform(1, 0, 0, 1, 0, 0);
      ctx.clearRect(0, 0, canvas.width, canvas.height);
    },
    destroy() {
      this.clear();
    },
  };
  return r;
}

