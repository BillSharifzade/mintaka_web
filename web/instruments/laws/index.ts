// S7 · Laws you can feel: three optional experiments behind tabs — Fitts's law, the Hick–Hyman
// law and pop-out search — and a card with the visitor's own numbers. Every measurement stays
// in this browser: this island makes no network requests at all.

import { $, h, render } from '../../shared/dom.ts';
import { fitts } from './fitts.ts';
import { hick } from './hick.ts';
import { L, num, type Api, type Experiment, type Report } from './kit.ts';
import { popout } from './popout.ts';

const EXPERIMENTS: readonly (readonly [id: string, build: Experiment])[] = [
  ['fitts', fitts],
  ['hick', hick],
  ['pop', popout],
];

export function mount(root: HTMLElement): void {
  const body = $('.instrument__body', root) ?? root.appendChild(h('div', { class: 'instrument__body' }));
  const badge = $('.instrument__status', root);
  if (badge) {
    badge.textContent = L('status');
    badge.className = 'instrument__status badge badge--info';
  }

  const announce = h('p', { class: 'visually-hidden', 'aria-live': 'polite' });
  const results = EXPERIMENTS.map(([id]) => h('div', { class: 'laws-result' }, h('p', { class: 'laws-result__name' }, L(id)), h('p', null, L('not-run'))));
  const tabs = EXPERIMENTS.map(([id], i) =>
    h(
      'button',
      {
        class: 'chip laws-tab',
        type: 'button',
        role: 'tab',
        id: `laws-tab-${id}`,
        'aria-controls': `laws-panel-${id}`,
        'aria-selected': 'false',
        tabindex: '-1',
        onclick: () => select(i),
      },
      L(id),
    ),
  );
  const panels = EXPERIMENTS.map(([id]) =>
    h('div', { class: 'laws-panel', role: 'tabpanel', id: `laws-panel-${id}`, 'aria-labelledby': `laws-tab-${id}`, tabindex: '0', hidden: true }),
  );
  const stops: (((reason?: string) => void) | undefined)[] = [];
  let current = -1;

  const api = (i: number): Api => ({
    report(r: Report) {
      const name = L(EXPERIMENTS[i]![0]);
      render(
        results[i]!,
        h('p', { class: 'laws-result__name' }, name),
        h(
          'dl',
          { class: 'laws-nums' },
          r.items.map(([label, value, unit]) => h('div', null, h('dt', null, label), h('dd', null, num(value, unit)))),
        ),
        r.plain && h('p', null, r.plain),
        r.note && h('p', { class: 'laws-result__note' }, r.note),
        h('p', { class: 'laws-result__line' }, r.line),
      );
      announce.textContent = `${name}: ${r.items.map(([label, value, unit]) => `${label} ${value} ${unit}`.trim()).join(', ')}.`;
    },
    next: () => select((i + 1) % EXPERIMENTS.length, true),
  });

  function select(i: number, focus = false): void {
    if (i !== current) {
      stops[current]?.(); // leaving a tab abandons its run
      tabs.forEach((tab, j) => {
        tab.setAttribute('aria-selected', String(i === j));
        tab.tabIndex = i === j ? 0 : -1;
        panels[j]!.hidden = i !== j;
      });
      current = i;
      stops[i] ??= EXPERIMENTS[i]![1](panels[i]!, api(i)); // built the first time it's shown
    }
    if (focus) tabs[i]!.focus();
  }

  const tablist = h('div', { class: 'laws-tabs', role: 'tablist', 'aria-label': L('tabs') }, tabs);
  tablist.addEventListener('keydown', (e: KeyboardEvent) => {
    const n = tabs.length;
    const to = ({ ArrowRight: current + 1, ArrowLeft: current + n - 1, Home: 0, End: n - 1 } as Record<string, number>)[e.key];
    if (to === undefined) return;
    e.preventDefault();
    select(to % n, true);
  });

  render(
    body,
    h(
      'div',
      { class: 'laws' },
      tablist,
      panels,
      h(
        'section',
        { class: 'laws-results', 'aria-labelledby': 'laws-results-title' },
        h('h4', { class: 'laws-results__title', id: 'laws-results-title' }, L('results')),
        results,
      ),
      h('p', { class: 'source' }, L('local')),
      announce,
    ),
  );
  select(0);

  // A hidden tab stops the clock in some browsers and not others: abandon the run, say why.
  document.addEventListener('visibilitychange', () => {
    if (document.visibilityState === 'hidden') stops[current]?.(L('hidden'));
  });
}
