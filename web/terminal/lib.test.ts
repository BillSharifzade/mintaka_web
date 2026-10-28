// @ts-ignore -- Node built-in; this repo carries no @types/node.
import { test } from 'node:test';
// @ts-ignore -- Node built-in.
import assert from 'node:assert/strict';
import { browser, complete, flatten, hops, parse, platform, stats, table } from './lib.ts';

test('ping stats: 10 samples, p90 is the 9th smallest (nearest rank)', () => {
  const samples = [12, 3, 7, 5, 9, 1, 4, 8, 6, 10];
  assert.deepEqual(stats(samples), { min: 1, median: 6.5, p90: 10, max: 12 });
  assert.deepEqual(samples, [12, 3, 7, 5, 9, 1, 4, 8, 6, 10], 'input not reordered');
});

test('ping stats: odd counts, one sample, none', () => {
  assert.deepEqual(stats([5, 1, 3]), { min: 1, median: 3, p90: 5, max: 5 });
  assert.deepEqual(stats([2.5]), { min: 2.5, median: 2.5, p90: 2.5, max: 2.5 });
  assert.equal(stats([1, 2, 3, 4, 5, 6, 7, 8, 9]).p90, 9); // ceil(8.1) = 9th
  assert.throws(() => stats([]));
});

test('parse: command lower-cased, argument kept', () => {
  assert.deepEqual(parse('  LANG   tg '), ['lang', 'tg']);
  assert.deepEqual(parse('help'), ['help', '']);
  assert.deepEqual(parse('   '), ['', '']);
});

const NAMES = ['help', 'whoami', 'pillars', 'ping', 'trace', 'proof', 'lang', 'sky', 'ctf', 'contact', 'clear', 'exit'];

test('Tab completion of command names', () => {
  assert.deepEqual(complete('wh', NAMES), { line: 'whoami ', options: ['whoami'] });
  assert.deepEqual(complete('p', NAMES), { line: 'p', options: ['pillars', 'ping', 'proof'] });
  assert.deepEqual(complete('pi', NAMES), { line: 'pi', options: ['pillars', 'ping'] });
  assert.deepEqual(complete('c', NAMES), { line: 'c', options: ['ctf', 'contact', 'clear'] });
  assert.deepEqual(complete('co', NAMES), { line: 'contact ', options: ['contact'] });
  assert.deepEqual(complete('xyz', NAMES), { line: 'xyz', options: [] });
  assert.deepEqual(complete('PRO', NAMES), { line: 'proof ', options: ['proof'] });
});

test('Tab completion extends to the common prefix, and completes arguments', () => {
  assert.deepEqual(complete('a', ['alnitak', 'alnilam']), { line: 'alni', options: ['alnitak', 'alnilam'] });
  assert.deepEqual(complete('lang t', ['en', 'ru', 'tg']), { line: 'lang tg ', options: ['tg'] });
  assert.deepEqual(complete('lang ', ['en', 'ru', 'tg']), { line: 'lang ', options: ['en', 'ru', 'tg'] });
});

test('table: two aligned columns', () => {
  assert.equal(table([['a', '1'], ['long', '2']]), 'a     1\nlong  2');
  assert.equal(table([['k', 'v']], '  '), '  k  v');
  assert.equal(table([]), '');
});

test('flatten: dotted keys, arrays joined, null kept', () => {
  assert.deepEqual(flatten({ server: { region: 'dev', location: null }, headers: ['a', 'b'], tls: null, n: 3 }), [
    ['server.region', 'dev'],
    ['server.location', 'null'],
    ['headers', 'a, b'],
    ['tls', 'null'],
    ['n', '3'],
  ]);
});

