// The contact form's proof of work (docs/CONTRACTS.md §8.1): the smallest counter such that
// SHA-256(challenge[41] ‖ counter[8, little-endian]) starts with `difficulty` zero bits. Runs in
// pow.worker.ts; pure, and tested against node:crypto in pow.test.ts.
//
// The message is 49 bytes, so it is exactly one SHA-256 block once padded: a hand-written,
// single-block SHA-256 in Int32 arithmetic. Message words 0–9 hold only challenge bytes, so the
// first ten rounds are the same for every counter and run once ("midstate"); each try rebuilds
// words 10–12, the schedule and rounds 10–63.

const K = Int32Array.of(
  0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
  0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
  0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
  0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
  0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
  0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
  0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
  0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
);
const IV = Int32Array.of(0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19);

/** Message schedule: w[16…63] from the block in w[0…15]. */
function expand(w: Int32Array): void {
  for (let i = 16; i < 64; i++) {
    const x = w[i - 15]!;
    const y = w[i - 2]!;
    const s0 = ((x >>> 7) | (x << 25)) ^ ((x >>> 18) | (x << 14)) ^ (x >>> 3);
    const s1 = ((y >>> 17) | (y << 15)) ^ ((y >>> 19) | (y << 13)) ^ (y >>> 10);
    w[i] = (w[i - 16]! + s0 + w[i - 7]! + s1) | 0;
  }
}

/** Compression rounds `from`…63 on the working state `st` (a…h), in place. */
function rounds(st: Int32Array, w: Int32Array, from: number, to = 64): void {
  let a = st[0]!, b = st[1]!, c = st[2]!, d = st[3]!, e = st[4]!, f = st[5]!, g = st[6]!, h = st[7]!;
  for (let i = from; i < to; i++) {
    const t1 =
      (h +
        (((e >>> 6) | (e << 26)) ^ ((e >>> 11) | (e << 21)) ^ ((e >>> 25) | (e << 7))) +
        ((e & f) ^ (~e & g)) +
        K[i]! +
        w[i]!) |
      0;
    const t2 = ((((a >>> 2) | (a << 30)) ^ ((a >>> 13) | (a << 19)) ^ ((a >>> 22) | (a << 10))) + ((a & b) ^ (a & c) ^ (b & c))) | 0;
    h = g;
    g = f;
    f = e;
    e = (d + t1) | 0;
    d = c;
    c = b;
    b = a;
    a = (t1 + t2) | 0;
  }
  st[0] = a; st[1] = b; st[2] = c; st[3] = d; st[4] = e; st[5] = f; st[6] = g; st[7] = h;
}

/** The padded block of a message of at most 55 bytes, as 64 schedule words (first 16 filled). */
function block(msg: Uint8Array): Int32Array {
  if (msg.length > 55) throw new RangeError('one SHA-256 block holds at most 55 message bytes');
  const bytes = new Uint8Array(64);
  bytes.set(msg);
  bytes[msg.length] = 0x80;
  const view = new DataView(bytes.buffer);
  view.setUint32(60, msg.length * 8);
  const w = new Int32Array(64);
  for (let i = 0; i < 16; i++) w[i] = view.getInt32(i * 4);
  return w;
}

/** SHA-256 of a message that fits one block (≤ 55 bytes). */
export function sha256(msg: Uint8Array): Uint8Array {
  const w = block(msg);
  expand(w);
  const st = IV.slice();
  rounds(st, w, 0);
  const out = new DataView(new ArrayBuffer(32));
  for (let i = 0; i < 8; i++) out.setInt32(i * 4, (st[i]! + IV[i]!) | 0);
  return new Uint8Array(out.buffer);
}

/** Leading zero bits of a digest. */
export function zeroBits(digest: Uint8Array): number {
  let n = 0;
  for (const byte of digest) {
    if (byte) return n + Math.clz32(byte) - 24;
    n += 8;
  }
  return n;
}

/** `challenge ‖ counter` as the 49-byte message the server hashes. */
export function message(challenge: Uint8Array, counter: number): Uint8Array {
  const msg = new Uint8Array(49);
  msg.set(challenge);
  const view = new DataView(msg.buffer);
  view.setUint32(41, counter >>> 0, true);
  view.setUint32(45, Math.floor(counter / 2 ** 32), true);
  return msg;
}

/**
 * The smallest counter ≥ `start` whose hash has `difficulty` (0…64) leading zero bits. Counters
 * stay below 2^53, so they are exact JS numbers. `progress` hears the number of tries every 65,536.
 */
export function solve(challenge: Uint8Array, difficulty: number, progress?: (tried: number) => void, start = 0): number {
  if (challenge.length !== 41) throw new RangeError('the challenge is 41 bytes');
  const w = block(message(challenge, 0));
  const mid = IV.slice();
  rounds(mid, w, 0, 10); // words 0–9 never change
  const st = new Int32Array(8);
  const top = challenge[40]! << 24; // byte 40 opens word 10, followed by counter bytes 0–2
  const iv0 = IV[0]!, iv1 = IV[1]!;
  const d0 = Math.min(difficulty, 32), d1 = difficulty - 32;
  let lo = start % 2 ** 32;
  for (let hi = Math.floor(start / 2 ** 32); hi < 2 ** 21; hi++, lo = 0) {
    const w11 = ((hi & 0xff) << 16) | (((hi >>> 8) & 0xff) << 8) | ((hi >>> 16) & 0xff);
    w[12] = ((hi >>> 24) << 24) | 0x800000; // counter byte 7, then the 0x80 pad byte
    for (; lo < 2 ** 32; lo++) {
      w[10] = top | ((lo & 0xff) << 16) | (((lo >>> 8) & 0xff) << 8) | ((lo >>> 16) & 0xff);
      w[11] = ((lo >>> 24) << 24) | w11;
      expand(w);
      st.set(mid);
      rounds(st, w, 10);
      const h0 = (st[0]! + iv0) | 0;
      if (Math.clz32(h0) >= d0 && (d1 <= 0 || (h0 === 0 && Math.clz32((st[1]! + iv1) | 0) >= d1))) {
        return hi * 2 ** 32 + lo;
      }
      if ((lo & 0xffff) === 0xffff) progress?.(hi * 2 ** 32 + lo + 1 - start);
    }
  }
  throw new RangeError('no counter below 2^53');
}

/** base64url without padding → bytes. */
export function fromBase64url(text: string): Uint8Array {
  return Uint8Array.from(atob(text.replace(/-/g, '+').replace(/_/g, '/')), (c) => c.charCodeAt(0));
}
