// What the browser measured about this page view. Everything here is read-only and local.

export function navEntry(): PerformanceNavigationTiming | undefined {
  return performance.getEntriesByType('navigation')[0] as PerformanceNavigationTiming | undefined;
}

/**
 * True if this page came from the service worker's offline copy instead of our server: the copy
 * carries `Server-Timing: sw` in place of the server's own timings (docs/CONTRACTS.md §15), and a
 * browser that drops it still shows a worker-served page with no server timings at all.
 */
export function offlineCopy(nav = navEntry()): boolean {
  const st = nav?.serverTiming;
  if (!nav || !st) return false;
  return st.some((m) => m.name === 'sw') || (nav.workerStart > 0 && st.length === 0);
}

/**
 * A `Server-Timing` metric from a response, in milliseconds.
 * Names the server sends: `app` (total), `route`, `render`, `tls` (first request on a TLS
 * connection only) and `krtt` (kernel smoothed RTT, when available). See docs/CONTRACTS.md.
 */
export function serverTiming(name: string, entry: PerformanceResourceTiming | undefined = navEntry()): number | undefined {
  const metric = entry?.serverTiming?.find((m) => m.name === name);
  return metric ? metric.duration : undefined;
}

export interface PageWeight {
  /** Bytes over the network for the document and every subresource so far (0 for cache hits). */
  transferred: number;
  /** Decoded body bytes, cache hits included. */
  decoded: number;
  requests: number;
  /** Requests served from the HTTP cache (transferSize 0 but a body). */
  fromCache: number;
  /** Origins other than ours that the page contacted. Should always be empty. */
  thirdPartyOrigins: string[];
}

export function pageWeight(): PageWeight {
  const entries: PerformanceResourceTiming[] = [];
  const nav = navEntry();
  if (nav) entries.push(nav);
  entries.push(...(performance.getEntriesByType('resource') as PerformanceResourceTiming[]));
  const third = new Set<string>();
  let transferred = 0;
  let decoded = 0;
  let fromCache = 0;
  for (const e of entries) {
    transferred += e.transferSize;
    decoded += e.decodedBodySize;
    if (e.transferSize === 0 && e.decodedBodySize > 0) fromCache++;
    try {
      const origin = new URL(e.name).origin;
      if (origin !== location.origin) third.add(origin);
    } catch {
      /* about:blank and friends */
    }
  }
  return { transferred, decoded, requests: entries.length, fromCache, thirdPartyOrigins: [...third] };
}

export interface PingSample {
  /** requestStart → responseStart, ms: one network round trip plus the server's own time. */
  total: number;
  /** Server-Timing `app` of the response, ms. */
  app: number | undefined;
  /** Server-Timing `krtt` (kernel RTT of the connection), ms. */
  krtt: number | undefined;
  /** False if the browser kept no Resource Timing entry; `total` is then wall-clock time. */
  timed: boolean;
}

/** One `GET /_ping` (204, no-store), timed by the browser's Resource Timing. */
export async function ping(): Promise<PingSample> {
  const url = new URL('/_ping', location.href).href;
  const t0 = performance.now();
  const res = await fetch(url, { cache: 'no-store' });
  await res.arrayBuffer();
  const wall = performance.now() - t0;
  if (!res.ok) throw new Error(`/_ping answered ${res.status}`);
  let entry: PerformanceResourceTiming | undefined;
  for (let i = 0; i < 3 && !entry; i++) {
    entry = (performance.getEntriesByName(url, 'resource') as PerformanceResourceTiming[]).findLast((e) => e.startTime >= t0);
    if (!entry) await new Promise((r) => setTimeout(r, 0));
  }
  const timed = !!entry && entry.requestStart > 0 && entry.responseStart >= entry.requestStart;
  return {
    total: timed ? entry!.responseStart - entry!.requestStart : wall,
    app: serverTiming('app', entry),
    krtt: serverTiming('krtt', entry),
    timed,
  };
}

/** True if the document request reused an existing connection (no DNS, TCP or TLS cost). */
export function connectionReused(nav = navEntry()): boolean {
  if (!nav) return false;
  return nav.connectEnd === nav.connectStart && nav.domainLookupEnd === nav.domainLookupStart;
}