const CHROME_LINUX = 'Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/141.0.0.0 Safari/537.36';
const EDGE = 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/141.0.0.0 Safari/537.36 Edg/141.0.0.0';
const FIREFOX = 'Mozilla/5.0 (X11; Linux x86_64; rv:145.0) Gecko/20100101 Firefox/145.0';
const SAFARI_IOS = 'Mozilla/5.0 (iPhone; CPU iPhone OS 26_0 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/26.0 Mobile/15E148 Safari/604.1';
const SAFARI_MAC = 'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/26.1 Safari/605.1.15';
const ANDROID_WEBVIEW = 'Mozilla/5.0 (Linux; Android 14; K; wv) AppleWebKit/537.36 (KHTML, like Gecko) Version/4.0 Chrome/141.0.0.0 Mobile Safari/537.36';
const SAMSUNG = 'Mozilla/5.0 (Linux; Android 14; SM-S918B) AppleWebKit/537.36 (KHTML, like Gecko) SamsungBrowser/28.0 Chrome/130.0.0.0 Mobile Safari/537.36';

test('browser: Client Hints first, skipping GREASE brands and plain Chromium', () => {
  const brands = [
    { brand: 'Not)A;Brand', version: '8' },
    { brand: 'Chromium', version: '141' },
    { brand: 'Google Chrome', version: '141' },
  ];
  assert.equal(browser(brands, CHROME_LINUX), 'Google Chrome 141');
  assert.equal(browser([{ brand: 'Chromium', version: '141' }, { brand: 'Not_A Brand', version: '24' }], ''), 'Chromium 141');
});

test('browser: a short UA parse when there are no Client Hints', () => {
  assert.equal(browser(undefined, CHROME_LINUX), 'Chrome 141');
  assert.equal(browser(undefined, EDGE), 'Microsoft Edge 141');
  assert.equal(browser(undefined, FIREFOX), 'Firefox 145');
  assert.equal(browser(undefined, SAFARI_IOS), 'Safari 26.0');
  assert.equal(browser(undefined, SAFARI_MAC), 'Safari 26.1');
  assert.equal(browser(undefined, ANDROID_WEBVIEW), 'Chrome 141');
  assert.equal(browser(undefined, SAMSUNG), 'Samsung Internet 28');
  assert.equal(browser([], 'curl/8.10'), '');
});

test('platform', () => {
  assert.equal(platform('Linux', ''), 'Linux');
  assert.equal(platform(undefined, ANDROID_WEBVIEW), 'Android');
  assert.equal(platform(undefined, SAFARI_IOS), 'iOS');
  assert.equal(platform(undefined, SAFARI_MAC), 'macOS');
  assert.equal(platform(undefined, EDGE), 'Windows');
  assert.equal(platform(undefined, FIREFOX), 'Linux');
  assert.equal(platform(undefined, 'something else'), '');
});

const nav = {
  startTime: 0,
  domainLookupStart: 2,
  domainLookupEnd: 14,
  connectStart: 14,
  connectEnd: 70,
  secureConnectionStart: 40,
  requestStart: 71,
  responseStart: 124,
  responseEnd: 126,
};

test('trace: a new TLS connection, hop by hop', () => {
  const timing = (name: string) => ({ app: 0.021, krtt: 48.2 })[name];
  assert.deepEqual(hops(nav, timing, true, false), [
    ['dns', 12, undefined],
    ['connect', 56, undefined],
    ['tls', 30],
    ['wait', 53],
    ['st-app', 0.021],
    ['st-krtt', 48.2],
    ['download', 2],
    ['total', 126],
  ]);
});

test('trace: a reused connection and plain HTTP say so', () => {
  const reused = { ...nav, domainLookupStart: 5, domainLookupEnd: 5, connectStart: 5, connectEnd: 5, secureConnectionStart: 0 };
  const none = () => undefined;
  const rows = hops(reused, none, true, true);
  assert.deepEqual(rows.slice(0, 3), [
    ['dns', 0, 'reused'],
    ['connect', 0, 'reused'],
    ['tls', 0, 'reused'],
  ]);
  assert.deepEqual(hops(nav, none, false, false)[2], ['tls', undefined, 'plain']);
  assert.deepEqual(hops({ ...nav, secureConnectionStart: 0 }, none, true, false)[2], ['tls', undefined], 'not reported');
});
