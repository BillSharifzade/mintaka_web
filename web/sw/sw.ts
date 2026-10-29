// The service worker (offline mode, spec §7.9, docs/CONTRACTS.md §15), served at /sw.js with
// `Cache-Control: no-cache`. Built on its own by xtask as a classic script; the generator replaces
// `self.__MINTAKA_SHELL__` with this build's file list (crates/site/src/sw.rs), so every build
// that changes a shipped file is a new worker.
//
// It keeps one cache per build: the shell of the pages a visitor opened (the HTML, main.js,
// islands.css, the preloaded fonts, the hero core and the renderer this device runs) and every
// hashed asset once used. Pages go to the network untouched while the browser is online; with no
// connection they come from the cache. /_proof, /_telemetry, /_ping, /_rum, /api/* and /sw.js are
// never stored or answered by the worker.

import { cacheName, current, installList, isWarm, prune, shellFor, staleCaches, storable, storedHeaders, strategy, type Shell } from './lib.ts';

// The parts of ServiceWorkerGlobalScope used here (the project compiles against the DOM library).
interface Extendable extends Event {
  waitUntil(p: Promise<unknown>): void;
}
interface FetchLike extends Extendable {
  request: Request;
  respondWith(r: Response | Promise<Response>): void;
}
interface MessageLike extends Extendable {
  data: unknown;
}
interface Scope {
  __MINTAKA_SHELL__: Shell;
  location: Location;
  navigator: Navigator;
  clients: { claim(): Promise<void>; matchAll(o: { type: 'window'; includeUncontrolled: boolean }): Promise<readonly { url: string }[]> };
  skipWaiting(): Promise<void>;
  addEventListener(type: 'install' | 'activate', fn: (e: Extendable) => void): void;
  addEventListener(type: 'fetch', fn: (e: FetchLike) => void): void;
  addEventListener(type: 'message', fn: (e: MessageLike) => void): void;
}
declare const self: Scope;

const SHELL: Shell = self.__MINTAKA_SHELL__;
const NAME = cacheName(SHELL.build);
const path = (url: string) => new URL(url).pathname;

/** The response as stored: same status and headers (see storedHeaders), decoded body. */
const stored = (res: Response) =>
  new Response(res.body, { status: res.status, statusText: res.statusText, headers: storedHeaders(res.headers) });

/** Fetch and store `paths` that aren't stored yet (pages: unless the stored copy is this build's). */
async function keep(paths: readonly string[]): Promise<void> {
  const cache = await caches.open(NAME);
  await Promise.all(
    paths.map(async (p) => {
      const page = !p.startsWith('/assets/');
      const have = await cache.match(p, { ignoreVary: true });
      if (have && (!page || current(SHELL, p, have.headers.get('etag')))) return;
      try {
        // Pages revalidate (a 304 costs a few hundred bytes); hashed assets may come from the HTTP cache.
        const res = await fetch(p, { cache: page ? 'no-cache' : 'default', credentials: 'omit' });
        if (!res.ok || res.type !== 'basic') return;
        // `/` is served in the visitor's language: keep that language's fonts with it.
        const lang = page && !(p in SHELL.pages) ? res.headers.get('content-language') : null;
        await cache.put(p, stored(res));
        if (lang) await keep(SHELL.fonts[lang] ?? []);
      } catch {
        /* offline or refused: try again on the next page view */
      }
    }),
  );
}

self.addEventListener('install', (e) => {
  e.waitUntil(
    (async () => {
      // The pages open right now (the one registering us included) and those earlier builds kept.
      const open = await self.clients.matchAll({ type: 'window', includeUncontrolled: true });
      const earlier: string[] = [];
      for (const name of staleCaches(await caches.keys(), SHELL.build)) {
        for (const req of await (await caches.open(name)).keys()) if (!path(req.url).startsWith('/assets/')) earlier.push(path(req.url));
      }
      const pages = [...open.map((c) => path(c.url)), ...earlier].filter((p) => storable(SHELL, p) && !p.startsWith('/assets/'));
      await keep(installList(SHELL, pages));
      await self.skipWaiting();
    })(),
  );
});

self.addEventListener('activate', (e) => {
  e.waitUntil(
    (async () => {
      await Promise.all(staleCaches(await caches.keys(), SHELL.build).map((n) => caches.delete(n)));
      const cache = await caches.open(NAME);
      const keys = await cache.keys();
      const gone = new Set(prune(SHELL, keys.map((k) => path(k.url))));
      await Promise.all(keys.filter((k) => gone.has(path(k.url))).map((k) => cache.delete(k)));
      await self.clients.claim();
    })(),
  );
});

self.addEventListener('message', (e) => {
  if (isWarm(e.data)) e.waitUntil(keep(shellFor(SHELL, e.data)));
});

self.addEventListener('fetch', (e) => {
  const how = strategy(e.request, self.location.origin, self.navigator.onLine);
  if (how === 'asset') e.respondWith(asset(e.request, (p) => e.waitUntil(p)));
  else if (how === 'page') e.respondWith(page(e.request));
});

/** Hashed and immutable: the stored copy if there is one, else the network (stored on the way). */
async function asset(req: Request, later: (p: Promise<unknown>) => void): Promise<Response> {
  const cache = await caches.open(NAME);
  const hit = await cache.match(req, { ignoreVary: true });
  if (hit) return hit;
  const res = await fetch(req);
  if (res.ok && res.type === 'basic' && storable(SHELL, path(req.url))) later(cache.put(req.url, stored(res.clone())).catch(() => {}));
  return res;
}

/** No connection: the stored page, else the network's own error (the browser's offline page). */
async function page(req: Request): Promise<Response> {
  const hit = await (await caches.open(NAME)).match(req, { ignoreSearch: true, ignoreVary: true });
  return hit ?? fetch(req);
}
