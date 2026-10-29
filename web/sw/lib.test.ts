// @ts-ignore -- Node built-in; this repo carries no @types/node.
import { test } from 'node:test';
// @ts-ignore -- Node built-in.
import assert from 'node:assert/strict';
import { OFFLINE_TIMING, cacheName, current, installList, isWarm, prune, shellFor, staleCaches, storable, storedHeaders, strategy, type Shell } from './lib.ts';

const O = 'https://mintaka.tj';
const SHELL: Shell = {
  build: '2026-10-29.42870d86',
  pages: {
    '/en/': { lang: 'en', etag: 'aa11' },
    '/en/privacy/': { lang: 'en', etag: 'bb22' },
    '/ru/': { lang: 'ru', etag: 'cc33' },
  },
  core: ['/assets/main.A.js', '/assets/chunk.D.js', '/assets/islands.C.css', '/assets/hero.H.js'],
  fonts: { en: ['/assets/fonts/space-grotesk-latin.F.woff2'], ru: ['/assets/fonts/onest-cyrillic.R.woff2'] },
  renderers: { A: ['/assets/webgpu.G.js'], B: ['/assets/webgl.W.js'], C: ['/assets/canvas.V.js'] },
  assets: [
    '/assets/main.A.js', '/assets/chunk.D.js', '/assets/islands.C.css', '/assets/hero.H.js', '/assets/webgpu.G.js', '/assets/webgl.W.js',
    '/assets/canvas.V.js', '/assets/fonts/space-grotesk-latin.F.woff2', '/assets/fonts/onest-cyrillic.R.woff2', '/assets/hud.U.js',
    '/assets/routing.wasm.X.wasm',
  ],
};

const req = (url: string, mode = 'cors', method = 'GET') => ({ method, mode, url: url.startsWith('http') ? url : O + url });

test('strategy: hashed assets come from the cache', () => {
  for (const url of ['/assets/main.A.js', '/assets/fonts/space-grotesk-latin.F.woff2', '/assets/routing.wasm.X.wasm', '/assets/anything-else.js']) {
    assert.equal(strategy(req(url), O, true), 'asset', url);
    assert.equal(strategy(req(url), O, false), 'asset', url);
  }
});

test('strategy: live endpoints and the worker itself are never handled', () => {
  for (const url of ['/_proof', '/_telemetry', '/_ping', '/_rum', '/api/pow', '/api/contact', '/api/ctf/stage', '/sw.js']) {
    for (const online of [true, false]) {
      assert.equal(strategy(req(url), O, online), 'network', url);
      assert.equal(strategy(req(url, 'navigate'), O, online), 'network', `${url} navigated`);
    }
  }
});

test('strategy: pages go to the network untouched online, to the cache offline', () => {
  for (const url of ['/', '/en/', '/en/privacy/', '/en/no-such-page', '/en/?utm=x']) {
    assert.equal(strategy(req(url, 'navigate'), O, true), 'network', url);
    assert.equal(strategy(req(url, 'navigate'), O, false), 'page', url);
  }
  // The proof console's HEAD of this page, and fetches that aren't navigations, stay on the network.
  assert.equal(strategy(req('/en/', 'cors', 'HEAD'), O, false), 'network');
  assert.equal(strategy(req('/en/', 'cors'), O, false), 'network');
});

test('strategy: other methods, origins and files are left alone', () => {
  assert.equal(strategy(req('/api/contact', 'cors', 'POST'), O, true), 'network');
  assert.equal(strategy(req('/_rum', 'no-cors', 'POST'), O, false), 'network');
  assert.equal(strategy(req('/assets/main.A.js', 'cors', 'POST'), O, true), 'network');
  assert.equal(strategy(req('https://example.com/assets/x.js'), O, true), 'network');
  for (const url of ['/favicon.ico', '/robots.txt', '/manifest.webmanifest', '/.well-known/security.txt']) {
    assert.equal(strategy(req(url), O, false), 'network', url);
  }
});

test('cache names: one per build; the others are stale', () => {
  assert.equal(cacheName(SHELL.build), 'mintaka-2026-10-29.42870d86');
  const names = ['mintaka-2026-10-28.deadbeef', 'mintaka-2026-10-29.42870d86', 'someone-elses-cache', 'mintaka-2026-10-27.00000000'];
  assert.deepEqual(staleCaches(names, SHELL.build), ['mintaka-2026-10-28.deadbeef', 'mintaka-2026-10-27.00000000']);
});

test('what is precached for a page view: page, core, its fonts, its renderer, its used assets', () => {
  const got = shellFor(SHELL, { page: '/en/', lang: 'en', tier: 'B', used: ['/assets/hud.U.js', '/assets/main.A.js', '/assets/old.Z.js'] });
  assert.deepEqual(got, [
    '/en/',
    '/assets/main.A.js',
    '/assets/chunk.D.js',
    '/assets/islands.C.css',
    '/assets/hero.H.js',
    '/assets/fonts/space-grotesk-latin.F.woff2',
    '/assets/webgl.W.js',
    '/assets/hud.U.js',
  ]);
  assert.ok(!got.includes('/assets/old.Z.js'), 'assets of another build are never stored');
});

