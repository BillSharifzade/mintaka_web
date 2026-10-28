// S5 · This page's armour. A live scan, in the visitor's browser, of the security headers this
// page was served with (same-origin HEAD of this URL), the TLS facts from /_proof, whether the
// page is cross-origin isolated, and whether Trusted Types are enforced. Grading is pure.

import { $, h, render } from '../shared/dom.ts';
import { lang, pluralKey, t, tn } from '../shared/i18n.ts';
import { getProof, type Proof } from '../shared/proof.ts';

export const HEADERS = [
  'content-security-policy',
  'cross-origin-opener-policy',
  'cross-origin-embedder-policy',
  'cross-origin-resource-policy',
  'permissions-policy',
  'referrer-policy',
  'x-content-type-options',
  'strict-transport-security',
] as const;

export type Header = (typeof HEADERS)[number];

/** ok: in place · weak: sent but permissive · missing · na: not applicable (HSTS on plain HTTP). */
export type Status = 'ok' | 'weak' | 'missing' | 'na';

export interface Check {
  name: Header;
  status: Status;
  value: string | null;
}

/** Does the policy stop injected scripts? Looks at script-src, else default-src (CSP Level 3). */
function strictScripts(csp: string): boolean {
  const directives = new Map<string, string[]>();
  for (const part of csp.split(';')) {
    const [name, ...sources] = part.trim().toLowerCase().split(/\s+/);
    if (name && !directives.has(name)) directives.set(name, sources); // the first one counts
  }
  const sources = directives.get('script-src') ?? directives.get('default-src');
  if (!sources) return false;
  const hashed = sources.some((x) => /^'(nonce|sha256|sha384|sha512)-/.test(x)); // makes 'unsafe-inline' inert
  return !sources.some(
    (x) => x === "'unsafe-eval'" || x === '*' || x === 'data:' || x === 'http:' || x === 'https:' || (x === "'unsafe-inline'" && !hashed),
  );
}

/** Grade one header's value. `secure`: the page arrived over HTTPS. */
export function check(name: Header, raw: string | null, secure: boolean): Status {
  if (name === 'strict-transport-security' && !secure) return 'na'; // browsers ignore it over HTTP
  if (raw === null || !raw.trim()) return 'missing';
  const value = raw.split(/[;,]/)[0]!.trim().toLowerCase();
  const ok = (good: boolean): Status => (good ? 'ok' : 'weak');
  switch (name) {
    case 'content-security-policy':
      return ok(strictScripts(raw));
    case 'cross-origin-opener-policy':
      return ok(value === 'same-origin' || value === 'noopener-allow-popups');
    case 'cross-origin-embedder-policy':
      return ok(value === 'require-corp' || value === 'credentialless');
    case 'cross-origin-resource-policy':
      return ok(value === 'same-origin' || value === 'same-site');
    case 'permissions-policy':
      return ok(/=\s*\(\s*\)/.test(raw)); // at least one feature switched off
    case 'referrer-policy': {
      const last = raw.split(',').map((x) => x.trim().toLowerCase()).filter(Boolean).pop() ?? ''; // browsers use the last one
      return ok(['no-referrer', 'same-origin', 'strict-origin', 'strict-origin-when-cross-origin'].includes(last));
    }
    case 'x-content-type-options':
      return ok(value === 'nosniff');
    case 'strict-transport-security':
      return ok(Number(/max-age\s*=\s*"?(\d+)/i.exec(raw)?.[1] ?? 0) >= 31_536_000); // a year, as preload requires
  }
}

/** Every header we send (docs/CONTRACTS.md §4.2), graded; `ok` of `of` applicable ones. */
export function grade(get: (name: string) => string | null, secure: boolean): { checks: Check[]; ok: number; of: number } {
  const checks = HEADERS.map((name): Check => {
    const value = get(name);
    return { name, value, status: check(name, value, secure) };
  });
  return {
    checks,
    ok: checks.filter((c) => c.status === 'ok').length,
    of: checks.filter((c) => c.status !== 'na').length,
  };
}

/** A long header value cut to about `max` characters, at a separator where one is near. */
export function shorten(value: string, max = 44): string {
  if (value.length <= max) return value;
  const cut = value.slice(0, max);
  const at = Math.max(cut.lastIndexOf(';'), cut.lastIndexOf(','), cut.lastIndexOf(' '));
  return (at > max / 2 ? cut.slice(0, at) : cut).replace(/[\s;,]+$/, '') + '…';
}

/** "content-security-policy" → "Content-Security-Policy". */
export const display = (name: string): string => name.replace(/(^|-)([a-z])/g, (_, dash: string, c: string) => dash + c.toUpperCase());

export type Exchange = 'hybrid' | 'pq' | 'classical' | 'plain';

/** Which post-quantum line applies to this connection. */
export function exchange(tls: Proof['tls']): Exchange {
  if (!tls) return 'plain';
  if (!tls.post_quantum) return 'classical';
  return /x25519|secp|p-?256|p-?384/i.test(tls.group) ? 'hybrid' : 'pq';
}

export type TrustedTypesState = 'enforced' | 'open' | 'unsupported';

/**
 * Which explanation fits the isolation results: the browser is only blamed when our headers were
 * sent. `get` reads this page's response headers (undefined when the HEAD request failed).
 * Returns message keys for cross-origin isolation and for Trusted Types.
 */
export function explain(isolated: boolean, tt: TrustedTypesState, get?: (name: string) => string | null): [string, string] {
  const sent = (name: string, test: RegExp) => test.test(get?.(name) ?? '');
  const policies = sent('cross-origin-opener-policy', /same-origin/i) && sent('cross-origin-embedder-policy', /require-corp|credentialless/i);
  const required = sent('content-security-policy', /require-trusted-types-for\s+'script'/i);
  return [
    isolated ? 'coi-yes' : get && !policies ? 'coi-no-headers' : 'coi-no',
    tt === 'open' && get && !required ? 'tt-open-policy' : 'tt-' + tt,
  ];
}

/**
 * Are Trusted Types enforced? Assign a plain string to innerHTML of a detached throwaway element:
 * under `require-trusted-types-for 'script'` the browser throws. The element is never inserted,
 * read or kept. This is the one deliberate HTML-sink write in the codebase, and it is the test.
 */
export function trustedTypes(doc: Pick<Document, 'createElement'> = document): TrustedTypesState {
  const probe = doc.createElement('div');
  try {
    probe.innerHTML = 'mintaka';
  } catch {
    return 'enforced';
  }
  return 'trustedTypes' in globalThis ? 'open' : 'unsupported';
}

// ——— The island ———

const T = (key: string, vars?: Record<string, string | number>): string => t('armour-' + key, vars);
const BADGE: Record<string, string> = { ok: 'ok', weak: 'warn', missing: 'bad', na: 'plain', yes: 'ok', no: 'warn', enforced: 'ok', open: 'bad', unsupported: 'plain' };
const badge = (state: string, label: string) => h('span', { class: `badge badge--${BADGE[state] ?? 'plain'}` }, label);
const mono = (text: string) => h('span', { class: 'mono' }, text);
const reason = (err: unknown) => (err instanceof Error ? err.message : String(err));

export function mount(root: HTMLElement): void {
  const body = $('.instrument__body', root) ?? root.appendChild(h('div', { class: 'instrument__body' }));
  const status = $('.instrument__status', root);
  const secure = location.protocol === 'https:';
  const summary = h('p', { class: 'arm-sum', 'aria-live': 'polite' }, T('scanning'));
  render(body, h('div', { class: 'arm' }, summary));
  if (status) status.textContent = T('status-scanning');

  const head = fetch(location.href, { method: 'HEAD', cache: 'no-store' });
  const proof = getProof();
  void Promise.allSettled([head, proof]).then(([res, p]) => {
    const parts: Node[] = [];
    let get: ((name: string) => string | null) | undefined;
    if (res.status === 'fulfilled' && res.value.ok) {
      get = (n) => res.value.headers.get(n);
      const { checks, ok, of } = grade(get, secure);
      const count = (n: number) => h('span', { class: 'num' }, new Intl.NumberFormat(lang()).format(n));
      render(summary, ...tn(pluralKey('armour-summary', of), { ok: count(ok), n: count(of) }));
      if (!secure) summary.append(' ', T('summary-http'));
      parts.push(table(checks), h('p', { class: 'source' }, T('head-src')));
    } else {
      render(summary, T('head-error', { reason: res.status === 'fulfilled' ? `HTTP ${res.value.status}` : reason(res.reason) }));
    }
    parts.push(h('div', { class: 'arm-cards' }, tlsCard(p), isolationCard(get)));
    body.firstElementChild!.append(...parts);
    if (status) {
      status.textContent = T('status-done');
      status.classList.add('badge--info');
    }
  });
}

function table(checks: Check[]): HTMLElement {
  const cell = (tag: 'th' | 'td', role: string, ...kids: (Node | string)[]) => h(tag, { role, ...(tag === 'th' ? { scope: role === 'columnheader' ? 'col' : 'row' } : {}) }, ...kids);
  const value = (v: string | null, st: Status): Node | string => {
    if (v === null) return T(st === 'na' ? 'https-only' : 'not-sent');
    if (v.length <= 44) return h('code', { class: 'arm-val' }, v);
    return h('details', { class: 'arm-more' }, h('summary', null, h('code', { class: 'arm-val' }, shorten(v))), h('code', { class: 'arm-val arm-full' }, v));
  };
  return h(
    'table',
    { class: 'arm-table', role: 'table' },
    h('caption', { class: 'visually-hidden' }, T('caption')),
    h('thead', { role: 'rowgroup' }, h('tr', { role: 'row' }, ...['header', 'status', 'value'].map((c) => cell('th', 'columnheader', T('col-' + c))))),
    h(
      'tbody',
      { role: 'rowgroup' },
      checks.map((c) =>
        h(
          'tr',
          { role: 'row' },
          cell('th', 'rowheader', h('code', { class: 'arm-name' }, display(c.name)), h('span', { class: 'arm-why' }, T('why-' + c.name))),
          cell('td', 'cell', badge(c.status, T('st-' + c.status))),
          cell('td', 'cell', value(c.value, c.status)),
        ),
      ),
    ),
  );
}

function card(title: string, ...kids: (Node | string | null)[]): HTMLElement {
  return h('div', { class: 'arm-card' }, h('h4', { class: 'arm-card__title' }, title), ...kids);
}

const row = (label: string, value: Node | string) => h('div', null, h('dt', null, label), h('dd', null, value));

function tlsCard(p: PromiseSettledResult<Proof>): HTMLElement {
  if (p.status === 'rejected') return card(T('tls-title'), h('p', null, T('tls-error', { reason: reason(p.reason) })));
  const tls = p.value.tls;
  const kind = exchange(tls);
  return card(
    T('tls-title'),
    tls &&
      h(
        'dl',
        { class: 'kv' },
        row(T('tls-version'), mono(tls.version.replace(/^TLSv?/i, 'TLS '))),
        row(T('tls-cipher'), mono(tls.cipher)),
        row(T('tls-group'), mono(tls.group)),
      ),
    h(
      'p',
      { class: 'arm-pq' },
      badge(kind === 'hybrid' || kind === 'pq' ? 'ok' : 'na', T('pq-badge-' + kind)),
      ' ',
      ...tn('armour-pq-' + kind, { group: tls ? mono(tls.group) : '' }),
    ),
    h('p', { class: 'source' }, T('tls-src')),
  );
}

/** `get`: this page's response headers, when the HEAD request worked. */
function isolationCard(get?: (name: string) => string | null): HTMLElement {
  const coi = globalThis.crossOriginIsolated === true ? 'yes' : 'no';
  const tt = trustedTypes();
  const line = (label: string, state: string, key: string, why: string) =>
    h('div', null, h('dt', null, label), h('dd', null, badge(state, T(key))), h('dd', { class: 'arm-line' }, T(why + '-line')));
  const [coiWhy, ttWhy] = explain(coi === 'yes', tt, get);
  return card(
    T('iso-title'),
    h('dl', { class: 'arm-iso' }, line(T('coi'), coi, 'coi-' + coi, coiWhy), line(T('tt'), tt, 'tt-' + tt, ttWhy)),
    h('p', { class: 'source' }, T('iso-src')),
  );
}
