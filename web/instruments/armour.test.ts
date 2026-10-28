// @ts-ignore -- Node built-in; this repo carries no @types/node.
import { test } from 'node:test';
// @ts-ignore -- Node built-in.
import assert from 'node:assert/strict';
import { check, display, exchange, explain, grade, HEADERS, shorten, trustedTypes } from './armour.ts';
import type { Proof } from '../shared/proof.ts';

// Exactly what docs/CONTRACTS.md §4.2 says the server sends over TLS.
const CSP =
  "default-src 'none'; script-src 'self' 'wasm-unsafe-eval'; style-src 'self' 'sha256-abc='; img-src 'self'; font-src 'self'; connect-src 'self'; worker-src 'self'; manifest-src 'self'; form-action 'self'; base-uri 'none'; frame-ancestors 'none'; require-trusted-types-for 'script'; trusted-types mintaka; upgrade-insecure-requests";
const OURS: Record<string, string> = {
  'content-security-policy': CSP,
  'cross-origin-opener-policy': 'same-origin',
  'cross-origin-embedder-policy': 'require-corp',
  'cross-origin-resource-policy': 'same-origin',
  'permissions-policy': 'camera=(), microphone=(), geolocation=(), payment=(), usb=(), browsing-topics=()',
  'referrer-policy': 'no-referrer',
  'x-content-type-options': 'nosniff',
  'strict-transport-security': 'max-age=63072000; includeSubDomains; preload',
};
const from = (h: Record<string, string>) => (name: string) => (name in h ? h[name]! : null);

test('our own headers over TLS: 8 of 8 in place', () => {
  const g = grade(from(OURS), true);
  assert.equal(g.ok, 8);
  assert.equal(g.of, 8);
  assert.deepEqual(g.checks.map((c) => c.name), [...HEADERS]);
  assert.ok(g.checks.every((c) => c.status === 'ok' && c.value === OURS[c.name]));
});

test('plain HTTP: HSTS is not applicable and not counted', () => {
  const { 'strict-transport-security': _, ...dev } = OURS;
  const g = grade(from(dev), false);
  assert.equal(g.of, 7);
  assert.equal(g.ok, 7);
  assert.equal(g.checks.at(-1)?.status, 'na');
  assert.equal(check('strict-transport-security', OURS['strict-transport-security']!, false), 'na'); // ignored over HTTP
});

test('missing headers', () => {
  const g = grade(() => null, true);
  assert.equal(g.ok, 0);
  assert.equal(g.of, 8);
  assert.ok(g.checks.every((c) => c.status === 'missing' && c.value === null));
  assert.equal(check('x-content-type-options', '   ', true), 'missing');
});

test('CSP: what stops injected scripts and what does not', () => {
  assert.equal(check('content-security-policy', "script-src 'self'", true), 'ok');
  assert.equal(check('content-security-policy', "default-src 'self'", true), 'ok'); // falls back to default-src
  assert.equal(check('content-security-policy', "default-src 'self' 'unsafe-inline'", true), 'weak');
  assert.equal(check('content-security-policy', "script-src 'self' 'unsafe-eval'", true), 'weak');
  assert.equal(check('content-security-policy', "script-src 'self' 'wasm-unsafe-eval'", true), 'ok');
  assert.equal(check('content-security-policy', "script-src 'unsafe-inline' 'nonce-r4nd0m'", true), 'ok'); // the nonce disables unsafe-inline
  assert.equal(check('content-security-policy', "script-src https: 'self'", true), 'weak');
  assert.equal(check('content-security-policy', 'script-src *', true), 'weak');
  assert.equal(check('content-security-policy', "img-src 'self'", true), 'weak'); // no script policy at all
  assert.equal(check('content-security-policy', "script-src 'self'; script-src *", true), 'ok'); // the first directive counts
  assert.equal(check('content-security-policy', "SCRIPT-SRC 'SELF'", true), 'ok');
});

