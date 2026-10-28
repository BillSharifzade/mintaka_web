// What the visitor's device and settings ask of us.

export const reducedMotion = (): boolean => matchMedia('(prefers-reduced-motion: reduce)').matches;

/** Data Saver (Chrome/Android) or an explicit Save-Data preference. */
export function saveData(): boolean {
  const c = (navigator as Navigator & { connection?: { saveData?: boolean } }).connection;
  return c?.saveData === true;
}

/** Run when the browser is idle (or after `timeout` ms at the latest). */
export function onIdle(fn: () => void, timeout = 1500): void {
  if ('requestIdleCallback' in window) requestIdleCallback(() => fn(), { timeout });
  else setTimeout(fn, 1);
}

/** Call `fn` once, the first time `el` comes within `margin` of the viewport. */
export function whenNear(el: Element, fn: () => void, margin = '100% 0px'): void {
  if (!('IntersectionObserver' in window)) return fn();
  const io = new IntersectionObserver(
    (entries) => {
      if (entries.some((e) => e.isIntersecting)) {
        io.disconnect();
        fn();
      }
    },
    { rootMargin: margin },
  );
  io.observe(el);
}

// Single-key shortcuts (`~` terminal, `P` proof console) can be switched off — WCAG 2.2 SC 2.1.4.
// The preference is a per-viewer convenience, so localStorage is fine; it may be unavailable.
const SHORTCUTS_KEY = 'mintaka.shortcuts';

export function shortcutsEnabled(): boolean {
  try {
    return localStorage.getItem(SHORTCUTS_KEY) !== 'off';
  } catch {
    return true;
  }
}

export function setShortcutsEnabled(on: boolean): void {
  try {
    localStorage.setItem(SHORTCUTS_KEY, on ? 'on' : 'off');
  } catch {
    /* private mode: the toggle lasts for this page view only */
  }
  document.documentElement.toggleAttribute('data-shortcuts-off', !on);
}
