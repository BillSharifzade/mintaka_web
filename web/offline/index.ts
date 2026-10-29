// Offline mode, the page's half (spec §7.9, docs/CONTRACTS.md §15). Loaded on idle by main.ts:
// registers /sw.js under Trusted Types and tells it what this page view used, so the shell opens
// with no connection next time. Also the state an instrument shows when its code couldn't load
// because the browser is offline.

import { $, h } from '../shared/dom.ts';
import { lastValue, on } from '../shared/bus.ts';
import { whenOnline } from '../shared/env.ts';
import { t } from '../shared/i18n.ts';
import { scriptURL } from '../shared/tt.ts';

let started = false;

/** The /assets/ files this page view has loaded so far (Resource Timing; nothing is sent anywhere else). */
function used(): string[] {
  const out: string[] = [];
  for (const e of performance.getEntriesByType('resource')) {
    const url = new URL(e.name);
    if (url.origin === location.origin && url.pathname.startsWith('/assets/')) out.push(url.pathname);
  }
  return out;
}

export function start(): void {
  const sw = navigator.serviceWorker as ServiceWorkerContainer | undefined;
  if (started || !sw || !isSecureContext) return;
  started = true;
  const warm = (to: ServiceWorker | null | undefined) =>
    to?.postMessage({ type: 'warm', page: location.pathname, lang: document.documentElement.lang, tier: lastValue('tier') ?? null, used: used() });
  sw.register(scriptURL('/sw.js') as string, { scope: '/', updateViaCache: 'none' }).catch((err: unknown) => {
    if (MINTAKA_DEV) console.info('[mintaka] offline mode unavailable:', err);
  });
  void sw.ready.then((reg) => {
    warm(reg.active);
    // The renderer is known once the hero settles; a new worker (a new build) starts empty.
    on('tier', () => warm(sw.controller ?? reg.active));
    sw.addEventListener('controllerchange', () => warm(sw.controller));
  });
}

/**
 * An island's code (or its WebAssembly: `what` = 'wasm') didn't load and the browser is offline:
 * say so in its frame instead of leaving the no-JavaScript fallback, and try again when the
 * connection is back.
 */
export function unavailable(root: HTMLElement, retry: () => void, what: 'island' | 'wasm' = 'island'): void {
  const status = $('.instrument__status', root);
  if (status) {
    status.textContent = t('offline-status');
    status.className = 'instrument__status badge badge--plain';
  }
  const body = $('.instrument__body', root);
  if (body && !$('.offline-note', body)) body.prepend(h('p', { class: 'instrument__fallback offline-note', role: 'status' }, t('offline-' + what)));
  whenOnline(() => {
    $('.offline-note', root)?.remove();
    retry();
  });
}
