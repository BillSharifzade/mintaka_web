// @ts-ignore -- Node built-in; this repo carries no @types/node.
import { test } from 'node:test';
// @ts-ignore -- Node built-in.
import assert from 'node:assert/strict';
import { cls, clsText, cookieCount, cookies, httpVersion, slowest, tlsVersion, transport } from './metrics.ts';
import { __setStrings, pluralKey, tp } from '../shared/i18n.ts';
import type { Proof } from '../shared/proof.ts';

const shift = (startTime: number, value: number, hadRecentInput = false) => ({ startTime, value, hadRecentInput });

test('CLS: no shifts is a measured zero', () => {
  assert.equal(cls([]), 0);
});

test('CLS: shifts less than 1 s apart share a session window', () => {
  assert.equal(cls([shift(100, 0.1), shift(900, 0.05), shift(1800, 0.02)]), 0.1 + 0.05 + 0.02);
});

test('CLS: a gap of 1 s or more starts a new window; the largest window wins', () => {
  assert.equal(cls([shift(0, 0.1), shift(1000, 0.05)]), 0.1);
  assert.equal(cls([shift(0, 0.01), shift(3000, 0.2), shift(3500, 0.1)]), 0.2 + 0.1);
});

test('CLS: a window is at most 5 s long', () => {
  // Shifts every 900 ms: the window from 0 holds 0, 900, …, 4500; the one at 5400 starts over.
  const shifts = Array.from({ length: 8 }, (_, i) => shift(i * 900, 0.01));
  assert.ok(Math.abs(cls(shifts) - 0.06) < 1e-12, String(cls(shifts)));
});

test('CLS: shifts right after input do not count', () => {
  assert.equal(cls([shift(0, 0.3, true), shift(200, 0.01)]), 0.01);
});

test('slowest interaction: only entries that belong to an interaction', () => {
  assert.equal(slowest([]), 0);
  const entries = [
    { interactionId: 0, duration: 300 }, // a mousemove-like event, not an interaction
    { interactionId: 7, duration: 48 },
    { interactionId: 7, duration: 56 },
    { interactionId: 9, duration: 24 },
    { duration: 500 }, // no interactionId at all
  ];
  assert.equal(slowest(entries), 56);
  assert.equal(slowest([{ interactionId: 11, duration: 40 }], 56), 56, 'keeps the worst seen before');
  assert.equal(slowest([{ interactionId: 12, duration: 72 }], 56), 72);
});

test('CLS text', () => {
  assert.equal(clsText(0, 'en'), '0');
  assert.equal(clsText(0.004, 'en'), '0.004');
  assert.equal(clsText(0.1234, 'en'), '0.12');
  assert.equal(clsText(0.1234, 'ru'), '0,12');
});

test('cookies visible to scripts', () => {
  assert.equal(cookieCount(''), 0);
  assert.equal(cookieCount('  '), 0);
  assert.equal(cookieCount('a=1'), 1);
  assert.equal(cookieCount('a=1; b=2'), 2);
});

test('version labels', () => {
  assert.equal(tlsVersion('TLSv1.3'), 'TLS 1.3');
  assert.equal(tlsVersion('TLSv1.2'), 'TLS 1.2');
  assert.equal(httpVersion('HTTP/2.0'), 'HTTP/2');
  assert.equal(httpVersion('HTTP/3.0'), 'HTTP/3');
  assert.equal(httpVersion('HTTP/1.1'), 'HTTP/1.1');
});

const proof = (tls: Proof['tls']) => ({ tls }) as Proof;

test('pill transport: PQ only when the server says so', () => {
  const base = { version: 'TLSv1.3', cipher: 'TLS13_AES_128_GCM_SHA256', alpn: 'h2', handshake_us: 1830 };
  assert.equal(transport(proof({ ...base, group: 'X25519MLKEM768', post_quantum: true })), 'pq');
  assert.equal(transport(proof({ ...base, group: 'X25519', post_quantum: false })), 'TLS 1.3');
  assert.equal(transport(proof(null)), 'dev');
  assert.equal(transport(undefined), null, 'not known yet');
  assert.equal(transport(null), null, '/_proof failed');
});

test('cookies: only the server makes them ours', () => {
  assert.deepEqual(cookies('', 0), { count: 0, ours: false });
  assert.deepEqual(cookies('other_app=1', 0), { count: 1, ours: false }, 'localhost: another app on the same host');
  assert.deepEqual(cookies('', 2), { count: 2, ours: true }, 'HttpOnly cookies the page cannot see');
  assert.deepEqual(cookies('a=1; b=2; c=3', 1), { count: 3, ours: true });
  assert.deepEqual(cookies('', undefined), { count: 0, ours: false }, 'nothing visible, /_proof unknown');
  assert.deepEqual(cookies('x=1', undefined), { count: 1, ours: undefined }, "can't tell without /_proof");
});

test('plural keys: exact number, then the plural category, then the default', () => {
  __setStrings({
    'hud-requests': '{n} requests, {cached} from cache',
    'hud-requests[one]': '{n} request, {cached} from cache',
    'hud-requests[other]': '{n} requests, {cached} from cache',
    'hud-requests[0]': 'no requests yet',
  });
  assert.equal(pluralKey('hud-requests', 1), 'hud-requests[one]');
  assert.equal(pluralKey('hud-requests', 7), 'hud-requests[other]');
  assert.equal(pluralKey('hud-requests', 0), 'hud-requests[0]');
  assert.equal(pluralKey('hud-plain', 3), 'hud-plain', 'no variants: the message itself');
  assert.equal(tp('hud-requests', 1, { cached: 0 }), '1 request, 0 from cache');
  assert.equal(tp('hud-requests', 1200, { cached: 3 }), '1,200 requests, 3 from cache');
});
