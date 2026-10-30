// The service worker's decisions as pure functions (offline mode, spec §7.9, docs/CONTRACTS.md
// §15). sw.ts wires them to the worker's events; lib.test.ts tests them in Node.
import { at, within } from '../shared/base.ts';

/** What the generator writes into sw.js for this build (crates/site/src/sw.rs). */
export interface Shell {
  /** The build id: the cache is named after it. */
  build: string;
  /** Page path → its language and the hash of its bytes (the server's ETag, unquoted): one per A/B test variant, any of which the server may send. */
  pages: Record<string, { lang: string; etags: string[] }>;
  /** Language-independent shell: main.js, islands.css, the hero core, with their imports. */
  core: string[];
  /** Language → the fonts its pages preload. */
  fonts: Record<string, string[]>;
  /** Render tier → its renderer chunk with its imports (A WebGPU, B WebGL2, C Canvas 2D). */
  renderers: Record<string, string[]>;
  /** Every /assets/ file of this build: the only assets ever stored. */
  assets: string[];
}

/** `network`: not handled, the browser fetches as if there were no worker. */
export type Strategy = 'network' | 'asset' | 'page';

/** Never cached, never answered by the worker: live measurements, APIs, the worker itself. */
const LIVE = /^\/(_|api\/|sw\.js$)/;

/** A hashed `/assets/` file of this site (a URL path, with the site's prefix). */
export const isAsset = (path: string): boolean => path.startsWith(at('/assets/'));

/**
 * How the worker treats a request. Hashed `/assets/` files come from the cache once stored. A page
 * goes to the network untouched while the browser is online — answering it from the worker would
 * replace its Navigation Timing (DNS, TCP, TLS) with the worker's own, and this page measures
 * itself — and comes from the cache when the browser reports no connection.
 */
export function strategy(req: { method: string; mode: string; url: string }, origin: string, online: boolean): Strategy {
  if (req.method !== 'GET') return 'network';
  const url = new URL(req.url);
  // Paths outside the site's URL prefix belong to whatever else shares the origin.
  const own = within(url.pathname);
  if (url.origin !== origin || own === null || LIVE.test(own)) return 'network';
  if (own.startsWith('/assets/')) return 'asset';
  return req.mode === 'navigate' && !online ? 'page' : 'network';
}

const PREFIX = 'mintaka-';
export const cacheName = (build: string): string => PREFIX + build;

/** Our caches from other builds (deleted when this worker activates). */
export const staleCaches = (names: readonly string[], build: string): string[] =>
  names.filter((n) => n.startsWith(PREFIX) && n !== cacheName(build));

/** May this path be stored: a page of this build, `/`, or an asset of this build. */
export function storable(shell: Shell, path: string): boolean {
  return path === at('/') || path in shell.pages || shell.assets.includes(path);
}

/** Stored paths that don't belong to this build (pruned on activation). */
export const prune = (shell: Shell, stored: readonly string[]): string[] => stored.filter((p) => !storable(shell, p));

/** What a page tells the worker about its view (web/offline/index.ts). */
export interface Warm {
  type: 'warm';
  /** location.pathname */
  page: string;
  /** <html lang> */
  lang: string;
  /** The hero's render tier, once known. */
  tier: string | null;
  /** /assets/ paths this page view has loaded so far. */
  used: string[];
}

export function isWarm(data: unknown): data is Warm {
  const w = data as Partial<Warm> | null;
  return (
    typeof w === 'object' && w !== null && w.type === 'warm' && typeof w.page === 'string' && typeof w.lang === 'string' &&
    (w.tier === null || typeof w.tier === 'string') && Array.isArray(w.used) && w.used.every((u) => typeof u === 'string')
  );
}

/**
 * The shell of one page view: its page, the core, its language's fonts, its renderer, and the
 * assets it used — of this build only, each once.
 */
export function shellFor(shell: Shell, w: Pick<Warm, 'page' | 'lang' | 'tier' | 'used'>): string[] {
  const lang = shell.pages[w.page]?.lang ?? w.lang;
  const all = [w.page, ...shell.core, ...(shell.fonts[lang] ?? []), ...(w.tier ? (shell.renderers[w.tier] ?? []) : []), ...w.used];
  return [...new Set(all)].filter((p) => storable(shell, p));
}

/** The install-time precache for pages already open: core plus each page with its fonts. */
export function installList(shell: Shell, pages: readonly string[]): string[] {
  return [...new Set([...shell.core, ...pages.flatMap((p) => shellFor(shell, { page: p, lang: '', tier: null, used: [] }))])];
}

/** Is a stored page this build's (any of its variants)? Compares its ETag (`"hash"`, `"hash-br"`, `"hash-gz"`). */
export function current(shell: Shell, path: string, etag: string | null): boolean {
  const hash = etag?.replace(/^(W\/)?"|"$/g, '').replace(/-(br|gz)$/, '');
  if (!hash) return false;
  const page = shell.pages[path];
  return (page ? [page] : Object.values(shell.pages)).some((p) => p.etags.includes(hash));
}

/** Replaces the server's Server-Timing on a stored copy: what it measured was another request. */
export const OFFLINE_TIMING = 'sw;desc="offline copy"';

/**
 * Headers of a stored copy: everything the server sent — CSP, COOP, COEP, CORP and the rest, so
 * the page is exactly as isolated offline — except its timings, and the length and coding of a
 * body the cache keeps decoded.
 */
export function storedHeaders(headers: Iterable<[string, string]>): [string, string][] {
  const out: [string, string][] = [];
  for (const [name, value] of headers) {
    const n = name.toLowerCase();
    if (n !== 'server-timing' && n !== 'content-length' && n !== 'content-encoding') out.push([n, value]);
  }
  out.push(['server-timing', OFFLINE_TIMING]);
  return out;
}
