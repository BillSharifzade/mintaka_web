// Pure helpers for the terminal (web/terminal/index.ts). No DOM, so Node tests can import them.

export interface Stats {
  min: number;
  median: number;
  p90: number;
  max: number;
}

/** min, median, 90th percentile (nearest rank: with 10 samples, the 9th smallest) and max. */
export function stats(samples: readonly number[]): Stats {
  const s = [...samples].sort((a, b) => a - b);
  const n = s.length;
  if (!n) throw new RangeError('no samples');
  const mid = n >> 1;
  return {
    min: s[0]!,
    median: n % 2 ? s[mid]! : (s[mid - 1]! + s[mid]!) / 2,
    p90: s[Math.ceil(0.9 * n) - 1]!,
    max: s[n - 1]!,
  };
}

/** "  LANG  tg " → ["lang", "tg"]. */
export function parse(line: string): [string, string] {
  const [cmd = '', ...rest] = line.trim().split(/\s+/);
  return [cmd.toLowerCase(), rest.join(' ')];
}

/** Complete the last word of `line` from `words`: one match completes it, several extend it to their common prefix. */
export function complete(line: string, words: readonly string[]): { line: string; options: string[] } {
  const cut = line.lastIndexOf(' ') + 1;
  const word = line.slice(cut).toLowerCase();
  const options = words.filter((w) => w.startsWith(word));
  if (!options.length) return { line, options };
  let common = options[0]!;
  for (const o of options) while (!o.startsWith(common)) common = common.slice(0, -1);
  return { line: line.slice(0, cut) + common + (options.length === 1 ? ' ' : ''), options };
}

/** Rows as two aligned columns. */
export function table(rows: readonly (readonly [string, string])[], indent = ''): string {
  const width = Math.max(0, ...rows.map(([k]) => k.length)) + 2;
  return rows.map(([k, v]) => indent + k.padEnd(width) + v).join('\n');
}

/** `{ a: { b: 1 }, c: [2, 3] }` → [["a.b", "1"], ["c", "2, 3"]]. */
export function flatten(value: unknown, prefix = ''): [string, string][] {
  if (value === null || typeof value !== 'object') return [[prefix, String(value)]];
  if (Array.isArray(value)) return [[prefix, value.join(', ')]];
  return Object.entries(value).flatMap(([k, v]) => flatten(v, prefix ? `${prefix}.${k}` : k));
}

const UA: [RegExp, string][] = [
  [/Edg(?:e|A|iOS)?\/(\d+)/, 'Microsoft Edge'],
  [/OPR\/(\d+)/, 'Opera'],
  [/SamsungBrowser\/(\d+)/, 'Samsung Internet'],
  [/(?:Firefox|FxiOS)\/(\d+)/, 'Firefox'],
  [/(?:Chrome|CriOS)\/(\d+)/, 'Chrome'],
  [/Version\/(\d+(?:\.\d+)?).*Safari/, 'Safari'],
];

/** Browser name and major version: User-Agent Client Hints if present, else a short UA parse. */
export function browser(brands: readonly { brand: string; version: string }[] | undefined, ua: string): string {
  const real = (brands ?? []).filter((b) => !/not.?a.?brand/i.test(b.brand));
  const pick = real.find((b) => b.brand !== 'Chromium') ?? real[0];
  if (pick) return `${pick.brand} ${pick.version}`;
  for (const [re, name] of UA) {
    const m = re.exec(ua);
    if (m) return `${name} ${m[1]}`;
  }
  return '';
}

/** Operating system: Client Hints' platform if present, else from the UA string. */
export function platform(hint: string | undefined, ua: string): string {
  if (hint) return hint;
  const os: [RegExp, string][] = [
    [/Android/, 'Android'],
    [/iPhone|iPad|iPod/, 'iOS'],
    [/CrOS/, 'ChromeOS'],
    [/Mac OS X/, 'macOS'],
    [/Windows/, 'Windows'],
    [/Linux/, 'Linux'],
  ];
  return os.find(([re]) => re.test(ua))?.[1] ?? '';
}

type Nav = Pick<
  PerformanceNavigationTiming,
  | 'startTime'
  | 'domainLookupStart'
  | 'domainLookupEnd'
  | 'connectStart'
  | 'connectEnd'
  | 'secureConnectionStart'
  | 'requestStart'
  | 'responseStart'
  | 'responseEnd'
>;

/** A hop of the page request: message key, duration in ms (undefined: none), note key. */
export type Hop = [key: string, ms: number | undefined, note?: string];

/** The navigation request hop by hop (Navigation Timing), with the server's Server-Timing inside. */
export function hops(nav: Nav, timing: (name: string) => number | undefined, secure: boolean, reused: boolean): Hop[] {
  const out: Hop[] = [
    ['dns', nav.domainLookupEnd - nav.domainLookupStart, reused ? 'reused' : undefined],
    ['connect', nav.connectEnd - nav.connectStart, reused ? 'reused' : undefined],
  ];
  if (!secure) out.push(['tls', undefined, 'plain']);
  else if (reused) out.push(['tls', 0, 'reused']);
  else out.push(['tls', nav.secureConnectionStart > 0 ? nav.connectEnd - nav.secureConnectionStart : undefined]);
  out.push(['wait', nav.responseStart - nav.requestStart]);
  for (const name of ['app', 'krtt', 'tls']) {
    const ms = timing(name);
    if (ms !== undefined) out.push(['st-' + name, ms]);
  }
  out.push(['download', nav.responseEnd - nav.responseStart], ['total', nav.responseEnd - nav.startTime]);
  return out;
}
