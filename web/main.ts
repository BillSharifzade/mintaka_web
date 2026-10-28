// Boot. Module scripts run after the document is parsed, so the page is already readable here.
// Keep this file tiny: every feature below is an island, loaded only when it is about to matter.

import { $, $$, h, isTyping } from './shared/dom.ts';
import { emit, on } from './shared/bus.ts';
import { onIdle, setShortcutsEnabled, shortcutsEnabled, whenNear } from './shared/env.ts';
import { assetUrl } from './shared/assets.ts';

interface Island {
  mount(root: HTMLElement): void;
}

// Island styles are one small stylesheet, fetched once, in parallel with the first island's code.
let styles: Promise<void> | null = null;
function islandStyles(): Promise<void> {
  styles ??= new Promise<void>((resolve) => {
    let href: string;
    try {
      href = assetUrl('islands.css');
    } catch {
      return resolve();
    }
    const link = h('link', { rel: 'stylesheet', href });
    link.addEventListener('load', () => resolve(), { once: true });
    link.addEventListener('error', () => resolve(), { once: true });
    document.head.append(link);
  });
  return styles;
}

/** Load an island module together with the island styles. */
function withStyles<T>(load: () => Promise<T>): Promise<T> {
  return Promise.all([load(), islandStyles()]).then(([m]) => m);
}

// `data-island="name"` containers, mounted when they come within one screen of the viewport.
const islands: Record<string, () => Promise<Island>> = {
  physics: () => import('./instruments/physics.ts'),
  armour: () => import('./instruments/armour.ts'),
  trace: () => import('./instruments/trace.ts'),
  live: () => import('./instruments/live.ts'),
  laws: () => import('./instruments/laws/index.ts'),
  contact: () => import('./instruments/contact.ts'),
  routing: () => import('./instruments/routing/index.ts'),
  shield: () => import('./instruments/shield/index.ts'),
  loadsim: () => import('./instruments/loadsim/index.ts'),
};

for (const root of $$('[data-island]')) {
  const name = root.dataset['island'] ?? '';
  const load = islands[name];
  if (!load) continue;
  whenNear(root, () => {
    withStyles(load)
      .then((m) => m.mount(root))
      .catch((err: unknown) => console.error(`[mintaka] island "${name}" failed`, err));
  });
}

const canvas = $<HTMLCanvasElement>('#hero-canvas');
if (canvas) onIdle(() => void import('./hero/index.ts').then((m) => m.start(canvas)), 300);

onIdle(() => {
  void import('./nav/index.ts').then((m) => m.start());
  void withStyles(() => import('./hud/index.ts')).then((m) => m.mount());
  void import('./console/index.ts').then((m) => m.greet());
});

on('proof:open', () => void withStyles(() => import('./hud/index.ts')).then((m) => m.open()));
on('terminal:open', () => void withStyles(() => import('./terminal/index.ts')).then((m) => m.open()));

// `~` opens the terminal, `P` the proof console. Physical keys, so they work on any layout;
// quiet while typing; can be switched off from the footer (WCAG 2.2 SC 2.1.4).
setShortcutsEnabled(shortcutsEnabled());
for (const b of $$('[data-action="shortcuts"]')) b.setAttribute('aria-pressed', String(shortcutsEnabled()));
document.addEventListener('keydown', (e) => {
  if (e.defaultPrevented || e.ctrlKey || e.metaKey || e.altKey || isTyping(e.target)) return;
  if (!shortcutsEnabled()) return;
  if (e.code === 'Backquote') {
    e.preventDefault();
    emit('terminal:open', undefined);
  } else if (e.code === 'KeyP' && !e.shiftKey) {
    e.preventDefault();
    emit('proof:open', undefined);
  }
});

// Declarative actions: `<button data-action="terminal">`. Links keep their default navigation.
document.addEventListener('click', (e) => {
  const el = (e.target as Element | null)?.closest<HTMLElement>('[data-action]');
  switch (el?.dataset['action']) {
    case 'proof':
      emit('proof:open', undefined);
      break;
    case 'terminal':
      emit('terminal:open', undefined);
      break;
    case 'shortcuts': {
      const next = !shortcutsEnabled();
      setShortcutsEnabled(next);
      for (const b of $$('[data-action="shortcuts"]')) b.setAttribute('aria-pressed', String(next));
      break;
    }
  }
});
