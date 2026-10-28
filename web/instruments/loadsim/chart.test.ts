import { assert, near, test } from '../testkit.ts';
import { area, HIST, line, linear, loadRate, log, nice, Series } from './chart.ts';

const series = (...tail: number[]) => {
  const s = new Series();
  for (const x of tail) s.push(x);
  return s;
};

test('Series keeps the last HIST values, oldest first', () => {
  const s = series(1, 2, 3);
  assert.equal(s.v.length, HIST);
  assert.deepEqual([...s.v.slice(-3)], [1, 2, 3]);
  assert.ok(Number.isNaN(s.v[0]!));
  assert.equal(s.max(), 3);
  for (let i = 0; i < HIST + 5; i++) s.push(i);
  assert.equal(s.v[0], 5);
  assert.equal(s.v[HIST - 1], HIST + 4);
  s.clear();
  assert.equal(s.max(), 0);
});

test('nice: 1, 2 or 5 × 10^k, with a floor', () => {
  assert.equal(nice(0), 1);
  assert.equal(nice(0, 20), 20);
  assert.equal(nice(380), 500);
  assert.equal(nice(400), 500);
  assert.equal(nice(500), 500);
  assert.equal(nice(1201), 2000);
  assert.equal(nice(6100), 10000);
});

test('scales map into 0…1 and clamp', () => {
  const lin = linear(400);
  near(lin(100), 0.25);
  assert.equal(lin(-5), 0);
  assert.equal(lin(900), 1);
  const lg = log(10, 10_000);
  near(lg(10), 0);
  near(lg(100), 1 / 3);
  near(lg(1000), 2 / 3);
  assert.equal(lg(1), 0);
  assert.equal(lg(1e9), 1);
});

test('line: newest at the right edge; NaN breaks it', () => {
  const s = series(0, 400, Number.NaN, 200);
  const d = line(s.v, 239, 10, linear(400));
  // x = i for a 239-wide box; y runs from 9 (zero) to 1 (top).
  assert.equal(d, 'M236 9L237 1M239 5');
});

test('area: closed down to the baseline, one shape per run', () => {
  const s = series(100, 100, Number.NaN, 100);
  const d = area(s.v, 239, 10, linear(100));
  // Values at x = 236, 237 (a run), a gap at 238, then 239 alone.
  assert.equal(d, 'M236 1L237 1L237 9L236 9ZM239 1L239 9L239 9Z');
  assert.equal(area(new Series().v, 239, 10, linear(1)), '');
});

test('loadRate: the multiplier times the base rate', () => {
  assert.equal(loadRate(20, 20), 400);
  assert.equal(loadRate(1, 20), 20);
  assert.equal(loadRate(100, 20), 2000);
});
