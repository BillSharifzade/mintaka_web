// The fuzzer island's logic, and its host against the real module that ships: `cargo xtask build`
// writes it to web/wasm/fuzzer/ (generated, gitignored), so build once before running these.

// @ts-ignore -- Node built-in; this repo carries no @types/node.
import { test } from 'node:test';
// @ts-ignore -- Node built-in.
import assert from 'node:assert/strict';
// @ts-ignore -- Node built-in.
import { existsSync, readFileSync } from 'node:fs';
import { ascii, count, escapeInput, hex, parseInput, rate, seconds, splitPanic } from './format.ts';
import { FuzzRun, PARSE_CODES, Probe, maskToPoints, minimize } from './host.ts';
import { POINTS, SKY, STARS, layout } from './sky.ts';

const wasmPath = new URL('../../wasm/fuzzer/fuzzer_bg.wasm', import.meta.url);
if (!existsSync(wasmPath)) throw new Error('web/wasm/fuzzer/fuzzer_bg.wasm is missing: run `cargo xtask build` first');
const module = new WebAssembly.Module(readFileSync(wasmPath));
const read = (path: string): string => readFileSync(new URL(path, import.meta.url), 'utf8');
const MINIMAL_CRASH = Uint8Array.from([0x4f, 0x52, 0x49, 0x02, 0x01, 0, 0, 0, 0, 0x4e, 0x01]);

test('hex and ascii views', () => {
  const b = Uint8Array.from([0x4f, 0x52, 0x49, 0x02, 0xff]);
  assert.equal(hex(b), '4F 52 49 02 FF');
  assert.equal(hex(b, 2), '4F 52 …');
  assert.equal(ascii(b), 'ORI··');
});

test('visitor input: escapes, hex, non-ASCII, round trip', () => {
  assert.deepEqual(parseInput('ORI\\x02\\x01'), Uint8Array.from([0x4f, 0x52, 0x49, 2, 1]));
  assert.deepEqual(parseInput('4F 52 49 02'), Uint8Array.from([0x4f, 0x52, 0x49, 2]));
  assert.equal(parseInput('bad \\xZZ'), null);
  assert.equal(parseInput('x'.repeat(100))?.length, 64);
  assert.deepEqual(parseInput('ж'), Uint8Array.from([0xd0, 0xb6]));
  assert.deepEqual(parseInput('✦🌟'), Uint8Array.from([0xe2, 0x9c, 0xa6, 0xf0, 0x9f, 0x8c, 0x9f]));
  assert.equal(escapeInput(MINIMAL_CRASH), 'ORI\\x02\\x01\\x00\\x00\\x00\\x00N\\x01');
  assert.deepEqual(parseInput(escapeInput(MINIMAL_CRASH)), MINIMAL_CRASH);
});

test('numbers', () => {
  assert.equal(count(1284551, 'en'), '1,284,551');
  assert.equal(rate(2_140_000, 'en'), '2.1 M');
  assert.equal(rate(48_200, 'en'), '48 k');
  assert.equal(seconds(4, 'en'), '0.004 s');
  assert.equal(seconds(620, 'en'), '0.62 s');
});

test('panic message split', () => {
  const p = splitPanic('panicked at crates/fuzzer/src/target.rs:129:42:\nrange end index 12 out of range for slice of length 11');
  assert.equal(p.location, 'crates/fuzzer/src/target.rs:129:42');
  assert.equal(p.text, 'range end index 12 out of range for slice of length 11');
});

test('masks', () => {
  assert.deepEqual(maskToPoints(0b101, 0), [0, 2]);
  assert.deepEqual(maskToPoints(0, 0b1000000), [38]);
});

test('every point has a place in the sky, and the module agrees on how many', () => {
  const run = new FuzzRun(module, { mode: 'guided', build: 'fixed', seed: [1, 0] });
  assert.equal(run.points, POINTS);
  const { points } = layout();
  assert.equal(points.length, POINTS);
  for (const p of points) assert.ok(p.x > 0 && p.x < SKY.width && p.y > 0 && p.y < SKY.height, `point ${p.id} off the sky`);
});

test('the strings file and the island agree, both ways', () => {
  const ftl = new Set([...read('../../../content/en/islands/fuzzer.ftl').matchAll(/^(fuzzer-[a-z0-9-]+) =/gm)].map((m) => m[1]!));
  const used = new Set([...read('./index.ts').matchAll(/'(fuzzer-[a-z0-9-]+)'/g)].map((m) => m[1]!));
  for (let i = 0; i < POINTS; i++) used.add(`fuzzer-point-${i}`);
  for (const code of PARSE_CODES.slice(1)) used.add(`fuzzer-reason-${code}`);
  for (const name of Object.keys(STARS)) used.add(`fuzzer-star-${name.toLowerCase()}`);
  for (const key of used) assert.ok(ftl.has(key), `${key} is used but not in fuzzer.ftl`);
  for (const key of ftl) assert.ok(used.has(key), `${key} is in fuzzer.ftl but never used`);
});

