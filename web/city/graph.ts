// Dushanbe's streets for the lab's opening experiment: the compact `MKCT` file written at build
// time by crates/site/src/city.rs, decoded into typed arrays, and searched with Dijkstra and A*.
// Everything here is plain arithmetic on typed arrays: no allocation per search, so a search over
// all 8,393 intersections takes about a millisecond, and the timings shown are the real thing.

export interface City {
  /** Intersections. */
  n: number;
  x: Float32Array;
  y: Float32Array;
  /** Extent, metres (x east, y south, from the north-west corner). */
  w: number;
  h: number;
  /** Directed arcs, CSR: arcs of u are off[u]..off[u + 1]. */
  off: Uint32Array;
  from: Uint32Array;
  to: Uint32Array;
  /** Length, metres. */
  len: Float32Array;
  /** The street an arc follows: s along its drawn direction, ~s against it. */
  street: Int32Array;
  /** Street polylines, end nodes included: points pOff[s]..pOff[s + 1] of px/py. */
  pOff: Uint32Array;
  px: Float32Array;
  py: Float32Array;
}

export function decode(buf: ArrayBuffer | Uint8Array): City {
  const b = buf instanceof Uint8Array ? buf : new Uint8Array(buf);
  if (b[0] !== 0x4d || b[1] !== 0x4b || b[2] !== 0x43 || b[3] !== 0x54) throw new Error('not a city file');
  if (b[4] !== 2) throw new Error(`city file version ${b[4]}`);
  // Seven columns, each read by its own cursor (crates/site/src/city.rs has the format).
  const at = [5, 0, 0, 0, 0, 0, 0, 0];
  const leb = (c: number): number => {
    let v = 0;
    let scale = 1;
    let byte: number;
    do {
      byte = b[at[c]!++]!;
      v += (byte & 0x7f) * scale;
      scale *= 128;
    } while (byte & 0x80);
    return v;
  };
  const zig = (c: number): number => {
    const v = leb(c);
    return v % 2 ? -(v + 1) / 2 : v / 2;
  };
  const n = leb(0);
  const ns = leb(0);
  const wu = leb(0);
  const hu = leb(0);
  const unit = leb(0);
  const lens: number[] = [];
  for (let k = 0; k < 7; k++) lens.push(leb(0));
  let start = at[0]!;
  for (let k = 0; k < 7; k++) {
    at[k] = start;
    start += lens[k]!;
  }
  if (start !== b.length) throw new Error('corrupt city file');
  const gx = new Int32Array(n);
  const gy = new Int32Array(n);
  const known = new Uint8Array(n);
  const su = new Uint32Array(ns);
  const sv = new Uint32Array(ns);
  const sdir = new Uint8Array(ns);
  const slen = new Float32Array(ns);
  const pOff = new Uint32Array(ns + 1);
  const pxs: number[] = [];
  const pys: number[] = [];
  let next = 0;
  let rx = 0;
  let ry = 0;
  let s = 0;
  let arcs = 0;
  for (let u = 0; u < n; u++) {
    if (!known[u]) {
      // A root of the walk: no street led here.
      rx += zig(1);
      ry += zig(1);
      gx[u] = rx;
      gy[u] = ry;
      known[u] = 1;
      next = u + 1;
    }
    for (let c = leb(0); c > 0; c--, s++) {
      const flags = leb(2);
      const k = Math.floor(flags / 8);
      const dir = flags & 3;
      let v: number;
      if (flags & 4) {
        v = next++;
        gx[v] = gx[u]! + zig(3);
        gy[v] = gy[u]! + zig(3);
        known[v] = 1;
      } else {
        v = u + leb(4);
      }
      su[s] = u;
      sv[s] = v;
      sdir[s] = dir;
      pOff[s] = pxs.length;
      let lx = gx[u]!;
      let ly = gy[u]!;
      pxs.push(lx * unit);
      pys.push(ly * unit);
      let length = 0;
      for (let j = 0; j < k; j++) {
        const qx = lx + zig(5);
        const qy = ly + zig(6);
        length += Math.hypot(qx - lx, qy - ly);
        pxs.push(qx * unit);
        pys.push(qy * unit);
        lx = qx;
        ly = qy;
      }
      length += Math.hypot(gx[v]! - lx, gy[v]! - ly);
      pxs.push(gx[v]! * unit);
      pys.push(gy[v]! * unit);
      slen[s] = length * unit;
      arcs += dir === 0 ? 2 : 1;
    }
  }
  pOff[ns] = pxs.length;
  if (s !== ns || next !== n) throw new Error('corrupt city file');
  const x = new Float32Array(n);
  const y = new Float32Array(n);
  for (let k = 0; k < n; k++) {
    x[k] = gx[k]! * unit;
    y[k] = gy[k]! * unit;
  }

  const deg = new Uint32Array(n + 1);
  for (let s = 0; s < ns; s++) {
    if (sdir[s] !== 2) deg[su[s]!]!++;
    if (sdir[s] !== 1) deg[sv[s]!]!++;
  }
  const off = new Uint32Array(n + 1);
  for (let k = 0; k < n; k++) off[k + 1] = off[k]! + deg[k]!;
  const fill = off.slice(0, n);
  const from = new Uint32Array(arcs);
  const to = new Uint32Array(arcs);
  const len = new Float32Array(arcs);
  const street = new Int32Array(arcs);
  const add = (a: number, c: number, l: number, st: number) => {
    const at = fill[a]!++;
    from[at] = a;
    to[at] = c;
    len[at] = l;
    street[at] = st;
  };
  for (let s = 0; s < ns; s++) {
    if (sdir[s] !== 2) add(su[s]!, sv[s]!, slen[s]!, s);
    if (sdir[s] !== 1) add(sv[s]!, su[s]!, slen[s]!, ~s);
  }
  return { n, x, y, w: wu * unit, h: hu * unit, off, from, to, len, street, pOff, px: Float32Array.from(pxs), py: Float32Array.from(pys) };
}

