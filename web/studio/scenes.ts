/**
 * One scroll loop for the studio page. The browser scrolls natively (no scroll hijacking: find in
 * page, keyboard, anchors and assistive technology all behave); the pinned scenes are CSS
 * `position: sticky`, and this loop only reads the scroll position once a frame and turns it into
 * each scene's progress through its track.
 *
 * Also here: the header takes the colourway of the chapter beneath it (a scene can override it
 * while a part of the mark is filling the screen), hides while the reader scrolls down, and the
 * footer's wordmark rises as the page slides off it.
 */

import { clamp, easeIn, lerp } from './math.ts';
import type { Sky } from './sky.ts';

export interface Scene {
  el: HTMLElement;
  /** Progress through the scene's track, 0..1 (−1 until first measured). */
  p: number;
  top: number;
  len: number;
  measure(): void;
  update(p: number): void;
  /** The colourway the scene shows at progress `p`, when it overrides its chapter's. */
  theme?(p: number): string | undefined;
}

const base = (el: HTMLElement): Pick<Scene, 'el' | 'p' | 'top' | 'len'> => ({ el, p: -1, top: 0, len: 1 });

export function run(header: HTMLElement | null, scenes: Scene[]): void {
  const chapters = [...document.querySelectorAll<HTMLElement>('main > .ch')];
  const footer = document.querySelector<HTMLElement>('.ft');
  const wordmark = footer?.querySelector<HTMLElement>('.ft__mark') ?? null;
  let edges: [number, number][] = [];
  let vh = innerHeight;
  let docH = 0;
  let footH = 0;
  let lastY = scrollY;
  let hidden = false;
  let queued = false;

  const tick = () => {
    queued = false;
    const y = scrollY;
    for (const s of scenes) {
      const p = clamp((y - s.top) / s.len);
      if (p !== s.p) {
        s.p = p;
        s.update(p);
      }
    }
    if (header) {
      const line = y + 32;
      let theme = 'void';
      for (let i = 0; i < chapters.length; i++) {
        const [a, b] = edges[i] ?? [0, 0];
        if (line >= a && line < b) {
          const ch = chapters[i]!;
          const scene = scenes.find((s) => s.el === ch);
          theme = scene?.theme?.(scene.p) ?? ch.dataset['theme'] ?? theme;
          break;
        }
      }
      if (header.dataset['theme'] !== theme) header.dataset['theme'] = theme;
      header.classList.toggle('is-solid', y > 24);
      const dy = y - lastY;
      if (Math.abs(dy) > 8) {
        const hide = dy > 0 && y > vh * 0.5;
        if (hide !== hidden) {
          hidden = hide;
          header.classList.toggle('is-hidden', hide);
        }
        lastY = y;
      }
    }
    if (wordmark && footH) {
      const f = clamp(1 - (docH - (y + vh)) / footH);
      wordmark.style.translate = `0 ${((1 - f) * 38).toFixed(2)}%`;
    }
  };

  const measure = () => {
    vh = innerHeight;
    const y = scrollY;
    edges = chapters.map((ch) => {
      const r = ch.getBoundingClientRect();
      return [r.top + y, r.bottom + y];
    });
    for (const s of scenes) {
      const r = s.el.getBoundingClientRect();
      s.top = r.top + y;
      s.len = Math.max(1, r.height - vh);
      s.measure();
      s.p = -1;
    }
    docH = document.documentElement.scrollHeight;
    footH = footer?.offsetHeight ?? 0;
    tick();
  };

  addEventListener(
    'scroll',
    () => {
      if (!queued) {
        queued = true;
        requestAnimationFrame(tick);
      }
    },
    { passive: true },
  );
  let pending = 0;
  const remeasure = () => {
    cancelAnimationFrame(pending);
    pending = requestAnimationFrame(measure);
  };
  addEventListener('resize', remeasure);
  new ResizeObserver(remeasure).observe(document.body);
  measure();
}

/** The hero: the words step back, then the page zooms through the mark's body into Starlight. */
export function heroScene(el: HTMLElement, sky: Sky): Scene {
  const copy = el.querySelector<HTMLElement>('.hero__copy');
  const foot = el.querySelector<HTMLElement>('.hero__foot');
  return {
    ...base(el),
    measure() {},
    update(p) {
      sky.zoom(p);
      const k = clamp(p / 0.3);
      if (copy) {
        copy.style.translate = `0 ${(-k * 12).toFixed(2)}vh`;
        copy.style.opacity = (1 - k).toFixed(3);
        // Out of sight is out of the tab order too.
        copy.style.visibility = k >= 1 ? 'hidden' : '';
      }
      if (foot) foot.style.opacity = (1 - clamp(p / 0.1)).toFixed(3);
    },
    theme: () => (sky.covered() ? 'light' : undefined),
  };
}

