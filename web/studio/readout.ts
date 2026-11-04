/**
 * This page's own numbers, from the reader's browser: the compressed size of the page and of
 * everything it loaded, how much of that is JavaScript, how many requests went to anyone but us,
 * and when the first byte and the first paint arrived. Nothing is sent anywhere. Files that load
 * later (portraits, as they scroll in) join the totals as they arrive.
 */

import { numberLocale } from './math.ts';

export function readout(lang: string): void {
  const box = document.querySelector<HTMLElement>('.readout');
  if (!box || typeof PerformanceObserver === 'undefined') return;
  const locale = numberLocale(lang);
  const kb = new Intl.NumberFormat(locale, { minimumFractionDigits: 1, maximumFractionDigits: 1 });
  const whole = new Intl.NumberFormat(locale, { maximumFractionDigits: 0 });
  const set = (key: string, text: string) => {
    const el = box.querySelector<HTMLElement>(`[data-m="${key}"]`);
    if (el && el.textContent !== text) el.textContent = text;
  };
  // Compressed body bytes: what crossed the network for each file (also known for cached files).
  const size = (e: PerformanceResourceTiming) => e.encodedBodySize || e.transferSize || 0;

  const render = () => {
    const nav = performance.getEntriesByType('navigation')[0] as PerformanceNavigationTiming | undefined;
    const html = nav ? size(nav) : 0;
    let total = html;
    let js = 0;
    let third = 0;
    for (const e of performance.getEntriesByType('resource') as PerformanceResourceTiming[]) {
      const b = size(e);
      total += b;
      if (e.initiatorType === 'script' || /\.m?js(?:$|\?)/.test(e.name)) js += b;
      try {
        if (new URL(e.name).origin !== location.origin) third++;
      } catch {
        third++;
      }
    }
    if (html) set('html', kb.format(html / 1000));
    set('total', kb.format(total / 1000));
    set('js', kb.format(js / 1000));
    set('third', whole.format(third));
    if (nav && nav.responseStart > 0) set('ttfb', whole.format(nav.responseStart - nav.startTime));
    const fcp = performance.getEntriesByName('first-contentful-paint')[0];
    if (fcp) set('fcp', whole.format(fcp.startTime));
    box.hidden = false;
  };

  const start = () => {
    render();
    try {
      new PerformanceObserver(render).observe({ type: 'resource' });
    } catch {
      // An old browser without observer types: the first reading stands.
    }
  };
  if (document.readyState === 'complete') setTimeout(start, 250);
  else addEventListener('load', () => setTimeout(start, 250), { once: true });
}
