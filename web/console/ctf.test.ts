// The console challenge's client: reading a run token, and its strings.

// @ts-ignore -- Node built-in; this repo carries no @types/node.
import { test } from 'node:test';
// @ts-ignore -- Node built-in.
import assert from 'node:assert/strict';
// @ts-ignore -- Node built-in.
import { readFileSync } from 'node:fs';
import { STAGES, stageOf } from './ctf.ts';

const read = (path: string): string => readFileSync(new URL(path, import.meta.url), 'utf8');

/** `r1.` + base64url(nonce[16] ‖ stage ‖ tag[16]), as the server writes it. */
function token(stage: number, bytes = 33): string {
  const raw = Array.from({ length: bytes }, (_, i) => (i === 16 ? stage : (i * 37 + 11) & 0xff));
  return 'r1.' + btoa(String.fromCharCode(...raw)).replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/, '');
}

test('the stage is read from the token', () => {
  assert.equal(stageOf(token(1)), 1);
  assert.equal(stageOf(token(STAGES + 1)), 5);
  assert.equal(stageOf(token(2, 32)), null, 'wrong length');
  assert.equal(stageOf('r2.' + token(1).slice(3)), null, 'unknown version');
  assert.equal(stageOf('r1.%%%'), null);
  assert.equal(stageOf(''), null);
});

test('the strings (inline and lazy) and the module agree, both ways', () => {
  const files = read('../../content/en/islands/ctf.ftl') + read('../../content/en/islands/lazy/ctf.ftl');
  const ftl = new Set([...files.matchAll(/^(ctf-[a-z0-9-]+) =/gm)].map((m) => m[1]!));
  const src = read('./ctf.ts');
  const used = new Set([...src.matchAll(/'(ctf-[a-z0-9-]+)'/g)].map((m) => m[1]!));
  for (const star of ['rigel', 'saiph', 'betelgeuse', 'meissa']) for (const suffix of ['', '-title', '-hint']) used.add(`ctf-${star}${suffix}`);
  for (const reason of ['limited', 'failed']) used.add(`ctf-${reason}`);
  for (const key of used) assert.ok(ftl.has(key), `${key} is used but not in ctf.ftl`);
  for (const key of ftl) assert.ok(used.has(key), `${key} is in ctf.ftl but never used`);
});

test('every console help line has its string', () => {
  const ftl = read('../../content/en/islands/console.ftl');
  for (const [, key] of read('./index.ts').matchAll(/T\('([a-z-]+)'\)/g)) assert.match(ftl, new RegExp(`^console-${key} =`, 'm'), key);
});
