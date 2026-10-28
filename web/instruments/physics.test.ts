// @ts-ignore -- Node built-in; this repo carries no @types/node.
import { test } from 'node:test';
// @ts-ignore -- Node built-in.
import assert from 'node:assert/strict';
import { ceilKm, FIBRE, floorMs, fromNavigation, haversine, judge, lightFloor, msText, reach } from './physics.ts';
import type { Place } from './tz/tzdb.ts';

const near = (actual: number, expected: number, eps: number) =>
  assert.ok(Math.abs(actual - expected) <= eps, `${actual} is not within ${eps} of ${expected}`);

// Server location from the /_proof example, and Frankfurt's city centre.
const dushanbe = { lat: 38.5598, lon: 68.787 };
const frankfurt: Place = { zone: 'Europe/Frankfurt', city: 'Frankfurt', lat: 50.1109, lon: 8.6821 };

test('light in fibre: c ÷ 1.468 ≈ 204.2 km per ms', () => {
  near(FIBRE, 204.2183, 1e-4);
  near(lightFloor(1), 2 / 204.2183, 1e-7); // ≈ 9.79 µs per km of distance, round trip
});

test('haversine: exact cases on the mean sphere (R = 6371.0088 km)', () => {
  const R = 6371.0088;
  near(haversine(dushanbe, dushanbe), 0, 1e-9);
  near(haversine({ lat: 0, lon: 0 }, { lat: 0, lon: 1 }), (Math.PI * R) / 180, 1e-6); // 111.195 km
  near(haversine({ lat: 0, lon: 0 }, { lat: 90, lon: 0 }), (Math.PI * R) / 2, 1e-6);
  near(haversine({ lat: 0, lon: 0 }, { lat: 0, lon: 180 }), Math.PI * R, 1e-6); // antipodes
  near(haversine({ lat: 10, lon: 170 }, { lat: 10, lon: -170 }), haversine({ lat: 10, lon: 0 }, { lat: 10, lon: 20 }), 1e-6); // across the date line
});

test('haversine: Dushanbe to Frankfurt', () => {
  // Computed independently (spherical law of cosines, R = 6371.0088 km): 4810.64 km.
  // An ellipsoidal reference (Vincenty, WGS84) gives 4822.66 km — the sphere is within 0.3 %.
  const d = haversine(dushanbe, frankfurt);
  near(d, 4810.64, 0.05);
  near(haversine(frankfurt, dushanbe), d, 1e-9);
  near(lightFloor(d), 47.113, 0.001);
});

test('upper bound: half the round trip at the speed of light in glass', () => {
  near(reach(53), 26.5 * 204.2183, 1e-3); // 5411.8 km
  assert.equal(ceilKm(reach(53)), 5420); // rounded up, so "at most 5,420 km" stays true
  near(reach(0.05), 5.1055, 1e-4);
  assert.equal(ceilKm(reach(0.05)), 5.2);
});

test('display rounding never weakens a bound', () => {
  near(ceilKm(5.11), 5.2, 1e-12);
  near(ceilKm(0.3), 0.3, 1e-12); // 0.3 / 0.1 is 2.9999…, not 3
  assert.equal(ceilKm(999.2), 1000);
  assert.equal(ceilKm(1000), 1000);
  assert.equal(ceilKm(1001), 1010);
  assert.equal(floorMs(47.113), 47.1);
  assert.equal(floorMs(43.69), 43.6);
  assert.equal(floorMs(9.99), 9.9);
  assert.equal(floorMs(123.4), 123.4);
  near(floorMs(0.3), 0.3, 1e-12);
  near(floorMs(0.0215), 0.021, 1e-12);
  near(floorMs(0.00456), 0.0045, 1e-12);
  for (const x of [0.00123, 0.0456, 0.3, 0.789, 4.56, 4.7, 47.9, 123.4, 5411.78]) {
    assert.ok(floorMs(x) <= x && ceilKm(x) >= x, String(x));
  }
});

