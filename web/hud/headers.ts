// The proof console's "Security headers" row and its raw view. Imported the first time the panel
// opens, together with the HEAD request it needs, so the console's first load stays small.
// Imports only shared modules (never ./index.ts), so the console stays a single chunk.

import { h, render, s } from '../shared/dom.ts';
import { t } from '../shared/i18n.ts';

const T = (key: string): string => t('hud-' + key);
const HSTS = 'strict-transport-security';
const EXPECTED = [
  'content-security-policy',
  'cross-origin-opener-policy',
  'cross-origin-embedder-policy',
  'cross-origin-resource-policy',
  'permissions-policy',
  'referrer-policy',
  'x-content-type-options',
  HSTS,
];

/** Same-origin HEAD of this page: our security headers into `cell`, every header into `raw`. */
export async function scan(cell: HTMLElement, raw: HTMLElement): Promise<void> {
  const res = await fetch(location.href, { method: 'HEAD', cache: 'no-store' });
  const secure = location.protocol === 'https:';
  render(
    cell,
    h(
      'ul',
      { class: 'hud-list', role: 'list' },
      EXPECTED.map((name) => {
        const sent = res.headers.has(name);
        const note = !secure && name === HSTS ? 'https-only' : 'missing';
        return h(
          'li',
          { class: sent ? 'hud-sent' : 'hud-miss' },
          sent && s('svg', { class: 'hud-tick', viewBox: '0 0 12 12', 'aria-hidden': 'true' }, s('path', { d: 'M2.5 6.5 5 9l4.5-6' })),
          h('span', { class: 'mono' }, name),
          sent
            ? h('span', { class: 'visually-hidden' }, T('sent'))
            : h('span', { class: `badge badge--${note === 'missing' ? 'bad' : 'plain'}` }, T(note)),
        );
      }),
    ),
  );
  raw.textContent = [...res.headers].map(([k, v]) => `${k}: ${v}`).join('\n');
}
