// @ts-ignore -- Node built-in; this repo carries no @types/node.
import { test } from 'node:test';
// @ts-ignore -- Node built-in.
import assert from 'node:assert/strict';
import { EVENT_KEYS, beacon, inp, route, worthSending, type Test } from './vitals.ts';
import { take, track } from '../shared/rum.ts';

test('INP: the slowest interaction, one outlier ignored per 50', () => {
  assert.equal(inp([]), undefined, 'no interactions, no INP');
  assert.equal(inp([40, 120, 64]), 120);
  // 120 interactions, the 10 slowest kept: the third slowest counts.
  const worst = [900, 400, 300, 200, 180, 170, 160, 150, 140, 130];
  assert.equal(inp(worst, 120), 300);
  assert.equal(inp(worst, 49), 900);
  assert.equal(inp(worst, 50), 400);
  assert.equal(inp([80, 60], 5000), 60, 'never past the shortest kept');
});

test('route: the canonical path, or 404 — never the address bar', () => {
  assert.equal(route('https://mintaka.tj/en/', '/'), '/en/');
  assert.equal(route('https://mintaka.tj/en/privacy/', '/en/privacy/?ref=x'), '/en/privacy/');
  assert.equal(route('https://mintaka.tj/en/404', '/en/whatever/they/typed'), '404');
  assert.equal(route(undefined, '/en/'), '/en/');
  assert.equal(route('not a url', '/en/'), '/en/');
});

test('beacon: rounded, ranged, and metrics only on the first', () => {
  const b = beacon('/en/', true, { ttfb: 92.44, fcp: 640.25, lcp: 812.66, cls: 0.000123, inp: 48 }, { proof_open: 2 });
  assert.deepEqual(b, { route: '/en/', view: true, metrics: { ttfb: 92.4, fcp: 640.3, lcp: 812.7, cls: 0.0001, inp: 48 }, events: { proof_open: 2 } });
  assert.deepEqual(beacon('/en/', true, { lcp: undefined, cls: 0, inp: Number.NaN, fcp: 70_000, ttfb: -3 }, {}), {
    route: '/en/',
    view: true,
    metrics: { cls: 0, ttfb: 0 },
  });
  assert.deepEqual(beacon('/en/', false, { lcp: 800 }, { lang_switch: 1 }), { route: '/en/', events: { lang_switch: 1 } });
});

test('beacon: unknown events are dropped, counts kept to 1–20', () => {
  const b = beacon('404', true, {}, { proof_open: 25, nope: 1, 'ctf_stage:5': 1, 'render_tier:B': 1, terminal_open: 0 });
  assert.deepEqual(b.events, { proof_open: 20, 'render_tier:B': 1 });
  assert.ok(!worthSending(beacon('/en/', false, {}, {})), 'a later beacon with nothing new is not sent');
  assert.ok(worthSending(beacon('/en/', true, {}, {})), 'a page view always is');
});

test('track(): safe before anything loads, counted per key, capped', () => {
  take();
  track('proof_open');
  track('proof_open');
  track('ctf_stage', 2);
  track('instrument_start', 'security');
  for (let i = 0; i < 30; i++) track('terminal_open');
  assert.deepEqual(take(), { proof_open: 2, 'ctf_stage:2': 1, 'instrument_start:security': 1, terminal_open: 20 });
  assert.deepEqual(take(), {}, 'taken once');
});

test('every key track() can produce is one the server knows', () => {
  const keys = [
    'cta_start_project', 'proof_open', 'terminal_open', 'contact_submit', 'lang_switch',
    ...[1, 2, 3, 4].map((n) => `ctf_stage:${n}`),
    ...['physics', 'efficiency', 'security', 'highload', 'design'].flatMap((p) => [`instrument_start:${p}`, `instrument_complete:${p}`]),
    ...['A', 'B', 'C', 'D'].map((t) => `render_tier:${t}`),
  ];
  assert.deepEqual([...keys].sort(), [...EVENT_KEYS].sort());
  assert.equal(EVENT_KEYS.length, 23, 'crates/proof/src/rum.rs EVENTS has 23');
});

test('A/B test: the variant on every beacon, each metric converting once', () => {
  const t: Test = { variant: 'hero-title-hardware', metrics: ['cta_start_project', 'contact_submit'] };
  // The page view's first beacon exposes it; a click on "Start a project" before that converts it.
  assert.deepEqual(beacon('/en/', true, {}, { cta_start_project: 2, 'render_tier:B': 1 }, t), {
    route: '/en/',
    view: true,
    events: { cta_start_project: 2, 'render_tier:B': 1 },
    variant: 'hero-title-hardware',
    converted: ['cta_start_project'],
  });
  // Later: another click is an event, not a second conversion…
  assert.deepEqual(beacon('/en/', false, {}, { cta_start_project: 1 }, t), { route: '/en/', events: { cta_start_project: 1 }, variant: 'hero-title-hardware' });
  // …and sending the form is the secondary metric's first time.
  assert.deepEqual(beacon('/en/', false, {}, { contact_submit: 1, cta_start_project: 1 }, t), {
    route: '/en/',
    events: { contact_submit: 1, cta_start_project: 1 },
    variant: 'hero-title-hardware',
    converted: ['contact_submit'],
  });
  assert.deepEqual(t.metrics, [], 'both reached');
  assert.equal(beacon('/en/', false, {}, { contact_submit: 1 }, t).converted, undefined, 'once per page view');
});

test('A/B test: an exposure without a conversion, and conversions only of known events', () => {
  const t: Test = { variant: 'hero-title', metrics: ['cta_start_project'] };
  assert.deepEqual(beacon('/en/', true, { lcp: 800 }, {}, t), { route: '/en/', view: true, metrics: { lcp: 800 }, variant: 'hero-title' });
  // An event the server wouldn't know is dropped, so it can't convert either.
  const odd: Test = { variant: 'hero-title', metrics: ['nope'] };
  assert.deepEqual(beacon('/en/', true, {}, { nope: 1 }, odd), { route: '/en/', view: true, variant: 'hero-title' });
  assert.ok(!worthSending(beacon('/en/', false, {}, {}, t)), 'nothing new: not sent, test or not');
});
