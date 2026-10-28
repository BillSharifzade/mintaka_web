// The hero engine: the Mintaka mark as a working machine. Requests pour into the top of the
// hourglass, squeeze through the belt — where they speed up and turn Mintaka blue — and leave the
// bottom in four ordered streams. Scrolling turns the streams into a ray of light (S2), then
// lets the particles settle into the real Orion (S3), where the logo's stars take their places
// in the sky.
//
// Tiers: A WebGPU (compute) → B WebGL2 (transform feedback) → C Canvas 2D → D the static poster
// already in the page. Every tier simulates the same physics; A and B run it on the GPU.
// Reduced motion and Data Saver go straight to D. Everything here is decorative: the canvas is
// aria-hidden and all meaning lives in the text.

import { emit, on, type Tier } from '../shared/bus.ts';
import { reducedMotion, saveData } from '../shared/env.ts';
import { LOGO, VIEWBOX, progress, weights } from './geometry.ts';
import type { Frame, Renderer } from './frame.ts';
import { seed, type Fig } from './particles.ts';

/** orion.rs order: the uniforms, the SVG circles and the colour picks all use it. */
const STARS = ['betelgeuse', 'meissa', 'bellatrix', 'mintaka', 'alnilam', 'alnitak', 'saiph', 'rigel'] as const;
const LINES: [number, number][] = [
  [0, 1], [1, 2], [2, 3], [3, 7], [0, 5], [5, 6], [5, 4], [4, 3],
];
/** Radius in the logo (0 = not drawn as a star there). */
const LOGO_R: Record<string, number> = { betelgeuse: 20, mintaka: 14, alnilam: 6, alnitak: 6 };
const RUST = [0.886, 0.333, 0.176];
const BLUE = [0.239, 0.482, 1.0];
const WHITE = [0.957, 0.961, 0.969];

interface Anchors {
  fig: Fig;
  ray: [number, number, number, number];
  orion: Float32Array;
  cdf: Float32Array;
  lines: Float32Array;
  keys: [number, number, number, number];
}

function docRect(el: Element): DOMRect {
  const r = el.getBoundingClientRect();
  return new DOMRect(r.left + scrollX, r.top + scrollY, r.width, r.height);
}

function measure(): Anchors | null {
  const figEl = document.querySelector('[data-anchor="hourglass"]');
  const rayEl = document.querySelector('[data-anchor="ray"]');
  const orionEl = document.querySelector('[data-anchor="orion"]');
  if (!figEl || !rayEl || !orionEl) return null;
  const f = docRect(figEl);
  const scale = f.width / VIEWBOX.w;
  const fig = { ox: f.x - VIEWBOX.x * scale, oy: f.y - VIEWBOX.y * scale, scale };
  const r = docRect(rayEl);
  const orion = new Float32Array(24);
  const weightsR: number[] = [];
  STARS.forEach((id, i) => {
    const c = orionEl.querySelector(`[data-star="${id}"]`);
    const b = c ? docRect(c) : new DOMRect();
    orion[i * 3] = b.x + b.width / 2;
    orion[i * 3 + 1] = b.y + b.height / 2;
    orion[i * 3 + 2] = Math.max(1.5, b.width / 2);
    weightsR.push(Math.pow(Math.max(1.5, b.width / 2), 1.4));
  });
  const total = weightsR.reduce((a, b) => a + b, 0);
  const cdf = new Float32Array(8);
  let acc = 0;
  weightsR.forEach((w, i) => (cdf[i] = acc += w / total));
  cdf[7] = 1;
  const lines = new Float32Array(32);
  LINES.forEach(([a, b], i) => lines.set([orion[a * 3]!, orion[a * 3 + 1]!, orion[b * 3]!, orion[b * 3 + 1]!], i * 4));
  const o = docRect(orionEl);
  const vh = innerHeight;
  const k1 = Math.max(1, r.y - vh * 0.55);
  const k2 = Math.max(k1 + 1, o.y + o.height / 2 - vh * 0.5);
  const k3 = Math.max(k2 + 1, o.y + o.height);
  return { fig, ray: [r.x, r.y + r.height / 2, r.width, r.height], orion, cdf, lines, keys: [0, k1, k2, k3] };
}

