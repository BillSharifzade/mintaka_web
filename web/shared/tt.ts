// The page's only Trusted Types policy (CSP: `trusted-types mintaka`). It exists for one sink:
// script URLs for Workers, and it only ever approves our own hashed files under /assets/ and the
// service worker at /sw.js (a fixed URL by necessity: its scope is the whole site).

interface TrustedTypePolicyLike {
  createScriptURL(input: string): unknown;
}
interface TrustedTypesLike {
  createPolicy(name: string, rules: { createScriptURL(input: string): string }): TrustedTypePolicyLike;
}

let policy: TrustedTypePolicyLike | null = null;

function check(input: string): string {
  const url = new URL(input, location.href);
  if (url.origin !== location.origin || !(url.pathname.startsWith('/assets/') || url.pathname === '/sw.js')) {
    throw new TypeError(`refusing script URL ${input}`);
  }
  return url.href;
}

/** A script URL the Worker constructor and serviceWorker.register() accept under Trusted Types. */
export function scriptURL(input: string): string | URL {
  const tt = (globalThis as { trustedTypes?: TrustedTypesLike }).trustedTypes;
  if (!tt) return check(input);
  policy ??= tt.createPolicy('mintaka', { createScriptURL: check });
  return policy.createScriptURL(input) as string;
}

/** Start a module worker from a build asset name (see assets.ts). */
export function worker(url: string, name?: string): Worker {
  return new Worker(scriptURL(url), { type: 'module', ...(name ? { name } : {}) });
}