test('what is precached: the language comes from the page, the tier may be unknown', () => {
  const ru = shellFor(SHELL, { page: '/ru/', lang: 'en', tier: null, used: [] });
  assert.ok(ru.includes('/assets/fonts/onest-cyrillic.R.woff2'));
  assert.ok(!ru.includes('/assets/fonts/space-grotesk-latin.F.woff2'));
  assert.ok(!ru.some((u) => /webg|canvas/.test(u)), 'no renderer before the hero settles');
  // Tier D draws the poster: no renderer chunk.
  assert.ok(!shellFor(SHELL, { page: '/ru/', lang: 'ru', tier: 'D', used: [] }).some((u) => /webg|canvas/.test(u)));
  // `/` is negotiated: kept, with the fonts of the language the page says it is in.
  const root = shellFor(SHELL, { page: '/', lang: 'ru', tier: 'A', used: [] });
  assert.equal(root[0], '/');
  assert.ok(root.includes('/assets/fonts/onest-cyrillic.R.woff2') && root.includes('/assets/webgpu.G.js'));
  // A 404 or any other path is not a page of this build: not stored.
  assert.ok(!shellFor(SHELL, { page: '/en/no-such-page', lang: 'en', tier: null, used: [] }).includes('/en/no-such-page'));
});

test('install list: the core, then each open page with its fonts', () => {
  assert.deepEqual(installList(SHELL, []), SHELL.core);
  const list = installList(SHELL, ['/en/privacy/', '/ru/']);
  assert.deepEqual(list.slice(0, SHELL.core.length), SHELL.core);
  for (const u of ['/en/privacy/', '/ru/', '/assets/fonts/space-grotesk-latin.F.woff2', '/assets/fonts/onest-cyrillic.R.woff2']) assert.ok(list.includes(u), u);
  assert.equal(new Set(list).size, list.length);
});

test('pruning keeps only this build', () => {
  assert.deepEqual(prune(SHELL, ['/en/', '/', '/assets/main.A.js', '/assets/main.OLD.js', '/en/gone/']), ['/assets/main.OLD.js', '/en/gone/']);
  assert.ok(storable(SHELL, '/') && storable(SHELL, '/en/privacy/') && !storable(SHELL, '/_proof'));
});

test('a stored page is current when its ETag is this build’s, whatever the coding', () => {
  assert.ok(current(SHELL, '/en/', '"aa11"'));
  assert.ok(current(SHELL, '/en/', '"aa11-br"'));
  assert.ok(current(SHELL, '/en/', '"aa11-gz"'));
  assert.ok(!current(SHELL, '/en/', '"bb22"'));
  assert.ok(!current(SHELL, '/en/', null));
  assert.ok(current(SHELL, '/', '"cc33-br"'), '`/` is some language’s home page');
  assert.ok(!current(SHELL, '/', '"ffff"'));
});

test('stored copies keep every security header and drop the server’s timings', () => {
  const sent: [string, string][] = [
    ['Content-Security-Policy', "default-src 'none'"],
    ['cross-origin-embedder-policy', 'require-corp'],
    ['cross-origin-opener-policy', 'same-origin'],
    ['cross-origin-resource-policy', 'same-origin'],
    ['content-type', 'text/html; charset=utf-8'],
    ['etag', '"aa11-br"'],
    ['content-encoding', 'br'],
    ['content-length', '9123'],
    ['server-timing', 'app;dur=0.021, krtt;dur=48.2'],
  ];
  const kept = new Map(storedHeaders(sent));
  for (const name of ['content-security-policy', 'cross-origin-embedder-policy', 'cross-origin-opener-policy', 'cross-origin-resource-policy', 'content-type', 'etag']) {
    assert.ok(kept.has(name), name);
  }
  assert.equal(kept.get('cross-origin-embedder-policy'), 'require-corp');
  assert.ok(!kept.has('content-encoding') && !kept.has('content-length'));
  assert.equal(kept.get('server-timing'), OFFLINE_TIMING, 'a stored copy never claims a server measurement');
});

test('warm messages are checked before use', () => {
  assert.ok(isWarm({ type: 'warm', page: '/en/', lang: 'en', tier: null, used: [] }));
  assert.ok(isWarm({ type: 'warm', page: '/en/', lang: 'en', tier: 'B', used: ['/assets/x.js'] }));
  for (const bad of [null, 'warm', { type: 'warm' }, { type: 'warm', page: '/', lang: 'en', tier: 3, used: [] }, { type: 'warm', page: '/', lang: 'en', tier: null, used: [1] }]) {
    assert.ok(!isWarm(bad), JSON.stringify(bad));
  }
});