test('the other headers: strong and weak values', () => {
  assert.equal(check('cross-origin-opener-policy', 'same-origin; report-to="coop"', true), 'ok');
  assert.equal(check('cross-origin-opener-policy', 'same-origin-allow-popups', true), 'weak');
  assert.equal(check('cross-origin-opener-policy', 'unsafe-none', true), 'weak');
  assert.equal(check('cross-origin-embedder-policy', 'credentialless', true), 'ok');
  assert.equal(check('cross-origin-embedder-policy', 'unsafe-none', true), 'weak');
  assert.equal(check('cross-origin-resource-policy', 'same-site', true), 'ok');
  assert.equal(check('cross-origin-resource-policy', 'cross-origin', true), 'weak');
  assert.equal(check('permissions-policy', 'geolocation=(self)', true), 'weak');
  assert.equal(check('permissions-policy', 'camera=( )', true), 'ok');
  assert.equal(check('referrer-policy', 'unsafe-url', true), 'weak');
  assert.equal(check('referrer-policy', 'no-referrer, strict-origin-when-cross-origin', true), 'ok'); // the last one wins
  assert.equal(check('referrer-policy', 'no-referrer, unsafe-url', true), 'weak');
  assert.equal(check('x-content-type-options', 'NoSniff', true), 'ok');
  assert.equal(check('x-content-type-options', 'sniff', true), 'weak');
  assert.equal(check('strict-transport-security', 'max-age=31536000', true), 'ok');
  assert.equal(check('strict-transport-security', 'max-age=86400; includeSubDomains', true), 'weak');
  assert.equal(check('strict-transport-security', 'max-age=0', true), 'weak');
  assert.equal(check('strict-transport-security', 'includeSubDomains', true), 'weak');
});

test('shorten: long values cut at a separator; short ones untouched', () => {
  assert.equal(shorten('nosniff'), 'nosniff');
  const s = shorten(CSP);
  assert.ok(s.endsWith('…') && s.length <= 45, s);
  assert.ok(CSP.startsWith(s.slice(0, -1)));
  assert.equal(shorten("default-src 'none'; script-src 'self' 'wasm-unsafe-eval'"), "default-src 'none'; script-src 'self'…");
  assert.equal(shorten('x'.repeat(60), 10), 'xxxxxxxxxx…'); // no separator: a hard cut
});

test('display names', () => {
  assert.equal(display('content-security-policy'), 'Content-Security-Policy');
  assert.equal(display('x-content-type-options'), 'X-Content-Type-Options');
});

const tls = (group: string, post_quantum: boolean): Proof['tls'] => ({
  version: 'TLSv1.3',
  cipher: 'TLS13_AES_128_GCM_SHA256',
  group,
  post_quantum,
  alpn: 'h2',
  handshake_us: 1830,
});

test('post-quantum line', () => {
  assert.equal(exchange(tls('X25519MLKEM768', true)), 'hybrid');
  assert.equal(exchange(tls('SecP256r1MLKEM768', true)), 'hybrid');
  assert.equal(exchange(tls('MLKEM1024', true)), 'pq');
  assert.equal(exchange(tls('X25519', false)), 'classical');
  assert.equal(exchange(null), 'plain');
});

test('Trusted Types probe: blocked, allowed, or not supported', () => {
  const doc = (blocks: boolean) => ({
    createElement: () =>
      ({
        set innerHTML(_: string) {
          if (blocks) throw new TypeError("This document requires 'TrustedHTML' assignment.");
        },
      }) as unknown as HTMLElement,
  });
  assert.equal(trustedTypes(doc(true) as Pick<Document, 'createElement'>), 'enforced');
  const g = globalThis as { trustedTypes?: unknown };
  assert.equal(trustedTypes(doc(false) as Pick<Document, 'createElement'>), 'trustedTypes' in g ? 'open' : 'unsupported');
  g.trustedTypes = {};
  try {
    assert.equal(trustedTypes(doc(false) as Pick<Document, 'createElement'>), 'open');
  } finally {
    delete g.trustedTypes;
  }
});

test('isolation lines blame the browser only when our headers were sent', () => {
  const sent = from(OURS);
  assert.deepEqual(explain(true, 'enforced', sent), ['coi-yes', 'tt-enforced']);
  assert.deepEqual(explain(false, 'open', sent), ['coi-no', 'tt-open']); // headers sent, browser didn't follow
  assert.deepEqual(explain(false, 'open', () => null), ['coi-no-headers', 'tt-open-policy']); // a server that sends nothing
  const noCoep = from({ ...OURS, 'cross-origin-embedder-policy': 'unsafe-none' });
  assert.deepEqual(explain(false, 'enforced', noCoep), ['coi-no-headers', 'tt-enforced']);
  const noTT = from({ ...OURS, 'content-security-policy': "default-src 'self'" });
  assert.deepEqual(explain(true, 'open', noTT), ['coi-yes', 'tt-open-policy']);
  assert.deepEqual(explain(false, 'unsupported'), ['coi-no', 'tt-unsupported']); // HEAD failed: no claim about headers
});