test('guided fuzzing crashes the buggy build with a real Rust panic', () => {
  const run = new FuzzRun(module, { mode: 'guided', build: 'buggy', seed: [1, 0] });
  let discoveries = 0;
  while (!run.crash && run.execs < 5_000_000) discoveries += run.step(10_000).discoveries.length;
  const crash = run.crash;
  if (!crash) throw new Error('no crash in 5M inputs');
  assert.ok(discoveries > 10);
  const { location, text } = splitPanic(crash.message);
  assert.match(location, /^crates\/fuzzer\/src\/target\.rs:\d+:\d+$/);
  assert.match(text, /^range end index \d+ out of range for slice of length \d+$/);
  assert.equal(crash.exec, run.execs);
  assert.deepEqual(run.step(10), { discoveries: [], crash: null }, 'a spent run does nothing more');
});

test('the panic points at the buggy line, and the fix is two lines below', () => {
  const result = new Probe(module).run(MINIMAL_CRASH, 'buggy');
  assert.equal(result.code, 'crash');
  const { location, text } = splitPanic(result.message ?? '');
  assert.equal(text, 'range end index 12 out of range for slice of length 11');
  const line = Number(location.split(':').at(-2));
  const source = read('../../../crates/fuzzer/src/target.rs').split('\n');
  assert.match(source[line - 1]!, /Build::Buggy => &data\[start\.\.start \+ len\],/);
  assert.match(source[line + 1]!, /Build::Fixed => data\.get\(start\.\.start \+ len\)\.ok_or\(ParseError::Truncated\)\?,/);
  // The crash card prints both lines verbatim.
  const island = read('./index.ts');
  assert.ok(island.includes('Build::Buggy => &data[start..start + len],'));
  assert.ok(island.includes('Build::Fixed => data.get(start..start + len).ok_or(ParseError::Truncated)?,'));
});

test('minimisation shrinks crashes to a 1-minimal core', () => {
  const probe = new Probe(module);
  for (const seed of [1, 2, 3, 4, 5, 6]) {
    const run = new FuzzRun(module, { mode: 'guided', build: 'buggy', seed: [seed, 0] });
    while (!run.crash) run.step(10_000);
    const min = minimize(run.crash.input, (b) => probe.crashes(b));
    const where = `seed ${seed}: ${hex(min)}`;
    assert.ok(probe.crashes(min), `${where} no longer crashes`);
    assert.ok(min.length <= 4 * 7 + 7, where);
    assert.deepEqual(Array.from(min.subarray(0, 4)), [0x4f, 0x52, 0x49, 0x02], where);
    for (let i = 0; i < min.length; i++) {
      const smaller = Uint8Array.from([...min.subarray(0, i), ...min.subarray(i + 1)]);
      assert.ok(!probe.crashes(smaller), `${where}: still crashes without byte ${i}`);
    }
  }
  assert.deepEqual(minimize(MINIMAL_CRASH, (b) => probe.crashes(b)), MINIMAL_CRASH);
});

test('the probe reports parse results and survives crashes', () => {
  const probe = new Probe(module);
  assert.equal(probe.run(Uint8Array.from([0x4f, 0x52]), 'buggy').code, 'bad-magic');
  assert.equal(probe.run(MINIMAL_CRASH, 'buggy').code, 'crash');
  assert.equal(probe.run(MINIMAL_CRASH, 'fixed').code, 'truncated');
  const ok = probe.run(parseInput('ORI\\x02\\x01\\x28\\x1E\\x05\\x09N\\x05ORION')!, 'buggy');
  assert.equal(ok.code, 'ok');
  assert.ok(ok.points.includes(27), 'a bright variable supergiant is a Betelgeuse');
});

test('the fixed build survives a million guided inputs', () => {
  const run = new FuzzRun(module, { mode: 'guided', build: 'fixed', seed: [7, 0] });
  while (run.execs < 1_000_000) assert.equal(run.step(50_000).crash, null);
  assert.ok(run.covered.length >= 30, `only ${run.covered.length} points`);
});

test('blind fuzzing stays in the header', () => {
  const run = new FuzzRun(module, { mode: 'blind', build: 'buggy', seed: [5, 0] });
  while (run.execs < 1_000_000) assert.equal(run.step(50_000).crash, null);
  assert.ok(run.covered.every((p) => p <= 2), `blind reached ${run.covered.join(', ')}`);
});
