// What every renderer receives each frame. Positions are in document CSS pixels, so the
// hourglass scrolls with the page; `scroll` turns them into screen positions.

import type { Weights } from './geometry.ts';
import type { Fig } from './particles.ts';

export interface Frame {
  time: number;
  dt: number;
  view: [number, number];
  scroll: [number, number];
  dpr: number;
  fig: Fig;
  w: Weights;
  ray: [number, number, number, number];
  /** Eight stars of Orion in S3: x, y, r (document px), in orion.rs order. */
  orion: Float32Array;
  cdf: Float32Array;
  /** Eight constellation lines: x1, y1, x2, y2 (document px). */
  lines: Float32Array;
  ptr: [number, number, number];
  tap: [number, number, number, number];
  parallax: [number, number];
  /** Dimmer hourglass on small screens, where it sits behind the text. */
  flowAlpha: number;
  /** Eight named-star sprites: screen x, y, radius, alpha. */
  stars: Float32Array;
  starCol: Float32Array;
}

export interface Renderer {
  readonly tier: 'A' | 'B' | 'C';
  /**
   * While set, the renderer reports how long each frame's own work took, in ms — GPU work for
   * A and B (measured only while sampling: it costs a little), CPU work for C. This, not the gap
   * between animation frames, is what the tier probe judges: browsers throttle frames for many
   * reasons that have nothing to do with us.
   */
  sample: ((ms: number) => void) | null;
  resize(width: number, height: number, dpr: number): void;
  draw(frame: Frame): void;
  clear(): void;
  destroy(): void;
}
