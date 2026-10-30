// The site's URL prefix (docs/CONTRACTS.md §3.1): `/mintaka` when the site lives at
// https://host/mintaka/, '' at the root of its origin. esbuild inlines it (`--define:MINTAKA_BASE`
// in xtask); unit tests run unbundled, at the root.

/** `/mintaka`, or ''. */
export const BASE: string = typeof MINTAKA_BASE === 'string' ? MINTAKA_BASE : '';

/** A path of this site as a URL path: `/_proof` → `/mintaka/_proof`. */
export function at(path: string): string {
  return BASE + path;
}

/** A URL path without the site's prefix (`/mintaka/en/` → `/en/`); null outside the site. */
export function within(pathname: string): string | null {
  if (!BASE) return pathname;
  if (pathname === BASE) return '/';
  return pathname.startsWith(BASE + '/') ? pathname.slice(BASE.length) : null;
}