/**
 * How we work: the belt is drawn through the three stars as the reader scrolls, each step arriving
 * as the line reaches its star; then Mintaka grows until the screen is its blue. On narrow screens
 * the steps stand in a column, the belt is simply drawn, and nothing is pinned.
 */
export function beltScene(el: HTMLElement): Scene {
  const pin = el.querySelector<HTMLElement>('.method__pin');
  const stars = [...el.querySelectorAll<HTMLElement>('.step__star')];
  const steps = [...el.querySelectorAll<HTMLElement>('.step')];
  const line = el.querySelector<SVGPathElement>('.belt__line');
  const ext = el.querySelector<SVGPathElement>('.belt__ext');
  const burst = el.querySelector<HTMLElement>('.burst--blue');
  const wide = matchMedia('(min-width: 60rem)');
  let pts: [number, number][] = [];
  let at: number[] = [];
  let total = 1;
  let cover = 0;
  let r0 = 14;
  const b0 = 0.62;
  const bLen = 0.3;
  return {
    ...base(el),
    measure() {
      if (!pin || !line || stars.length < 3) return;
      const pr = pin.getBoundingClientRect();
      pts = stars.map((s) => {
        const r = s.getBoundingClientRect();
        return [r.left + r.width / 2 - pr.left, r.top + r.height / 2 - pr.top];
      });
      const [p0, p1, p2] = pts as [[number, number], [number, number], [number, number]];
      const l1 = Math.hypot(p1[0] - p0[0], p1[1] - p0[1]);
      const l2 = Math.hypot(p2[0] - p1[0], p2[1] - p1[1]);
      total = Math.max(1, l1 + l2);
      at = [0, l1 / total, 1];
      line.setAttribute('d', `M${p0[0].toFixed(1)} ${p0[1].toFixed(1)}L${p1[0].toFixed(1)} ${p1[1].toFixed(1)}L${p2[0].toFixed(1)} ${p2[1].toFixed(1)}`);
      // The belt runs on past both ends, dashed: a construction line, as on the brand board.
      const far = Math.hypot(pr.width, pr.height);
      const out = (from: [number, number], to: [number, number]) => {
        const d = Math.hypot(to[0] - from[0], to[1] - from[1]) || 1;
        return [to[0] + ((to[0] - from[0]) / d) * far, to[1] + ((to[1] - from[1]) / d) * far];
      };
      const [e0x, e0y] = out(p1, p0);
      const [e2x, e2y] = out(p1, p2);
      ext?.setAttribute('d', `M${e0x!.toFixed(1)} ${e0y!.toFixed(1)}L${p0[0].toFixed(1)} ${p0[1].toFixed(1)}M${p2[0].toFixed(1)} ${p2[1].toFixed(1)}L${e2x!.toFixed(1)} ${e2y!.toFixed(1)}`);
      line.style.strokeDasharray = wide.matches ? total.toFixed(1) : '';
      if (!wide.matches) line.style.strokeDashoffset = '';
      r0 = (stars[2]?.offsetWidth ?? 28) / 2;
      cover = Math.hypot(Math.max(p2[0], pr.width - p2[0]), Math.max(p2[1], pr.height - p2[1]));
      if (burst && !wide.matches) burst.style.visibility = 'hidden';
    },
    update(p) {
      if (!wide.matches || !line || pts.length < 3) return;
      const draw = clamp(p / 0.56);
      line.style.strokeDashoffset = (total * (1 - draw)).toFixed(1);
      steps.forEach((s, i) => s.classList.toggle('on', draw >= (at[i] ?? 0) - 0.002));
      if (!burst) return;
      const b = clamp((p - b0) / bLen);
      if (b <= 0) {
        burst.style.visibility = 'hidden';
        return;
      }
      const [mx, my] = pts[2]!;
      burst.style.visibility = 'visible';
      burst.style.clipPath = `circle(${lerp(r0, cover, easeIn(b)).toFixed(1)}px at ${mx.toFixed(1)}px ${my.toFixed(1)}px)`;
    },
    theme: (p) => (wide.matches && p > b0 + bLen * 0.6 ? 'blue' : undefined),
  };
}
