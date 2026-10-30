// @ts-ignore -- Node built-in; this repo carries no @types/node.
import { test } from 'node:test';
// @ts-ignore -- Node built-in.
import assert from 'node:assert/strict';
import { rows } from './experiment.ts';
import type { RumExperiment } from '../shared/rum.ts';

const exp = (views: number[], conversions: number[], result: RumExperiment['result'] = null): RumExperiment => ({
  id: 'hero-headline',
  metric: 'cta_start_project',
  secondary: ['contact_submit'],
  plan: { baseline: 0.05, mde: 0.3, alpha: 0.05, power: 0.8, sample_size: 4578 },
  variants: ['hero-title', 'hero-title-hardware', 'hero-title-cycle'].map((id, i) => ({
    id,
    views: views[i]!,
    conversions: conversions[i]!,
    secondary: (i === 1 ? { contact_submit: 2 } : {}) as Record<string, number>,
  })),
  result,
});

const RESULT = [
  { variant: 'hero-title-hardware', z: 2.9, p: 0.0037, significant: true },
  { variant: 'hero-title-cycle', z: -0.8, p: 0.42, significant: false },
];

test('A/B test rows: progress, rates, the control first', () => {
  const r = rows(exp([1204, 1185, 0], [61, 70, 0]));
  assert.equal(r.done, false);
  assert.equal(r.alpha, 0.025, 'alpha shared by two comparisons');
  assert.deepEqual(r.rows.map((x) => [x.id, x.control, x.views, x.conversions]), [
    ['hero-title', true, 1204, 61],
    ['hero-title-hardware', false, 1185, 70],
    ['hero-title-cycle', false, 0, 0],
  ]);
  assert.ok(Math.abs(r.rows[0]!.rate - 61 / 1204) < 1e-12);
  assert.equal(r.rows[2]!.rate, 0, 'no page views, no rate');
  assert.deepEqual(r.rows.map((x) => x.secondary), [[0], [2], [0]]);
});

test('A/B test rows: no verdict before every variant has its page views, even if one is sent', () => {
  const early = rows(exp([4578, 4578, 4577], [229, 290, 210], RESULT));
  assert.equal(early.done, false);
  assert.ok(early.rows.every((x) => x.verdict === undefined));
});

test('A/B test rows: once complete, each variant against the control', () => {
  const r = rows(exp([4578, 4578, 4578], [229, 290, 210], RESULT));
  assert.equal(r.done, true);
  assert.equal(r.rows[0]!.verdict, undefined, 'the control is not compared with itself');
  assert.deepEqual(r.rows[1]!.verdict, { p: 0.0037, significant: true, better: true });
  assert.deepEqual(r.rows[2]!.verdict, { p: 0.42, significant: false, better: false });
});

test('A/B test rows: counts past the plan are never shown, rates never above 1', () => {
  const r = rows(exp([5000, 4578, 4578], [9000, 1, 1], RESULT));
  assert.equal(r.rows[0]!.views, 4578);
  assert.equal(r.rows[0]!.rate, 1);
});