export interface Found {
  /** Metres; Infinity when the target can't be reached. */
  dist: number;
  /** Intersections settled, in order: the search's own picture of its work. */
  order: Uint32Array;
  settled: number;
  /** For each settled intersection, the arc it was reached by (-1 for the source). */
  tree: Int32Array;
  /** Arcs of the path, source to target. */
  path: number[];
  ms: number;
}

/** Dijkstra (`astar` false) or A* with the straight-line distance, which never overestimates a
 *  street's length — so both return the same, shortest, distance. Reuses its buffers. */
export class Search {
  private dist: Float64Array;
  private via: Int32Array;
  private seen: Uint32Array;
  private done: Uint32Array;
  private epoch = 0;
  private hk: Float64Array;
  private hn: Uint32Array;
  private order: Uint32Array;
  private c: City;

  constructor(c: City) {
    this.c = c;
    this.dist = new Float64Array(c.n);
    this.via = new Int32Array(c.n);
    this.seen = new Uint32Array(c.n);
    this.done = new Uint32Array(c.n);
    this.hk = new Float64Array(c.to.length + 1);
    this.hn = new Uint32Array(c.to.length + 1);
    this.order = new Uint32Array(c.n);
  }

  run(s: number, t: number, astar: boolean): Found {
    const t0 = performance.now();
    const { c, dist, via, seen, done, hk, hn, order } = this;
    const e = ++this.epoch;
    const tx = c.x[t]!;
    const ty = c.y[t]!;
    const h = astar ? (v: number) => Math.hypot(c.x[v]! - tx, c.y[v]! - ty) : () => 0;
    let size = 0;
    const push = (k: number, v: number) => {
      let j = ++size;
      while (j > 1 && hk[j >> 1]! > k) {
        hk[j] = hk[j >> 1]!;
        hn[j] = hn[j >> 1]!;
        j >>= 1;
      }
      hk[j] = k;
      hn[j] = v;
    };
    const pop = (): number => {
      const top = hn[1]!;
      const k = hk[size]!;
      const v = hn[size--]!;
      let j = 1;
      for (;;) {
        let m = j << 1;
        if (m > size) break;
        if (m < size && hk[m + 1]! < hk[m]!) m++;
        if (hk[m]! >= k) break;
        hk[j] = hk[m]!;
        hn[j] = hn[m]!;
        j = m;
      }
      hk[j] = k;
      hn[j] = v;
      return top;
    };
    seen[s] = e;
    dist[s] = 0;
    via[s] = -1;
    push(h(s), s);
    let settled = 0;
    while (size > 0) {
      const u = pop();
      if (done[u] === e) continue;
      done[u] = e;
      order[settled++] = u;
      if (u === t) break;
      const du = dist[u]!;
      for (let a = c.off[u]!; a < c.off[u + 1]!; a++) {
        const v = c.to[a]!;
        if (done[v] === e) continue;
        const nd = du + c.len[a]!;
        if (seen[v] !== e || nd < dist[v]!) {
          seen[v] = e;
          dist[v] = nd;
          via[v] = a;
          push(nd + h(v), v);
        }
      }
    }
    const ms = performance.now() - t0;
    const tree = new Int32Array(settled);
    for (let k = 0; k < settled; k++) tree[k] = via[order[k]!]!;
    const path: number[] = [];
    const reached = done[t] === e;
    if (reached) for (let v = t; via[v]! >= 0; v = c.from[via[v]!]!) path.push(via[v]!);
    path.reverse();
    return { dist: reached ? dist[t]! : Infinity, order: order.slice(0, settled), settled, tree, path, ms };
  }
}

