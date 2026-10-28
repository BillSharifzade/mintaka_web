// Reading the IANA tz database: zone coordinates (zone1970.tab, zone.tab) and links (tzdata.zi).
// Used at build time by gen.ts, and at run time by the S2 instrument (`locate`). Pure functions.

/** Latitude, longitude in decimal degrees (north and east positive). */
export type LatLon = readonly [number, number];

/** A time zone resolved to its principal location. */
export interface Place {
  /** The zone whose location this is (an alias is resolved to its target). */
  zone: string;
  /** Human name from the zone id: "America/Argentina/Buenos_Aires" → "Buenos Aires". */
  city: string;
  lat: number;
  lon: number;
}

/** What gen.ts writes: zone → location, alias → zone. */
export type Table = Readonly<Record<string, LatLon | string>>;

const ISO6709 = /^([+-])(\d{2})(\d{2})(\d{2})?([+-])(\d{3})(\d{2})(\d{2})?$/;

function dms(sign: string, d: string, m: string, s: string | undefined, max: number, text: string): number {
  const mins = Number(m);
  const secs = s === undefined ? 0 : Number(s);
  const deg = Number(d) + mins / 60 + secs / 3600;
  if (mins >= 60 || secs >= 60 || deg > max) throw new RangeError(`coordinate out of range: ${text}`);
  return sign === '-' ? -deg : deg;
}

/** ISO 6709 sign-degrees-minutes[-seconds], as in zone.tab: "+3835+06848", "+404251-0740023". */
export function parseIso6709(text: string): LatLon {
  const m = ISO6709.exec(text);
  if (!m) throw new SyntaxError(`not an ISO 6709 coordinate: ${text}`);
  const [, ls, ld, lm, lsec, os, od, om, osec] = m as unknown as string[];
  return [dms(ls!, ld!, lm!, lsec, 90, text), dms(os!, od!, om!, osec, 180, text)];
}

/** Rows of zone1970.tab or zone.tab (tab-separated; column 2 coordinates, column 3 zone). */
export function parseZoneTab(text: string): Map<string, LatLon> {
  const out = new Map<string, LatLon>();
  for (const line of text.split('\n')) {
    if (!line || line.startsWith('#')) continue;
    const [, coords, zone] = line.split('\t');
    if (coords && zone) out.set(zone, parseIso6709(coords));
  }
  return out;
}

/** `L target alias` lines of tzdata.zi → alias → target. */
export function parseLinks(zi: string): Map<string, string> {
  const out = new Map<string, string>();
  for (const line of zi.split('\n')) {
    const [kind, target, alias] = line.trim().split(/\s+/);
    if (kind === 'L' && target && alias) out.set(alias, target);
  }
  return out;
}

/** "2026e" from tzdata.zi's first line, `# version 2026e`. */
export function parseVersion(zi: string): string {
  return /^#\s*version\s+(\S+)/.exec(zi)?.[1] ?? 'unknown';
}

/**
 * System V names kept for compatibility (`EST`, `CET`, `PST8PDT`, …). They name a set of clock
 * rules, not a place, even where tzdata links them to a city — so they get no location.
 */
export const isLegacyName = (alias: string): boolean =>
  /^(CET|EET|MET|WET|EST|MST|HST|EST5EDT|CST6CDT|MST7MDT|PST8PDT)$/.test(alias);

/**
 * tzdata merges zones that have agreed since 1970 even across borders, and tzdata.zi drops the
 * `#= original` notes of tzdb's `backward` file. For these old names the merged target is in
 * another country, so point them back at their original zone (which zone.tab still locates).
 * Several are the CLDR ids that browsers report (Africa/Asmera, America/Coral_Harbour, …).
 */
export const ORIGINAL_ZONE: Readonly<Record<string, string>> = {
  'Africa/Asmera': 'Africa/Asmara',
  'Africa/Timbuktu': 'Africa/Bamako',
  'America/Coral_Harbour': 'America/Atikokan',
  'Antarctica/South_Pole': 'Antarctica/McMurdo',
  'Atlantic/Jan_Mayen': 'Arctic/Longyearbyen',
  Iceland: 'Atlantic/Reykjavik',
  'Pacific/Ponape': 'Pacific/Pohnpei',
  'Pacific/Truk': 'Pacific/Chuuk',
  'Pacific/Yap': 'Pacific/Chuuk',
};

/**
 * Merge coordinate sources (earlier sources win) and resolve every link to the nearest zone in
 * its chain that has coordinates. Links into `Etc/*` (UTC, GMT…) have no place and are dropped.
 */
export function buildTable(
  sources: readonly ReadonlyMap<string, LatLon>[],
  links: ReadonlyMap<string, string>,
  original: Readonly<Record<string, string>> = ORIGINAL_ZONE,
): { zones: Map<string, LatLon>; aliases: Map<string, string> } {
  const zones = new Map<string, LatLon>();
  for (const source of sources) for (const [zone, at] of source) if (!zones.has(zone)) zones.set(zone, at);
  const aliases = new Map<string, string>();
  for (const alias of links.keys()) {
    if (zones.has(alias) || isLegacyName(alias)) continue;
    const seen = new Set([alias]);
    const first = Object.hasOwn(original, alias) ? original[alias] : undefined;
    let target = first !== undefined && zones.has(first) ? first : links.get(alias);
    while (target !== undefined && !zones.has(target) && !seen.has(target)) {
      seen.add(target);
      target = links.get(target);
    }
    if (target !== undefined && zones.has(target)) aliases.set(alias, target);
  }
  return { zones, aliases };
}

export const cityOf = (zone: string): string => (zone.split('/').pop() ?? zone).replace(/_/g, ' ');

/** Find a time zone (as `Intl.DateTimeFormat().resolvedOptions().timeZone` names it). */
export function locate(tz: string, table: Table): Place | null {
  let zone = tz;
  let v = Object.hasOwn(table, zone) ? table[zone] : undefined;
  if (typeof v === 'string') {
    zone = v;
    v = Object.hasOwn(table, zone) ? table[zone] : undefined;
  }
  if (!Array.isArray(v)) return null;
  const [lat, lon] = v as LatLon;
  return { zone, city: cityOf(zone), lat, lon };
}
