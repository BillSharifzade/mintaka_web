// Initial particle state, shared by the GPU and CPU tiers: 8 floats per particle —
// position.xy, velocity.xy (document px), seed, phase, time in phase, spare.

import { FIELD_SHARE, LANES, VIEWBOX, bottomEdgeY, region } from './geometry.ts';

export const STRIDE = 8;

/** Where the logo's origin sits in the document, and its scale (px per logo unit). */
export interface Fig {
  ox: number;
  oy: number;
  scale: number;
}

function sample(want: 'top' | 'bottom' | 'channel', rand: () => number): [number, number] {
  for (let i = 0; i < 200; i++) {
    const x = VIEWBOX.x + rand() * VIEWBOX.w;
    const y = VIEWBOX.y + rand() * VIEWBOX.h;
    if (region(x, y) === want) return [x, y];
  }
  return [0, want === 'top' ? -60 : 60];
}

/** Fill the vessel as it would look mid-flow, so the very first frame already reads as the mark. */
export function seed(count: number, fig: Fig, rand: () => number = Math.random): Float32Array {
  const data = new Float32Array(count * STRIDE);
  for (let i = 0; i < count; i++) {
    const o = i * STRIDE;
    const s = rand();
    data[o + 4] = s;
    if (s < FIELD_SHARE) continue; // field stars are placed procedurally in the shader
    const r = rand();
    let x: number;
    let y: number;
    let phase: number;
    if (r < 0.44) {
      [x, y] = sample('top', rand);
      phase = 1;
    } else if (r < 0.48) {
      [x, y] = sample('channel', rand);
      phase = 2;
    } else if (r < 0.74) {
      [x, y] = sample('bottom', rand);
      phase = 3;
    } else if (r < 0.88) {
      const lane = LANES[Math.floor(rand() * 4)]!;
      x = lane + (rand() - 0.5) * 3;
      y = bottomEdgeY(lane) + rand() * 140;
      phase = 4;
    } else {
      const ang = -2.7 + rand() * 2.25;
      const rad = 95 + rand() * 70;
      x = -4 + Math.cos(ang) * rad;
      y = -62 + Math.sin(ang) * rad;
      phase = 0;
    }
    data[o] = fig.ox + x * fig.scale;
    data[o + 1] = fig.oy + y * fig.scale;
    data[o + 5] = phase;
    data[o + 6] = rand() * 2;
  }
  return data;
}
