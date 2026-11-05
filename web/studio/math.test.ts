// @ts-ignore -- Node built-in; this repo carries no @types/node.
import { test } from 'node:test';
// @ts-ignore -- Node built-in.
import assert from 'node:assert/strict';
import { easeInOutSine, glideMs } from './math.ts';

test('the glide eases evenly: it starts and ends at rest and passes halfway at half time', () => {
  assert.equal(easeInOutSine(0), 0);
  assert.ok(Math.abs(easeInOutSine(0.5) - 0.5) < 1e-12);
  assert.ok(Math.abs(easeInOutSine(1) - 1) < 1e-12);
  // Symmetric, and never faster than π/2 the average speed.
  for (let t = 0; t <= 1; t += 0.05) {
    assert.ok(Math.abs(easeInOutSine(t) + easeInOutSine(1 - t) - 1) < 1e-12);
    const v = (easeInOutSine(Math.min(1, t + 1e-6)) - easeInOutSine(t)) / 1e-6;
    assert.ok(v <= Math.PI / 2 + 1e-4, String(v));
  }
});

test('a longer glide takes longer, within 0.55–1.9 s; the hero to the brief takes ~1.7 s', () => {
  assert.equal(glideMs(0), 550);
  assert.ok(glideMs(1000) < glideMs(4000) && glideMs(4000) < glideMs(9000));
  assert.equal(glideMs(1e7), 1900);
  const hero = glideMs(9000);
  assert.ok(hero > 1600 && hero < 1800, String(hero));
});
