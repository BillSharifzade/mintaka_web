// @ts-ignore -- Node built-in; this repo carries no @types/node.
import { test } from 'node:test';
// @ts-ignore -- Node built-in.
import assert from 'node:assert/strict';
import { formatRaDec, project, unproject } from './astro.ts';

// Catalogue positions (J2000), as in crates/site/src/orion.rs.
const deg = (h: number, m: number, s: number) => (h + m / 60 + s / 3600) * 15;
const dms = (sign: number, d: number, m: number, s: number) => sign * (d + m / 60 + s / 3600);
const alnilam: [number, number] = [deg(5, 36, 12.8), dms(-1, 1, 12, 7)];
const betelgeuse: [number, number] = [deg(5, 55, 10.3), dms(1, 7, 24, 25)];
const rigel: [number, number] = [deg(5, 14, 32.3), dms(-1, 8, 12, 6)];

test('the projection matches the generator: Betelgeuse upper left, Rigel lower right, 18.6° apart', () => {
  const [bx, by] = project(...betelgeuse, alnilam);
  const [rx, ry] = project(...rigel, alnilam);
  assert.ok(bx < 0 && by < 0);
  assert.ok(rx > 0 && ry > 0);
  const d = Math.hypot(bx - rx, by - ry);
  assert.ok(d > 18 && d < 19.5, String(d));
});

test('unproject inverts project to well under an arcsecond', () => {
  for (const star of [betelgeuse, rigel, alnilam]) {
    const [ra, dec] = unproject(...project(...star, alnilam), alnilam);
    assert.ok(Math.abs(ra - star[0]) * 3600 < 0.1, `ra ${ra} vs ${star[0]}`);
    assert.ok(Math.abs(dec - star[1]) * 3600 < 0.1, `dec ${dec} vs ${star[1]}`);
  }
});

test('the readout format', () => {
  assert.equal(formatRaDec(...betelgeuse), 'RA 5h 55m · Dec +7° 24′');
  assert.equal(formatRaDec(...rigel), 'RA 5h 14m · Dec −8° 12′');
});
