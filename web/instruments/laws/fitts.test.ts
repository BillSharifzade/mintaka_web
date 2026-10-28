import { assert, near, test } from '../testkit.ts';
import { DESIGN, place } from './fitts.ts';
import { fittsId } from './stats.ts';

test('the design: 15 targets, three widths, IDs from 1 to 5 bits', () => {
  assert.equal(DESIGN.length, 15);
  assert.deepEqual([...new Set(DESIGN.map(([w]) => w))], [64, 40, 24]);
  const ids = DESIGN.map(([, id]) => id);
  assert.equal(Math.min(...ids), 1);
  assert.equal(Math.max(...ids), 5);
  assert.ok(DESIGN.every(([w]) => w >= 24), 'no target below WCAG 2.2’s 24 px minimum');
  // The distance each design point asks for gives back its ID.
  for (const [w, id] of DESIGN) near(fittsId(w * (2 ** id - 1), w), id, 1e-12);
});

test('place: the requested distance when the arena allows it', () => {
  let seed = 3;
  const random = () => (seed = (seed * 48271) % 2147483647) / 2147483647;
  for (let i = 0; i < 200; i++) {
    const p = { x: 400, y: 200 };
    const q = place(p, 150, 20, 800, 400, random);
    near(Math.hypot(q.x - p.x, q.y - p.y), 150, 1e-9, 'distance');
  }
});

test('place: fully inside the arena and never farther than asked', () => {
  let seed = 11;
  const random = () => (seed = (seed * 48271) % 2147483647) / 2147483647;
  const w = 320;
  const h = 360;
  for (let i = 0; i < 500; i++) {
    const r = [32, 20, 12][i % 3]!;
    const p = { x: r + random() * (w - 2 * r), y: r + random() * (h - 2 * r) };
    const d = 744 * random();
    const q = place(p, d, r, w, h, random);
    assert.ok(q.x >= r - 1e-9 && q.x <= w - r + 1e-9 && q.y >= r - 1e-9 && q.y <= h - r + 1e-9, `inside: ${JSON.stringify(q)}`);
    assert.ok(Math.hypot(q.x - p.x, q.y - p.y) <= d + 1e-9, 'never farther than asked');
  }
  // From the centre of a small arena, a 744 px move goes to a corner instead.
  const q = place({ x: 160, y: 180 }, 744, 12, w, h, () => 0);
  assert.ok(Math.hypot(q.x - 160, q.y - 180) > 150);
});

test('place: a start too close to a wall for the new radius still ends inside', () => {
  const q = place({ x: 12, y: 12 }, 5, 32, 320, 360, () => 0);
  assert.ok(q.x >= 32 && q.y >= 32);
});