/** Intersections that can reach `hub` and be reached from it: routes between any two of them
 *  always exist (the city's one-way streets leave a few corners that can't). */
export function strongly(c: City, hub: number): Uint8Array {
  const reach = (forward: boolean): Uint8Array => {
    const mark = new Uint8Array(c.n);
    // Reverse arcs, by head, for the backward pass.
    let roff: Uint32Array | null = null;
    let rarc: Uint32Array | null = null;
    if (!forward) {
      roff = new Uint32Array(c.n + 1);
      for (let a = 0; a < c.to.length; a++) roff[c.to[a]! + 1]!++;
      for (let k = 0; k < c.n; k++) roff[k + 1]! += roff[k]!;
      const fill = roff.slice(0, c.n);
      rarc = new Uint32Array(c.to.length);
      for (let a = 0; a < c.to.length; a++) rarc[fill[c.to[a]!]!++] = a;
    }
    const stack = [hub];
    mark[hub] = 1;
    while (stack.length) {
      const u = stack.pop()!;
      if (forward) {
        for (let a = c.off[u]!; a < c.off[u + 1]!; a++) {
          const v = c.to[a]!;
          if (!mark[v]) (mark[v] = 1), stack.push(v);
        }
      } else {
        for (let j = roff![u]!; j < roff![u + 1]!; j++) {
          const v = c.from[rarc![j]!]!;
          if (!mark[v]) (mark[v] = 1), stack.push(v);
        }
      }
    }
    return mark;
  };
  const f = reach(true);
  const r = reach(false);
  for (let k = 0; k < c.n; k++) f[k] = f[k]! & r[k]!;
  return f;
}

/** Nearest intersection to a point, among `ok` ones: a bucket grid, searched ring by ring. */
export class Nearest {
  private cells: Int32Array;
  private next: Int32Array;
  private cols: number;
  private rows: number;
  private c: City;
  private size: number;
  constructor(c: City, ok: Uint8Array, size = 150) {
    this.c = c;
    this.size = size;
    this.cols = Math.ceil(c.w / size) + 1;
    this.rows = Math.ceil(c.h / size) + 1;
    this.cells = new Int32Array(this.cols * this.rows).fill(-1);
    this.next = new Int32Array(c.n).fill(-1);
    for (let k = 0; k < c.n; k++) {
      if (!ok[k]) continue;
      const cell = this.cell(c.x[k]!, c.y[k]!);
      this.next[k] = this.cells[cell]!;
      this.cells[cell] = k;
    }
  }
  private cell(x: number, y: number): number {
    const cx = Math.min(this.cols - 1, Math.max(0, Math.floor(x / this.size)));
    const cy = Math.min(this.rows - 1, Math.max(0, Math.floor(y / this.size)));
    return cy * this.cols + cx;
  }
  find(x: number, y: number): number {
    const cx = Math.min(this.cols - 1, Math.max(0, Math.floor(x / this.size)));
    const cy = Math.min(this.rows - 1, Math.max(0, Math.floor(y / this.size)));
    let best = -1;
    let bd = Infinity;
    for (let r = 0; r < Math.max(this.cols, this.rows); r++) {
      for (let j = cy - r; j <= cy + r; j++) {
        if (j < 0 || j >= this.rows) continue;
        for (let i = cx - r; i <= cx + r; i++) {
          if (i < 0 || i >= this.cols || (Math.abs(i - cx) !== r && Math.abs(j - cy) !== r)) continue;
          for (let k = this.cells[j * this.cols + i]!; k >= 0; k = this.next[k]!) {
            const d = Math.hypot(this.c.x[k]! - x, this.c.y[k]! - y);
            if (d < bd) (bd = d), (best = k);
          }
        }
      }
      // Anything in a farther ring is at least r cells away.
      if (best >= 0 && bd <= r * this.size) break;
    }
    return best;
  }
}
