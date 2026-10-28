// @ts-ignore -- Node built-in; this repo carries no @types/node.
import { test } from 'node:test';
// @ts-ignore -- Node built-in.
import assert from 'node:assert/strict';
import { LOGO, MINTAKA_S, bottomEdgeY, progress, region, sdTop, sdVessel, weights } from './geometry.ts';

test('the stars sit where the construction board says, relative to the shape', () => {
  // Corner stars are on the quads' outlines, 9 units inside the grown shape.
  assert.ok(Math.abs(sdTop(LOGO['bellatrix']![0], LOGO['bellatrix']![1]) + 9) < 1e-6);
  // The middle of each bulb is inside; far away is outside.
  assert.equal(region(-2, -60), 'top');
  assert.equal(region(4, 60), 'bottom');
  assert.equal(region(0, 0), 'channel');
  assert.equal(region(-150, 0), 'outside');
  assert.equal(region(0, -140), 'outside');
});

test('the channel is 22 units wide', () => {
  // Along the belt normal from Alnilam: channel until 11, bottom bulb after.
  const n = [1 / Math.sqrt(5), 2 / Math.sqrt(5)];
  assert.equal(region(n[0]! * 10.9, n[1]! * 10.9), 'channel');
  assert.equal(region(n[0]! * 11.1, n[1]! * 11.1), 'bottom');
  assert.equal(region(-n[0]! * 11.1, -n[1]! * 11.1), 'top');
});

test('Mintaka is 29.07 units along the belt and the bottom edge is open below the bulb', () => {
  assert.ok(Math.abs(MINTAKA_S - 29.069) < 0.01);
  const y = bottomEdgeY(0);
  assert.ok(sdVessel(0, y - 1) < 0 && sdVessel(0, y + 1) > 0);
  assert.ok(sdVessel(0, y + 5, true) < 0, 'open bottom lets particles out');
});

test('scroll progress and scene weights', () => {
  const keys = [0, 800, 1600, 2400] as const;
  assert.equal(progress(-10, keys), 0);
  assert.equal(progress(400, keys), 0.5);
  assert.equal(progress(1600, keys), 2);
  assert.equal(progress(9999, keys), 3);
  const w0 = weights(0);
  assert.equal(w0.flow, 1);
  assert.equal(w0.ray, 0);
  const w1 = weights(1.1);
  assert.ok(w1.ray > 0.99 && w1.flow === 0);
  const w2 = weights(2.2);
  assert.ok(w2.orion > 0.99 && w2.ray === 0 && w2.alpha === 1);
  assert.equal(weights(3).alpha, 0);
});

test('fill targets land inside their bulb', async () => {
  const { fillTarget } = await import('./geometry.ts');
  for (let i = 0; i <= 10; i++) {
    for (let j = 0; j <= 10; j++) {
      const [tx, ty] = fillTarget('top', i / 10, j / 10);
      assert.equal(region(tx, ty), 'top', `top ${i},${j} → ${tx},${ty}`);
      const [bx, by] = fillTarget('bottom', i / 10, j / 10);
      assert.equal(region(bx, by), 'bottom', `bottom ${i},${j} → ${bx},${by}`);
    }
  }
});
