import { assert, hex, nodeSha256, test } from './testkit.ts';
import { fromBase64url, message, sha256, solve, zeroBits } from './pow.ts';

let seed = 0x5eed;
const random = (n: number): Uint8Array =>
  Uint8Array.from({ length: n }, () => ((seed = (seed * 1103515245 + 12345) >>> 0) >>> 16) & 0xff);

test('sha256: FIPS 180-2 vectors', () => {
  const enc = new TextEncoder();
  assert.equal(hex(sha256(enc.encode('abc'))), 'ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad');
  assert.equal(hex(sha256(new Uint8Array(0))), 'e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855');
});

test('sha256: matches node:crypto for every one-block length (0…55 bytes)', () => {
  for (let len = 0; len <= 55; len++) {
    for (let k = 0; k < 8; k++) {
      const msg = random(len);
      assert.equal(hex(sha256(msg)), hex(nodeSha256(msg)), `length ${len}`);
    }
  }
  assert.throws(() => sha256(new Uint8Array(56)), /55/);
});

test('message: challenge then the counter as 8 little-endian bytes', () => {
  const ch = random(41);
  const msg = message(ch, 0x0102030405);
  assert.equal(msg.length, 49);
  assert.deepEqual(Array.from(msg.subarray(0, 41)), Array.from(ch));
  assert.deepEqual(Array.from(msg.subarray(41)), [0x05, 0x04, 0x03, 0x02, 0x01, 0, 0, 0]);
});

test('zeroBits counts leading zero bits', () => {
  assert.equal(zeroBits(Uint8Array.of(0x80, 0)), 0);
  assert.equal(zeroBits(Uint8Array.of(0x01, 0xff)), 7);
  assert.equal(zeroBits(Uint8Array.of(0, 0, 0x20)), 18);
  assert.equal(zeroBits(Uint8Array.of(0, 0)), 16);
});

test('solve: the smallest counter, verified with node:crypto', () => {
  for (const difficulty of [0, 1, 4, 8, 12]) {
    const ch = random(41);
    const counter = solve(ch, difficulty);
    assert.ok(zeroBits(nodeSha256(message(ch, counter))) >= difficulty, `difficulty ${difficulty}`);
    for (let i = 0; i < counter; i++) {
      assert.ok(zeroBits(nodeSha256(message(ch, i))) < difficulty, `counter ${i} also solves ${difficulty}`);
    }
  }
});

test('solve: the fast path stays exact when the counter crosses 2^32', () => {
  // Start just below 2^32 with a difficulty that needs ~1000 tries, so the search must carry into
  // the high word; every counter it skipped must really fail, and the one it found must pass.
  let crossed = 0;
  for (let k = 0; k < 4; k++) {
    const ch = random(41);
    const start = 2 ** 32 - 3;
    const counter = solve(ch, 10, undefined, start);
    if (counter >= 2 ** 32) crossed++;
    assert.ok(zeroBits(nodeSha256(message(ch, counter))) >= 10);
    for (let i = start; i < counter; i++) assert.ok(zeroBits(nodeSha256(message(ch, i))) < 10, `skipped ${i}`);
  }
  assert.ok(crossed > 0, 'at least one search carried into the high word');
  // A counter with high bytes in use hashes the same way in both implementations.
  const ch = random(41);
  const found = solve(ch, 3, undefined, 2 ** 52 + 2 ** 40 + 12345);
  assert.ok(zeroBits(nodeSha256(message(ch, found))) >= 3);
  assert.equal(hex(sha256(message(ch, found))), hex(nodeSha256(message(ch, found))));
});

test('solve: difficulty 18, as the server sends it', () => {
  const ch = random(41);
  let heard = 0;
  const t0 = performance.now();
  const counter = solve(ch, 18, (tried) => (heard = tried));
  const ms = performance.now() - t0;
  assert.ok(zeroBits(nodeSha256(message(ch, counter))) >= 18);
  if (counter >= 0xffff) assert.ok(heard > 0, 'progress was reported');
  console.log(`  difficulty 18: counter ${counter} in ${ms.toFixed(0)} ms (${((counter + 1) / ms / 1000).toFixed(2)} M hashes/s)`);
});

test('solve: rejects a challenge of the wrong size', () => {
  assert.throws(() => solve(new Uint8Array(40), 1), /41/);
});

test('fromBase64url: 41 challenge bytes without padding', () => {
  const ch = random(41);
  const b64url = btoa(String.fromCharCode(...ch)).replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/, '');
  assert.equal(b64url.length, 55);
  assert.deepEqual(Array.from(fromBase64url(b64url)), Array.from(ch));
});
