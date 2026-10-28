// @ts-ignore -- Node built-in; this repo carries no @types/node.
import { test } from 'node:test';
// @ts-ignore -- Node built-in.
import assert from 'node:assert/strict';
import { buildTable, cityOf, isLegacyName, locate, parseIso6709, parseLinks, parseVersion, parseZoneTab, type LatLon } from './tzdb.ts';
import { version, zones } from './data.ts';

const near = (actual: number, expected: number, eps = 1e-9) =>
  assert.ok(Math.abs(actual - expected) <= eps, `${actual} is not within ${eps} of ${expected}`);

test('ISO 6709 ±DDMM±DDDMM', () => {
  const [lat, lon] = parseIso6709('+3835+06848'); // Asia/Dushanbe
  near(lat, 38 + 35 / 60);
  near(lon, 68 + 48 / 60);
  const [s, w] = parseIso6709('-3352+15113'); // Australia/Sydney
  near(s, -(33 + 52 / 60));
  near(w, 151 + 13 / 60);
  const [lat0, lon0] = parseIso6709('+0000-00000');
  near(lat0, 0);
  near(lon0, 0);
});

test('ISO 6709 ±DDMMSS±DDDMMSS', () => {
  const [lat, lon] = parseIso6709('+404251-0740023'); // America/New_York
  near(lat, 40 + 42 / 60 + 51 / 3600);
  near(lon, -(74 + 0 / 60 + 23 / 3600));
  const [lat2, lon2] = parseIso6709('-690022+0393524'); // Antarctica/Syowa
  near(lat2, -(69 + 0 / 60 + 22 / 3600));
  near(lon2, 39 + 35 / 60 + 24 / 3600);
});

test('ISO 6709 rejects malformed and out-of-range values', () => {
  for (const bad of ['3835+06848', '+3835 06848', '+385+06848', '+3835+0684', '+3860+06848', '+3835+06860', '+383560+06848', '+9001+00000', '+0000+18001', '']) {
    assert.throws(() => parseIso6709(bad), `${JSON.stringify(bad)} should not parse`);
  }
  assert.deepEqual(parseIso6709('+9000+18000'), [90, 180]);
});

const TAB = `# tzdb timezone descriptions
#country-
#codes\tcoordinates\tTZ\tcomments
TJ\t+3835+06848\tAsia/Dushanbe
DE,DK,NO,SE,SJ\t+5230+01322\tEurope/Berlin\tmost of Germany
US\t+404251-0740023\tAmerica/New_York\tEastern (most areas)
`;

test('zone.tab rows: comments skipped, coordinates parsed', () => {
  const rows = parseZoneTab(TAB);
  assert.equal(rows.size, 3);
  assert.deepEqual(rows.get('Asia/Dushanbe'), parseIso6709('+3835+06848'));
  assert.ok(rows.has('America/New_York'));
});

test('tzdata.zi: version and link lines', () => {
  const zi = '# version 2026e\n# redo posix_only\nZ Asia/Dushanbe 4:35:12 - LMT 1924 May 2\nR x 1 2 - Ja 1 0 0 -\nL Asia/Kolkata Asia/Calcutta\nL Etc/UTC UTC\n';
  assert.equal(parseVersion(zi), '2026e');
  assert.equal(parseVersion('no header'), 'unknown');
  const links = parseLinks(zi);
  assert.equal(links.size, 2);
  assert.equal(links.get('Asia/Calcutta'), 'Asia/Kolkata');
});