test('display: µs below 1 ms, else ms with one decimal', () => {
  assert.deepEqual(msText(0.021, 'en'), { value: '21', unit: 'µs' });
  assert.deepEqual(msText(0.0045, 'en'), { value: '4.5', unit: 'µs' });
  assert.deepEqual(msText(48.21, 'en'), { value: '48.2', unit: 'ms' });
  assert.deepEqual(msText(53, 'en'), { value: '53.0', unit: 'ms' });
  assert.deepEqual(msText(1234.56, 'en'), { value: '1,234.6', unit: 'ms' });
  assert.deepEqual(msText(48.21, 'ru'), { value: '48,2', unit: 'ms' });
});

test('display: floor + detours, as shown, add up to the round trip as shown', () => {
  const shown = (x: number) => {
    const { value, unit } = msText(x, 'en');
    return Number(value.replace(/,/g, '')) / (unit === 'µs' ? 1000 : 1);
  };
  for (const [rtt, floor] of [
    [48.21, 43.69],
    [53, 47.113],
    [60.04, 25.23],
    [2.2, 0.0245],
    [100.05, 99.95],
  ] as const) {
    const f = floorMs(floor);
    // Exact when both parts are in ms; within half a display step when one is in µs.
    near(shown(f) + shown(rtt - f), shown(rtt), f < 1 || rtt - f < 1 ? 0.05 : 1e-9);
  }
});

test('floor branch: the parts add up to the measured round trip', () => {
  const v = judge(53, frankfurt, dushanbe);
  assert.equal(v.kind, 'floor');
  if (v.kind !== 'floor') return;
  near(v.distance, 4810.64, 0.05);
  near(v.floor, 47.113, 0.001);
  near(v.floor + v.detours, 53, 1e-9);
  assert.ok(v.detours > 0);
});

test('faster-than-light branch: the time zone city is too far away', () => {
  const v = judge(20, frankfurt, dushanbe); // light needs 47 ms from Frankfurt; 20 ms is impossible
  assert.equal(v.kind, 'faster');
  if (v.kind !== 'faster') return;
  assert.ok(v.floor > 20);
  assert.equal(v.place.city, 'Frankfurt');
  assert.ok(reach(20) < v.distance, 'the bound proves the visitor is closer than the city');
  // Exactly at the floor is still possible.
  assert.equal(judge(lightFloor(haversine(frankfurt, dushanbe)), frankfurt, dushanbe).kind, 'floor');
});

test('branches without a floor', () => {
  assert.equal(judge(53, frankfurt, null).kind, 'no-server'); // server.location not configured
  assert.equal(judge(53, frankfurt, undefined).kind, 'no-proof'); // /_proof failed
  assert.equal(judge(53, null, dushanbe).kind, 'no-zone'); // UTC or an unknown zone
});

const nav = (fields: Record<string, number>, timings: Record<string, number> = {}) =>
  ({
    requestStart: 100,
    responseStart: 153.4,
    transferSize: 13_000,
    decodedBodySize: 40_000,
    ...fields,
    serverTiming: Object.entries(timings).map(([name, duration]) => ({ name, duration, description: '' })),
  }) as unknown as PerformanceNavigationTiming;

test('round trip from the navigation entry: kernel first, then the browser, never a cache hit', () => {
  assert.deepEqual(fromNavigation(nav({}, { app: 0.4, krtt: 48.21 })), { rtt: 48.21, server: 0.4, source: 'kernel' });
  const browser = fromNavigation(nav({}, { app: 0.4 }));
  assert.equal(browser?.source, 'navigation');
  near(browser!.rtt, 53, 1e-9);
  assert.equal(fromNavigation(nav({ transferSize: 0 }, { app: 0.4 })), undefined); // from the HTTP cache
  assert.equal(fromNavigation(nav({ requestStart: 0, responseStart: 0 })), undefined);
  assert.equal(fromNavigation(undefined), undefined);
  assert.equal(fromNavigation(nav({}))?.server, undefined); // no Server-Timing: the whole wait counts
});
