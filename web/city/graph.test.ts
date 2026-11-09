// @ts-ignore -- Node built-in; this repo carries no @types/node.
import { test } from 'node:test';
// @ts-ignore -- Node built-in.
import assert from 'node:assert/strict';
// @ts-ignore -- Node built-in.
import { readdirSync, readFileSync } from 'node:fs';
import { decode, Nearest, Search, strongly } from './graph.ts';

// The MKCT v2 writer, in miniature (crates/site/src/city.rs is the real one): the same walk.
function encode(nodes: [number, number][], streets: [number, number, number, [number, number][]][], unit = 2): Uint8Array {
  const cols: number[][] = [[], [], [], [], [], [], []];
  const leb = (c: number, v: number) => {
    do {
      let byte = v % 128;
      v = Math.floor(v / 128);
      if (v) byte |= 0x80;
      cols[c]!.push(byte);
    } while (v);
  };
  const zig = (c: number, v: number) => leb(c, v < 0 ? -2 * v - 1 : 2 * v);
  const n = nodes.length;
  const touching: number[][] = nodes.map(() => []);
  streets.forEach(([a, b], s) => {
    touching[a]!.push(s);
    if (b !== a) touching[b]!.push(s);
  });
  const id = new Array<number>(n).fill(-1);
  const byId: number[] = [];
  const owned: number[][] = [];
  const emitted = new Array<boolean>(streets.length).fill(false);
  const seeds = [...nodes.keys()].sort((p, q) => nodes[p]![1] - nodes[q]![1] || nodes[p]![0] - nodes[q]![0]);
  while (byId.length < n) {
    const root = seeds.find((k) => id[k] === -1)!;
    id[root] = byId.length;
    byId.push(root);
    const queue = [root];
    while (queue.length) {
      const u = queue.shift()!;
      const mine: number[] = [];
      for (const s of touching[u]!) {
        if (emitted[s]) continue;
        emitted[s] = true;
        const [a, b] = streets[s]!;
        const v = a === u ? b : a;
        if (id[v] === -1) {
          id[v] = byId.length;
          byId.push(v);
          queue.push(v);
        }
        mine.push(s);
      }
      owned.push(mine);
    }
  }
  const known = new Array<boolean>(n).fill(false);
  let root = [0, 0];
  byId.forEach((u, k) => {
    if (!known[u]) {
      zig(1, nodes[u]![0] - root[0]!);
      zig(1, nodes[u]![1] - root[1]!);
      root = nodes[u]!;
      known[u] = true;
    }
    leb(0, owned[k]!.length);
    for (const s of owned[k]!) {
      const [a, b, d, pts] = streets[s]!;
      const forward = a === u;
      const v = forward ? b : a;
      const dir = d === 0 ? 0 : forward ? d : 3 - d;
      const fresh = !known[v];
      leb(2, pts.length * 8 + (fresh ? 4 : 0) + dir);
      if (fresh) {
        zig(3, nodes[v]![0] - nodes[u]![0]);
        zig(3, nodes[v]![1] - nodes[u]![1]);
        known[v] = true;
      } else {
        leb(4, id[v]! - k);
      }
      let last = nodes[u]!;
      for (const q of forward ? pts : [...pts].reverse()) {
        zig(5, q[0] - last[0]);
        zig(6, q[1] - last[1]);
        last = q;
      }
    }
  });
  // The walk renumbers intersections; this test's assertions use the new ids through `id`.
  renumber = id;
  const head: number[] = [];
  const hleb = (v: number) => {
    do {
      let byte = v % 128;
      v = Math.floor(v / 128);
      if (v) byte |= 0x80;
      head.push(byte);
    } while (v);
  };
  [n, streets.length, 100, 100, unit, ...cols.map((c) => c.length)].forEach(hleb);
  return Uint8Array.from([0x4d, 0x4b, 0x43, 0x54, 2, ...head, ...cols.flat()]);
}
let renumber: number[] = [];

// A square 0-1-2-3 (units of 2 m), a one-way diagonal 0 → 2, and a bent street 1-2.
const square = encode(
  [[0, 0], [10, 0], [10, 10], [0, 10]],
  [
    [0, 1, 0, []],
    [0, 2, 1, []],
    [0, 3, 0, []],
    [1, 2, 0, [[15, 5]]],
    [2, 3, 0, []],
  ],
);

test('decodes positions in metres, street shapes, and arcs both ways unless one-way', () => {
  const c = decode(square);
  const r = renumber;
  assert.equal(c.n, 4);
  const pos = (k: number) => [c.x[r[k]!], c.y[r[k]!]];
  assert.deepEqual([pos(0), pos(1), pos(2), pos(3)], [[0, 0], [20, 0], [20, 20], [0, 20]]);
  assert.equal(c.w, 200);
  // 4 two-way streets and one one-way: 9 arcs.
  assert.equal(c.to.length, 9);
  // The bent street runs 1 → (30, 10) → 2: two legs of √(10² + 10²).
  const bent = [...c.from].findIndex((u, a) => u === r[1] && c.to[a] === r[2]);
  assert.ok(Math.abs(c.len[bent]! - 2 * Math.hypot(10, 10)) < 1e-4);
  // Nothing leaves 2 for 0: the diagonal is one-way.
  for (let a = c.off[r[2]!]!; a < c.off[r[2]! + 1]!; a++) assert.notEqual(c.to[a], r[0]);
});

test('Dijkstra and A* agree, and follow one-way streets', () => {
  const c = decode(square);
  const r = renumber;
  const s = new Search(c);
  const d = s.run(r[0]!, r[2]!, false);
  const a = s.run(r[0]!, r[2]!, true);
  assert.ok(Math.abs(d.dist - Math.hypot(20, 20)) < 1e-4, String(d.dist));
  assert.equal(a.dist, d.dist);
  assert.deepEqual(a.path, d.path);
  // Back from 2 to 0 there is no diagonal: 2 → 3 → 0, 40 m.
  const back = s.run(r[2]!, r[0]!, true);
  assert.equal(back.dist, 40);
  assert.equal(back.path.length, 2);
  assert.equal(c.from[back.path[0]!], r[2]);
  assert.equal(c.to[back.path[1]!], r[0]);
});

test('nearest intersection and the strongly connected part', () => {
  const c = decode(square);
  const r = renumber;
  const ok = strongly(c, r[0]!);
  assert.deepEqual([...ok], [1, 1, 1, 1]);
  const near = new Nearest(c, ok, 15);
  assert.equal(near.find(19, 3), r[1]);
  assert.equal(near.find(2, 18), r[3]);
});

test('on the real city, A* settles fewer intersections than Dijkstra for the same route', () => {
  let file: string | undefined;
  try {
    file = readdirSync('dist/assets/data').find((f: string) => /^city\.[A-Z0-9]+\.bin$/.test(f));
  } catch {
    return;
  }
  if (!file) return;
  const c = decode(readFileSync(`dist/assets/data/${file}`));
  assert.ok(c.n > 8000);
  const ok = strongly(c, Math.floor(c.n / 2));
  const near = new Nearest(c, ok);
  const s = new Search(c);
  const from = near.find(c.w * 0.3, c.h * 0.4);
  const to = near.find(c.w * 0.7, c.h * 0.6);
  const d = s.run(from, to, false);
  const a = s.run(from, to, true);
  assert.ok(Number.isFinite(d.dist));
  assert.ok(Math.abs(a.dist - d.dist) < 1e-6 * d.dist, `${a.dist} vs ${d.dist}`);
  assert.ok(a.settled < d.settled, `${a.settled} vs ${d.settled}`);
});