test('alias resolution: chains, cycles, places without coordinates, legacy names, overrides', () => {
  const at = (lat: number, lon: number): LatLon => [lat, lon];
  const zone1970 = new Map([
    ['Asia/Kolkata', at(22.53, 88.37)],
    ['Europe/Berlin', at(52.5, 13.37)],
    ['Africa/Nairobi', at(-1.28, 36.82)],
  ]);
  const zoneTab = new Map([
    ['Europe/Oslo', at(59.92, 10.75)], // tzdata links it to Berlin, but zone.tab knows where Oslo is
    ['Europe/Berlin', at(0, 0)], // earlier source wins
    ['Africa/Asmara', at(15.33, 38.88)],
  ]);
  const links = new Map([
    ['Asia/Calcutta', 'Asia/Kolkata'],
    ['Old/Name', 'Asia/Calcutta'], // chain: Old/Name → Asia/Calcutta → Asia/Kolkata
    ['Europe/Oslo', 'Europe/Berlin'],
    ['Loop/A', 'Loop/B'],
    ['Loop/B', 'Loop/A'],
    ['UTC', 'Etc/UTC'],
    ['CET', 'Europe/Berlin'],
    ['Africa/Asmara', 'Africa/Nairobi'],
    ['Africa/Asmera', 'Africa/Nairobi'], // tzdata's flattened link; its original zone is Africa/Asmara
  ]);
  const { zones: z, aliases } = buildTable([zone1970, zoneTab], links);
  assert.deepEqual(z.get('Europe/Berlin'), [52.5, 13.37]);
  assert.deepEqual(z.get('Europe/Oslo'), [59.92, 10.75]);
  assert.equal(aliases.get('Asia/Calcutta'), 'Asia/Kolkata');
  assert.equal(aliases.get('Old/Name'), 'Asia/Kolkata');
  assert.equal(aliases.has('Europe/Oslo'), false, 'a zone with its own coordinates is not an alias');
  assert.equal(aliases.has('Loop/A'), false);
  assert.equal(aliases.has('UTC'), false);
  assert.equal(aliases.has('CET'), false);
  assert.equal(aliases.get('Africa/Asmera'), 'Africa/Asmara');
  // Without the override the flattened link is followed as written.
  assert.equal(buildTable([zone1970, zoneTab], links, {}).aliases.get('Africa/Asmera'), 'Africa/Nairobi');
  assert.ok(isLegacyName('PST8PDT') && isLegacyName('EST') && !isLegacyName('Japan'));
});

test('locate: zones, aliases and names that are not places', () => {
  const table = { 'Asia/Kolkata': [22.53, 88.37] as LatLon, 'Asia/Calcutta': 'Asia/Kolkata' };
  assert.deepEqual(locate('Asia/Kolkata', table), { zone: 'Asia/Kolkata', city: 'Kolkata', lat: 22.53, lon: 88.37 });
  assert.deepEqual(locate('Asia/Calcutta', table), { zone: 'Asia/Kolkata', city: 'Kolkata', lat: 22.53, lon: 88.37 });
  for (const name of ['UTC', 'Etc/GMT-3', '', 'constructor', '__proto__', 'toString', 'hasOwnProperty']) {
    assert.equal(locate(name, table), null, name);
  }
  assert.equal(cityOf('America/Argentina/Buenos_Aires'), 'Buenos Aires');
  assert.equal(cityOf('America/Port-au-Prince'), 'Port-au-Prince');
  assert.equal(cityOf('Japan'), 'Japan');
});

test('generated data: sane, complete, aliases resolved', () => {
  assert.match(version, /^\d{4}[a-z]+$/);
  const dushanbe = locate('Asia/Dushanbe', zones);
  assert.ok(dushanbe && Math.abs(dushanbe.lat - 38.58) < 0.01 && Math.abs(dushanbe.lon - 68.8) < 0.01);
  assert.equal(locate('Asia/Calcutta', zones)?.zone, 'Asia/Kolkata');
  assert.equal(locate('Europe/Kiev', zones)?.city, 'Kyiv');
  assert.equal(locate('US/Eastern', zones)?.city, 'New York');
  assert.equal(locate('America/Coral_Harbour', zones)?.zone, 'America/Atikokan');
  assert.equal(locate('UTC', zones), null);
  assert.equal(locate('CET', zones), null);
  let places = 0;
  for (const [name, value] of Object.entries(zones)) {
    if (typeof value === 'string') {
      assert.ok(Array.isArray(zones[value]), `${name} → ${value} must point at a zone with coordinates`);
    } else {
      places++;
      const [lat, lon] = value;
      assert.ok(Math.abs(lat) <= 90 && Math.abs(lon) <= 180, name);
    }
  }
  assert.ok(places > 300, `only ${places} zones`);
});
