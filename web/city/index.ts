// The lab's opening experiment: Dushanbe's 8,393 intersections, searched live in the visitor's
// browser. Searches run on their own — Dijkstra spreading in rings, A* reaching like a cone — and
// each is drawn the way it actually worked: every intersection it settled, in the order it settled
// them, then the route it found. Pointing at the map routes from the pointer, every time the
// nearest intersection changes; a click moves the destination. Every number shown is measured.
//
// Three canvases: the streets (drawn once), the searches (drawn incrementally and fading), the
// routes (redrawn each frame — a few lines). It sleeps when off screen, when the tab is hidden,
// when paused; with reduced motion it draws the streets and routes from the pointer only.

import { emit, on } from '../shared/bus.ts';
import { assetUrl } from '../shared/assets.ts';
import { reducedMotion, saveData } from '../shared/env.ts';
import { numberLocale, t } from '../shared/i18n.ts';
import { decode, Nearest, Search, strongly, type City, type Found } from './graph.ts';

const BLUE = '61 123 255';
const SKY = '143 179 255';

interface Job {
  f: Found;
  astar: boolean;
  shown: number;
  step: number;
  done: number;
}

export async function start(root: HTMLElement): Promise<void> {
  const map = root.querySelector<HTMLElement>('.lab-hero__map');
  if (!map) return;
  if (saveData()) return void emit('tier', 'D');
  let c: City;
  try {
    const res = await fetch(assetUrl('city.bin'));
    if (!res.ok) throw new Error(String(res.status));
    c = decode(await res.arrayBuffer());
  } catch (err) {
    console.info('[mintaka] city:', err);
    return void emit('tier', 'D');
  }
  const still = reducedMotion();

  // Routes between any two of these intersections exist (one-way streets leave a few that don't).
  let hub = 0;
  let hd = Infinity;
  for (let k = 0; k < c.n; k++) {
    const d = Math.hypot(c.x[k]! - c.w * 0.5, c.y[k]! - c.h * 0.5);
    if (d < hd) (hd = d), (hub = k);
  }
  const ok = strongly(c, hub);
  const pool: number[] = [];
  for (let k = 0; k < c.n; k++) if (ok[k]) pool.push(k);
  const near = new Nearest(c, ok);
  const search = new Search(c);
  let target = near.find(c.w * 0.58, c.h * 0.46);

  // ── Canvases ──
  const layer = (cls: string) => {
    const cv = document.createElement('canvas');
    cv.className = cls;
    cv.setAttribute('aria-hidden', 'true');
    map.append(cv);
    return cv.getContext('2d')!;
  };
  const base = layer('lab-hero__streets');
  const glow = layer('lab-hero__waves');
  const top = layer('lab-hero__routes');
  let cw = 0;
  let ch = 0;
  let dpr = 1;
  let s = 1;
  let ox = 0;
  let oy = 0;
  const X = (mx: number) => ox + mx * s;
  const Y = (my: number) => oy + my * s;

  const street = (p: Path2D, a: number) => {
    const st = c.street[a]!;
    const id = st < 0 ? ~st : st;
    const lo = c.pOff[id]!;
    const hi = c.pOff[id + 1]!;
    p.moveTo(X(c.px[lo]!), Y(c.py[lo]!));
    for (let j = lo + 1; j < hi; j++) p.lineTo(X(c.px[j]!), Y(c.py[j]!));
  };

  function fit(): void {
    const r = map!.getBoundingClientRect();
    cw = r.width;
    ch = r.height;
    dpr = Math.min(devicePixelRatio || 1, cw < 700 ? 1.5 : 2);
    for (const ctx of [base, glow, top]) {
      ctx.canvas.width = Math.round(cw * dpr);
      ctx.canvas.height = Math.round(ch * dpr);
      ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
      ctx.lineCap = 'round';
      ctx.lineJoin = 'round';
    }
    // Cover the hero with the city, its dense middle in the middle.
    s = Math.max(cw / c.w, ch / c.h) * 1.12;
    ox = (cw - c.w * s) / 2;
    oy = (ch - c.h * s) / 2;
    const p = new Path2D();
    for (let id = 0; id < c.pOff.length - 1; id++) {
      const lo = c.pOff[id]!;
      p.moveTo(X(c.px[lo]!), Y(c.py[lo]!));
      for (let j = lo + 1; j < c.pOff[id + 1]!; j++) p.lineTo(X(c.px[j]!), Y(c.py[j]!));
    }
    base.clearRect(0, 0, cw, ch);
    base.strokeStyle = 'rgb(201 206 216 / 0.32)';
    base.lineWidth = 0.8;
    base.stroke(p);
    glow.clearRect(0, 0, cw, ch);
  }

  // ── Readouts ──
  const nf = (v: number, d = 0) => v.toLocaleString(numberLocale(), { minimumFractionDigits: d, maximumFractionDigits: d });
  const ms = (v: number) => nf(v, v < 1 ? 2 : 1);
  const km = (m: number) => nf(m / 1000, 1);
  const out = (k: string) => root.querySelector<HTMLElement>(`[data-c="${k}"]`);
  const log = root.querySelector<HTMLOListElement>('.lab-hero__log');
  const probe = out('probe');
  const times: Record<'a' | 'd', number[]> = { a: [], d: [] };
  let searches = 0;
  let settledTotal = 0;
  let frameMs = 0;
  const median = (v: number[]) => {
    if (!v.length) return 0;
    const sorted = [...v].sort((p, q) => p - q);
    return sorted[sorted.length >> 1]!;
  };
  let painted = 0;
  function paint(now: number): void {
    if (now - painted < 250) return;
    painted = now;
    const set = (k: string, v: string) => {
      const el = out(k);
      if (el && el.textContent !== v) el.textContent = v;
    };
    set('searches', nf(searches));
    set('astar', times.a.length ? ms(median(times.a)) : '—');
    set('dijkstra', times.d.length ? ms(median(times.d)) : '—');
    set('settled', nf(settledTotal));
    set('frame', frameMs ? ms(frameMs) : '—');
  }
  function note(f: Found, astar: boolean): void {
    searches++;
    settledTotal += f.settled;
    const v = times[astar ? 'a' : 'd'];
    v.push(f.ms);
    if (v.length > 60) v.shift();
    if (!log) return;
    const li = document.createElement('li');
    li.textContent = t('city-line', { algo: t(astar ? 'city-astar' : 'city-dijkstra'), km: km(f.dist), n: nf(f.settled), ms: ms(f.ms) });
    log.prepend(li);
    while (log.children.length > 4) log.lastElementChild!.remove();
  }

  // ── Searches ──
  const jobs: Job[] = [];
  let astarNext = false;
  function run(from: number, to: number, astar: boolean, animate: boolean): Found {
    const f = search.run(from, to, astar);
    note(f, astar);
    if (animate) jobs.push({ f, astar, shown: 0, step: Math.max(24, Math.ceil(f.settled / 48)), done: 0 });
    return f;
  }
  function ambient(): void {
    for (let tries = 0; tries < 24; tries++) {
      const a = pool[(Math.random() * pool.length) | 0]!;
      const b = pool[(Math.random() * pool.length) | 0]!;
      // Long enough to be worth watching, on screen at both ends.
      const d = Math.hypot(c.x[a]! - c.x[b]!, c.y[a]! - c.y[b]!) * s;
      // Where the words don't cover the map: the right side on wide screens, the top on narrow.
      const wide = cw > ch;
      const onScreen = (k: number) => {
        const sx = X(c.x[k]!);
        const sy = Y(c.y[k]!);
        // Narrow screens give the map its own window (hero.css), so all of it is open.
        return wide ? sx > cw * 0.42 && sx < cw - 24 && sy > 24 && sy < ch * 0.82 : sx > 16 && sx < cw - 16 && sy > 16 && sy < ch - 24;
      };
      // Far enough apart on screen to be worth watching.
      if (d < Math.min(cw, ch) * 0.35 || !onScreen(a) || !onScreen(b)) continue;
      run(a, b, astarNext, true);
      astarNext = !astarNext;
      return;
    }
  }

  // ── The pointer ──
  let px = -1;
  let py = -1;
  let from = -1;
  let probed: Found | null = null;
  const toMap = (e: PointerEvent): [number, number] => {
    const r = map.getBoundingClientRect();
    return [(e.clientX - r.left - ox) / s, (e.clientY - r.top - oy) / s];
  };
  root.addEventListener('pointermove', (e) => {
    if (e.pointerType !== 'mouse') return;
    [px, py] = toMap(e);
    wake();
  }, { passive: true });
  root.addEventListener('pointerleave', () => {
    px = py = -1;
    from = -1;
    probed = null;
    if (probe) probe.textContent = '';
    wake();
  });
  root.addEventListener('click', (e) => {
    if ((e.target as Element).closest('a, button')) return;
    const [mx, my] = toMap(e as PointerEvent);
    const k = near.find(mx, my);
    if ((e as PointerEvent).pointerType === 'mouse' || !(e as PointerEvent).pointerType) {
      // A click moves the destination; the pointer's route follows at once.
      target = k;
      from = -1;
    } else {
      // A tap searches from there, and shows the search.
      run(k, target, true, !still);
    }
    wake();
  });
  function follow(): void {
    if (px < 0) return;
    const k = near.find(px, py);
    if (k === from) return;
    from = k;
    probed = search.run(k, target, true);
    if (probe) probe.textContent = t('city-probe', { km: km(probed.dist), n: nf(probed.settled), ms: ms(probed.ms) });
  }

  // ── Frames ──
  const line = (ctx: CanvasRenderingContext2D, arcs: number[], rgb: string, alpha: number, width: number) => {
    if (!arcs.length || alpha <= 0) return;
    const p = new Path2D();
    for (const a of arcs) street(p, a);
    ctx.strokeStyle = `rgb(${rgb} / ${alpha * 0.25})`;
    ctx.lineWidth = width + 4;
    ctx.stroke(p);
    ctx.strokeStyle = `rgb(${rgb} / ${alpha})`;
    ctx.lineWidth = width;
    ctx.stroke(p);
  };
  const dot = (k: number, fill: string, r: number, ring = false) => {
    top.beginPath();
    top.arc(X(c.x[k]!), Y(c.y[k]!), r, 0, Math.PI * 2);
    if (ring) {
      top.strokeStyle = fill;
      top.lineWidth = 1.5;
      top.stroke();
    } else {
      top.fillStyle = fill;
      top.fill();
    }
  };

  let first: Found | null = null;
  let raf = 0;
  let visible = true;
  let paused = false;
  let lastAmbient = 0;
  function frame(now: number): void {
    raf = 0;
    const t0 = performance.now();
    // The searches: fade what was drawn, then draw what each search settled since last frame.
    glow.globalCompositeOperation = 'destination-out';
    glow.fillStyle = 'rgb(0 0 0 / 0.02)';
    glow.fillRect(0, 0, cw, ch);
    glow.globalCompositeOperation = 'lighter';
    for (const j of jobs) {
      if (j.shown >= j.f.settled) continue;
      const end = Math.min(j.f.settled, j.shown + j.step);
      const p = new Path2D();
      for (let k = Math.max(1, j.shown); k < end; k++) street(p, j.f.tree[k]!);
      // The frontier: a wide soft stroke, then the streets themselves, bright.
      glow.strokeStyle = `rgb(${j.astar ? SKY : BLUE} / 0.22)`;
      glow.lineWidth = 4;
      glow.stroke(p);
      glow.strokeStyle = `rgb(${j.astar ? SKY : BLUE} / 0.95)`;
      glow.lineWidth = 1.3;
      glow.stroke(p);
      j.shown = end;
      if (end >= j.f.settled) j.done = now;
    }
    glow.globalCompositeOperation = 'source-over';
    // The routes: each shows once its search has finished, then fades.
    top.clearRect(0, 0, cw, ch);
    for (let i = jobs.length - 1; i >= 0; i--) {
      const j = jobs[i]!;
      if (!j.done) continue;
      const age = (now - j.done) / 1000;
      if (age > 2.6) {
        jobs.splice(i, 1);
        continue;
      }
      const alpha = age < 1.6 ? 1 : 1 - (age - 1.6);
      line(top, j.f.path, '244 245 247', alpha, 1.75);
      if (j.f.path.length) {
        dot(c.from[j.f.path[0]!]!, `rgb(226 85 45 / ${alpha})`, 3.5);
        dot(c.to[j.f.path[j.f.path.length - 1]!]!, `rgb(244 245 247 / ${alpha})`, 4.5, true);
      }
    }
    follow();
    if (probed && px >= 0) {
      line(top, probed.path, SKY, 1, 2);
      dot(from, 'rgb(143 179 255)', 3.5);
    }
    dot(target, 'rgb(244 245 247)', 5.5, true);
    dot(target, 'rgb(244 245 247)', 1.5);
    if (still && first) line(top, first.path, '244 245 247', 0.9, 1.75);
    if (!still && !paused && now - lastAmbient > 1100 && jobs.length < 4) {
      lastAmbient = now;
      ambient();
    }
    frameMs = frameMs ? frameMs * 0.9 + (performance.now() - t0) * 0.1 : performance.now() - t0;
    paint(now);
    if (running()) raf = requestAnimationFrame(frame);
  }
  const running = () => visible && !document.hidden && (!paused || px >= 0 || jobs.length > 0) && (!still || px >= 0);
  function wake(): void {
    if (!raf && visible && !document.hidden) raf = requestAnimationFrame(frame);
  }

  fit();
  new ResizeObserver(() => {
    fit();
    wake();
  }).observe(map);
  new IntersectionObserver(([e]) => {
    visible = !!e?.isIntersecting;
    if (visible) wake();
  }).observe(root);
  document.addEventListener('visibilitychange', wake);
  on('sky:toggle', () => {
    paused = !paused;
    emit('sky', !paused);
    wake();
  });
  map.classList.add('is-on');
  emit('tier', still ? 'D' : 'C');
  if (!still) emit('sky', true);
  // One route to begin with, so the map is never just streets.
  first = run(near.find(c.w * 0.36, c.h * 0.6), target, false, !still);
  raf = requestAnimationFrame(frame);
}
