// @ts-ignore -- Node built-in; this repo carries no @types/node.
import { test } from 'node:test';
// @ts-ignore -- Node built-in.
import assert from 'node:assert/strict';
import { lines } from './visitors.ts';
import type { RumWindow } from '../shared/rum.ts';

const window = (metrics: RumWindow['metrics']): RumWindow => ({ from: 0, views: 30, routes: {}, metrics, events: {} });

test('visitors: Core Web Vitals first, every metric listed', () => {
  assert.deepEqual(
    lines(window({}), 20).map((l) => l.metric),
    ['lcp', 'inp', 'cls', 'fcp', 'ttfb'],
  );
});

test('visitors: too few samples show no number, even if one is sent', () => {
  const got = lines(window({ lcp: { n: 25, p75: 812 }, inp: { n: 7, p75: 48 }, cls: { n: 25, p75: 0 } }), 20);
  assert.deepEqual(got[0], { metric: 'lcp', n: 25, p75: 812 });
  assert.deepEqual(got[1], { metric: 'inp', n: 7, p75: null });
  assert.deepEqual(got[2], { metric: 'cls', n: 25, p75: 0 }, 'a measured zero is a number');
  assert.deepEqual(got[3], { metric: 'fcp', n: 0, p75: null });
});
