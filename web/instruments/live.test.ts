import { assert, test } from './testkit.ts';
import { peak, perDot, Ring, sparkPath } from './live.ts';

test('Ring keeps the last cap values, oldest first', () => {
  const r = new Ring(3);
  assert.deepEqual(r.values(), []);
  r.push(1);
  r.push(2);
  assert.deepEqual(r.values(), [1, 2]);
  assert.equal(r.size, 2);
  r.push(3);
  r.push(4);
  r.push(null);
  assert.deepEqual(r.values(), [3, 4, null]);
  assert.equal(r.size, 3);
  for (let i = 0; i < 100; i++) r.push(i);
  assert.deepEqual(r.values(), [97, 98, 99]);
});

test('Ring of 60 seconds', () => {
  const r = new Ring(60);
  for (let i = 1; i <= 75; i++) r.push(i);
  const v = r.values();
  assert.equal(v.length, 60);
  assert.equal(v[0], 16);
  assert.equal(v[59], 75);
});

test('peak ignores gaps', () => {
  assert.equal(peak([]), 0);
  assert.equal(peak([null, 3, null, 7, 2]), 7);
  assert.equal(peak([null]), 0);
});

test('sparkPath: newest value at the right edge, scaled to the peak', () => {
  // cap 5 in a 40×10 box: 10 units per second; y runs from 9 (zero) up to 1 (peak).
  assert.equal(sparkPath([0, 5, 10], 5, 40, 10), 'M20 9L30 5L40 1');
  assert.equal(sparkPath([2, 2], 3, 20, 10, 4), 'M10 5L20 5', 'explicit top');
});

test('sparkPath: a null breaks the line', () => {
  assert.equal(sparkPath([1, null, 1, 1], 4, 30, 10), 'M0 1M20 1L30 1');
});

test('sparkPath: all zero stays on the baseline; empty is empty', () => {
  assert.equal(sparkPath([0, 0], 2, 10, 10), 'M0 9L10 9');
  assert.equal(sparkPath([], 60, 120, 24), '');
});

test('sparkPath: out-of-range values are clamped into the box', () => {
  assert.equal(sparkPath([-5, 20], 2, 10, 10, 10), 'M0 9L10 1');
});

test('perDot: one request per dot until the stream would crowd', () => {
  assert.equal(perDot(0), 1);
  assert.equal(perDot(12), 1);
  assert.equal(perDot(40), 1);
  assert.equal(perDot(41), 2);
  assert.equal(perDot(150), 5);
  assert.equal(perDot(400), 10);
  assert.equal(perDot(401), 20);
  assert.equal(perDot(12_000), 500);
  for (const rps of [41, 99, 777, 5000, 123456]) assert.ok(rps / perDot(rps) <= 40, `${rps} rps`);
});
