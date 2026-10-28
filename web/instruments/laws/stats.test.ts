import { assert, near, test } from '../testkit.ts';
import { fit, fittsId, hickBits, mean, shuffle, throughput, type Pt } from './stats.ts';

// Every expected value below was worked out by hand (sums of squares written out in comments).

test('fit: an exact line', () => {
  const f = fit([[1, 2], [2, 4], [3, 6]]);
  assert.ok(f);
  near(f!.a, 0);
  near(f!.b, 2);
  near(f!.r2, 1);
  assert.equal(f!.n, 3);
});

test('fit: three scattered points', () => {
  // mean x 2, mean y 2; Sxx = 2, Sxy = 1, Syy = 2 → b = 0.5, a = 1, R² = 1² / (2·2) = 0.25
  const f = fit([[1, 1], [2, 3], [3, 2]])!;
  near(f.b, 0.5);
  near(f.a, 1);
  near(f.r2, 0.25);
});

test('fit: Fitts-like data in ms', () => {
  // mean x 2.5, mean y 475; Sxx = 5, Sxy = 550, Syy = 62500
  // b = 110 ms/bit, a = 475 − 110·2.5 = 200 ms, R² = 550² / (5·62500) = 0.968
  const f = fit([[1, 300], [2, 450], [3, 500], [4, 650]])!;
  near(f.b, 110);
  near(f.a, 200);
  near(f.r2, 0.968);
});

test('fit: R² equals 1 − SSres/SStot', () => {
  // mean x 3, mean y 600; Sxx = 10, Sxy = 1100, Syy = 140000 → b = 110, a = 270
  // predictions 380 490 710 820, residuals 20 10 −110 80, SSres = 19000 → R² = 1 − 19000/140000
  const f = fit([[1, 400], [2, 500], [4, 600], [5, 900]])!;
  near(f.b, 110);
  near(f.a, 270);
  near(f.r2, 1 - 19000 / 140000);
  near(f.r2, 0.8642857142857143);
});

test('fit: degenerate input', () => {
  assert.equal(fit([]), null);
  assert.equal(fit([[1, 1]]), null);
  assert.equal(fit([[2, 1], [2, 3]]), null, 'all x equal: no slope');
  const flat = fit([[0, 5], [10, 5]])!;
  near(flat.b, 0);
  near(flat.a, 5);
  near(flat.r2, 1);
});

test('fittsId: log2(D/W + 1)', () => {
  near(fittsId(3, 1), 2);
  near(fittsId(64, 64), 1);
  near(fittsId(7, 1), 3);
  near(fittsId(744, 24), 5);
  near(fittsId(0, 10), 0);
});

test('throughput: mean of ID/MT in bits per second', () => {
  near(throughput([[2, 500], [3, 750]]), 4); // 4 and 4 bits/s
  near(throughput([[1, 500], [3, 500]]), 4); // 2 and 6
  near(throughput([[1, 250], [2, 1000], [3, 600]]), 11 / 3); // 4, 2 and 5
});

test('hickBits: log2(n + 1)', () => {
  near(hickBits(1), 1);
  near(hickBits(3), 2);
  near(hickBits(7), 3);
  near(hickBits(2), 1.584962500721156);
  near(hickBits(8), Math.log2(9));
});

test('mean', () => {
  near(mean([1, 2, 3, 4]), 2.5);
});

test('shuffle: a permutation of the input', () => {
  let seed = 7;
  const random = () => (seed = (seed * 16807) % 2147483647) / 2147483647;
  const xs: Pt[] = [[1, 1], [2, 2], [3, 3], [4, 4], [5, 5]];
  const out = shuffle([...xs], random);
  assert.equal(out.length, xs.length);
  assert.deepEqual([...out].sort((p, q) => p[0] - q[0]), xs);
});