export async function start(first: HTMLCanvasElement): Promise<void> {
  let canvas = first;
  if (reducedMotion() || saveData()) {
    emit('tier', 'D');
    return;
  }
  let anchors = measure();
  if (!anchors) {
    emit('tier', 'D');
    return;
  }
  const narrow = matchMedia('(max-width: 52rem)').matches;
  const coarse = matchMedia('(pointer: coarse)').matches;
  const lowEnd = (navigator.hardwareConcurrency || 4) <= 4;
  const dprCap = narrow || lowEnd ? 1.5 : 2;

  // Steps down, in order, when frames can't keep up: fewer particles first, then the CPU,
  // then the poster. A canvas keeps its first context type, so every step gets a fresh one.
  const hasGpu = 'gpu' in navigator;
  const levels: { tier: 'A' | 'B' | 'C'; count: number }[] = [
    ...(hasGpu ? [{ tier: 'A' as const, count: narrow || coarse ? 30_000 : lowEnd ? 60_000 : 120_000 }] : []),
    ...(narrow || coarse
      ? [{ tier: 'B' as const, count: 12_000 }, { tier: 'B' as const, count: 5_000 }, { tier: 'C' as const, count: 1_500 }]
      : [{ tier: 'B' as const, count: lowEnd ? 22_000 : 40_000 }, { tier: 'B' as const, count: 12_000 }, { tier: 'C' as const, count: 3_000 }]),
  ];
  let level = 0;
  let renderer: Renderer | null = null;

  async function build(from: number): Promise<Renderer | null> {
    for (let i = from; i < levels.length; i++) {
      const { tier, count } = levels[i]!;
      if (renderer) {
        const fresh = canvas.cloneNode() as HTMLCanvasElement;
        canvas.replaceWith(fresh);
        canvas = fresh;
      }
      const init = seed(count, anchors!.fig);
      const lost = () => void downgrade('context lost');
      // Each tier is its own chunk: a visitor downloads only the renderer their browser runs.
      const r =
        tier === 'A'
          ? await (await import('./webgpu.ts')).createWebGPU(canvas, init, lost)
          : tier === 'B'
            ? (await import('./webgl.ts')).createWebGL(canvas, init, lost)
            : (await import('./canvas.ts')).createCanvas(canvas, init);
      if (r) {
        level = i;
        return r;
      }
    }
    return null;
  }

  renderer = await build(0);
  if (!renderer) {
    emit('tier', 'D');
    return;
  }

  let running = false;
  let paused = false;
  let raf = 0;
  let last = 0;
  let time = 0;
  let cleared = true;
  let shown = false;
  const ptr: [number, number, number] = [0, 0, 0];
  let ptrTarget = 0;
  const parallax: [number, number] = [0, 0];
  const parallaxTarget: [number, number] = [0, 0];
  const tap: [number, number, number, number] = [0, 0, -10, 0];
  // The spec's rule (§S1): drop a tier when the 90th-percentile frame time exceeds 12 ms on
  // desktop or 20 ms on mobile. "Frame time" is our own work, reported by the renderer.
  const budgetMs = narrow || coarse ? 20 : 12;
  const costs: number[] = [];
  let sampled = 0;

  const stars = new Float32Array(32);
  const starCol = new Float32Array(24);
  STARS.forEach((id, i) => starCol.set(id === 'betelgeuse' ? RUST : id === 'mintaka' ? BLUE : WHITE, i * 3));

  function size(): void {
    const dpr = Math.min(devicePixelRatio || 1, dprCap);
    renderer?.resize(Math.round(innerWidth * dpr), Math.round(innerHeight * dpr), dpr);
  }

  let downgrading = false;
  async function downgrade(why: string): Promise<void> {
    if (downgrading || !renderer) return;
    downgrading = true;
    const from = `${renderer.tier}·${levels[level]!.count}`;
    stop();
    renderer.destroy();
    const next = await build(level + 1);
    renderer = next;
    costs.length = 0;
    sampled = 0;
    downgrading = false;
    console.info(`[mintaka] hero: ${from} → ${next ? `${next.tier}·${levels[level]!.count}` : 'D'} (${why})`);
    if (!next) {
      stop();
      document.documentElement.classList.remove('engine-on');
      emit('tier', 'D');
      return;
    }
    size();
    emit('tier', next.tier as Tier);
    run();
  }

  function frame(now: number, fixedDt?: number): void {
    raf = 0;
    if (!running || !renderer || !anchors) return;
    const dt = fixedDt ?? Math.min(1 / 30, last ? (now - last) / 1000 : 1 / 60);
    last = now;
    // Sample real animation frames 30–120 of each tier (after shaders compile and caches warm).
    if (fixedDt === undefined) {
      sampled++;
      if (sampled === 30) renderer.sample = (ms) => costs.push(ms);
      if (sampled === 120) renderer.sample = null;
      if (sampled > 120 && costs.length >= 60 && probe()) return;
    }
    time += dt;

    const t = progress(scrollY, anchors.keys);
    if (t >= 3) {
      if (!cleared) renderer.clear();
      cleared = true;
      running = false;
      return;
    }
    cleared = false;
    const w = weights(t);
    ptr[2] += (ptrTarget - ptr[2]) * Math.min(1, dt * 6);
    parallax[0] += (parallaxTarget[0] - parallax[0]) * Math.min(1, dt * 3);
    parallax[1] += (parallaxTarget[1] - parallax[1]) * Math.min(1, dt * 3);
    if (tap[3] && time - tap[2] > 1.4) tap[3] = 0;

    // The logo's stars travel to their real places in the sky.
    const { fig, orion } = anchors;
    // The named stars step aside for the ray of light, then reappear in the sky.
    const dip = 1 - 0.88 * Math.max(0, 1 - Math.abs(t - 1.05) / 0.6);
    STARS.forEach((id, i) => {
      const lp = LOGO[id];
      const lx = lp ? fig.ox + lp[0] * fig.scale : orion[i * 3]!;
      const ly = lp ? fig.oy + lp[1] * fig.scale : fig.oy - 118 * fig.scale;
      const lr = (LOGO_R[id] ?? 0) * fig.scale;
      const m = w.morph;
      const x = lx + (orion[i * 3]! - lx) * m;
      const y = ly + (orion[i * 3 + 1]! - ly) * m;
      const r = lr + (orion[i * 3 + 2]! - lr) * m;
      const a0 = LOGO_R[id] ? 1 : 0;
      const a = (a0 + (1 - a0) * m) * dip * w.alpha;
      stars.set([x - scrollX, y - scrollY, r, a], i * 4);
    });

    const f: Frame = {
      time,
      dt,
      view: [innerWidth, innerHeight],
      scroll: [scrollX, scrollY],
      dpr: Math.min(devicePixelRatio || 1, dprCap),
      fig,
      w,
      ray: anchors.ray,
      orion,
      cdf: anchors.cdf,
      lines: anchors.lines,
      ptr,
      tap,
      parallax,
      flowAlpha: narrow ? 0.5 : 1,
      stars,
      starCol,
    };
    renderer.draw(f);
    if (!shown) {
      shown = true;
      document.documentElement.classList.add('engine-on');
    }
    if (fixedDt === undefined) raf = requestAnimationFrame(frame);
  }

  /** Step down if our own frame time is over budget. Returns true when it does. */
  function probe(): boolean {
    const sorted = costs.splice(0).sort((a, b) => a - b);
    const p90 = sorted[Math.floor(sorted.length * 0.9)] ?? 0;
    if (MINTAKA_DEV) console.info(`[mintaka] hero: tier ${renderer?.tier}·${levels[level]!.count}, p90 frame time ${p90.toFixed(2)} ms (budget ${budgetMs} ms)`);
    if (p90 <= budgetMs) return false;
    void downgrade(`90th-percentile frame time ${p90.toFixed(1)} ms > ${budgetMs} ms`);
    return true;
  }

  function run(): void {
    if (running || paused || document.hidden || !renderer) return;
    running = true;
    last = 0;
    raf = requestAnimationFrame(frame);
  }

  function stop(): void {
    running = false;
    if (raf) cancelAnimationFrame(raf);
    raf = 0;
  }

  addEventListener('scroll', run, { passive: true });
  addEventListener('resize', () => {
    anchors = measure() ?? anchors;
    size();
    run();
  });
  new ResizeObserver(() => {
    anchors = measure() ?? anchors;
  }).observe(document.body);
  void document.fonts?.ready.then(() => (anchors = measure() ?? anchors));
  document.addEventListener('visibilitychange', () => (document.hidden ? stop() : run()));

  addEventListener(
    'pointermove',
    (e) => {
      parallaxTarget[0] = (e.clientX / innerWidth) * 2 - 1;
      parallaxTarget[1] = (e.clientY / innerHeight) * 2 - 1;
      if (e.pointerType !== 'mouse') return;
      ptr[0] = e.clientX + scrollX;
      ptr[1] = e.clientY + scrollY;
      ptrTarget = 1;
    },
    { passive: true },
  );
  document.documentElement.addEventListener('pointerleave', () => (ptrTarget = 0));
  addEventListener(
    'pointerdown',
    (e) => {
      if (e.pointerType === 'mouse') return;
      tap[0] = e.clientX + scrollX;
      tap[1] = e.clientY + scrollY;
      tap[2] = time;
      tap[3] = 1;
    },
    { passive: true },
  );

  on('sky:toggle', () => {
    paused = !paused;
    if (paused) {
      stop();
      renderer?.clear();
      cleared = true;
      shown = false;
      document.documentElement.classList.remove('engine-on');
    } else {
      run();
    }
    emit('sky', !paused);
  });

  size();
  emit('tier', renderer.tier as Tier);
  emit('sky', true);
  run();

  // Development builds only: advance the simulation by N seconds at 60 Hz, synchronously —
  // for reviewing the choreography in automated or throttled browsers.
  if (MINTAKA_DEV) {
    (window as unknown as Record<string, unknown>)['__hero'] = {
      advance(seconds: number) {
        const was = running;
        running = true;
        for (let i = 0; i < Math.round(seconds * 60); i++) frame(performance.now(), 1 / 60);
        running = was;
        return renderer?.tier ?? 'D';
      },
    };
  }
}
