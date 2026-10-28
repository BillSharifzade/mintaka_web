// The `~` terminal: a real modal <dialog> with a prompt and a polite live log. Every answer comes
// from this page view: the browser's own measurements, /_proof and /_ping. Loaded on first use.

import { $, $$, h, render } from '../shared/dom.ts';
import { lang, t, tp } from '../shared/i18n.ts';
import { emit, lastValue, on } from '../shared/bus.ts';
import { getProof } from '../shared/proof.ts';
import { connectionReused, navEntry, ping, serverTiming } from '../shared/timing.ts';
import { duration, join } from '../shared/format.ts';
import { reducedMotion } from '../shared/env.ts';
import { browser, complete, flatten, hops, parse, platform, stats, table } from './lib.ts';

const T = (key: string, vars?: Record<string, string | number>): string => t('terminal-' + key, vars);
const TP = (key: string, n: number, vars?: Record<string, string | number>): string => tp('terminal-' + key, n, vars);
const PINGS = 10;
const PS1 = 'mintaka:~$';
const L = lang();
const fmt = (ms: number | undefined) => (ms === undefined ? '—' : join(duration(ms, L)).replace(/^0 µs$/, '0 ms'));
const reason = (err: unknown) => (err instanceof Error ? err.message : String(err));

let dialog: HTMLDialogElement | undefined;
let log: HTMLElement;
let input: HTMLInputElement;
let opener: HTMLElement | null = null;
let restore = true;
const history: string[] = [];
let cursor = 0; // index into history; history.length means the line being typed
let draft = '';
/** Where a command prints: its own block, reserved under its prompt line when it was entered. */
type Out = (text: string) => void;

const COMMANDS: Record<string, (arg: string, out: Out) => unknown> = {
  help: (_, out) =>
    out(table(Object.keys(COMMANDS).map((n) => [n === 'lang' ? 'lang <code>' : n, n === 'ping' ? TP('help-ping', PINGS) : T('help-' + n)]))),
  whoami,
  pillars: (_, out) => out(table(['rigel', 'saiph', 'betelgeuse', 'meissa'].map((star) => [T('star-' + star), T('pillar-' + star)]))),
  ping: pings,
  trace,
  proof: async (_, out) => out(table(flatten(await getProof({ fresh: true }))) + '\n' + T('proof-src')),
  lang: language,
  sky,
  ctf: (_, out) => out(T('ctf')),
  contact,
  clear: () => render(log),
  exit: () => dialog!.close(),
};

/** Open the terminal (idempotent); focus returns to whatever opened it. */
export function open(): void {
  dialog ??= build();
  if (!dialog.open) {
    opener = document.activeElement instanceof HTMLElement ? document.activeElement : null;
    restore = true;
    dialog.showModal();
  }
  input.focus();
}

function build(): HTMLDialogElement {
  log = h('div', { class: 'term-log', role: 'log', 'aria-live': 'polite', 'aria-label': T('log'), tabindex: '0' });
  input = h('input', {
    class: 'term-in',
    id: 'term-in',
    autocomplete: 'off',
    autocapitalize: 'off',
    autocorrect: 'off',
    spellcheck: 'false',
    enterkeyhint: 'send',
    'aria-describedby': 'term-keys',
  });
  input.addEventListener('keydown', keys);
  const d = h(
    'dialog',
    { class: 'sheet term', 'aria-labelledby': 'term-title' },
    h(
      'div',
      { class: 'term-bar' },
      h('h2', { class: 'term-title', id: 'term-title' }, T('title')),
      h('p', { class: 'term-keys', id: 'term-keys' }, T('keys')),
      h('button', { type: 'button', class: 'btn btn--quiet btn--sm term-close', onclick: () => d.close() }, T('close')),
    ),
    log,
    h(
      'form',
      { class: 'term-line', onsubmit: (e: SubmitEvent) => (e.preventDefault(), submit()) },
      h('label', { class: 'visually-hidden', for: 'term-in' }, T('input')),
      h('span', { class: 'term-ps1', 'aria-hidden': 'true' }, PS1),
      input,
    ),
  );
  d.addEventListener('close', () => restore && opener?.focus());
  document.body.append(d);
  print(T('welcome'));
  return d;
}

/** Print into `into` (the log itself for the welcome line and Tab suggestions). */
function print(text: string, into: HTMLElement = log): void {
  into.append(h('pre', { class: 'term-out' }, text));
  log.scrollTop = log.scrollHeight;
}

function keys(e: KeyboardEvent): void {
  if ((e.key === 'ArrowUp' || e.key === 'ArrowDown') && history.length) {
    e.preventDefault();
    if (cursor === history.length) draft = input.value;
    cursor = Math.max(0, Math.min(history.length, cursor + (e.key === 'ArrowUp' ? -1 : 1)));
    input.value = cursor === history.length ? draft : history[cursor]!;
  } else if (e.key === 'Tab' && !e.shiftKey && input.value.trim()) {
    // Only with something typed, so Tab still moves focus out of an empty prompt.
    e.preventDefault();
    const words = !input.value.includes(' ') ? Object.keys(COMMANDS) : parse(input.value)[0] === 'lang' ? langs() : [];
    const done = complete(input.value, words);
    if (done.line !== input.value) input.value = done.line;
    else if (done.options.length > 1) print(done.options.join('  '));
  } else if (e.key === 'l' && e.ctrlKey) {
    e.preventDefault();
    render(log);
  }
}

function submit(): void {
  const line = input.value;
  input.value = draft = '';
  if (line.trim() && history.at(-1) !== line) history.push(line);
  cursor = history.length;
  // The command's output goes in a block reserved right under its prompt line, so a slow command
  // (whoami, ping, proof) still prints above the commands typed after it.
  const box = h('div', { class: 'term-cmd' });
  log.append(h('pre', { class: 'term-echo' }, h('span', { class: 'term-ps1', 'aria-hidden': 'true' }, PS1 + ' '), line), box);
  log.scrollTop = log.scrollHeight;
  const [cmd, arg] = parse(line);
  if (!cmd) return;
  const run = Object.hasOwn(COMMANDS, cmd) ? COMMANDS[cmd] : undefined;
  const out: Out = (text) => print(text, box);
  Promise.resolve()
    .then(() => (run ? run(arg, out) : out(T('unknown', { cmd }))))
    .catch((err: unknown) => out(T('error', { reason: reason(err) })));
}

async function whoami(_: string, out: Out): Promise<void> {
  const hints = (navigator as Navigator & { userAgentData?: { brands: { brand: string; version: string }[]; platform: string } }).userAgentData;
  const ua = navigator.userAgent;
  const tier = lastValue('tier');
  let gpu = T('no');
  if ('gpu' in navigator) {
    gpu = T('gpu-no-adapter');
    const adapter = await Promise.race([navigator.gpu.requestAdapter().catch(() => null), new Promise<null>((r) => setTimeout(r, 2000, null))]);
    if (adapter) gpu = T('yes');
  }
  out(
    table([
      [T('who-browser'), [browser(hints?.brands, ua) || T('unknown-browser'), platform(hints?.platform, ua)].filter(Boolean).join(' · ')],
      [T('who-langs'), navigator.languages.join(', ')],
      [T('who-tier'), tier ? t('hud-tier-' + tier.toLowerCase()) : T('who-tier-none')],
      [T('who-motion'), T(reducedMotion() ? 'motion-reduced' : 'motion-full')],
      [T('who-cores'), String(navigator.hardwareConcurrency || '?')],
      [T('who-isolated'), T(globalThis.crossOriginIsolated ? 'yes' : 'no')],
      [T('who-gpu'), gpu],
    ]) + '\n' + T('who-local'),
  );
}

async function pings(_: string, out: Out): Promise<void> {
  out(TP('ping-start', PINGS));
  const total: number[] = [];
  const app: number[] = [];
  let clock = false;
  let failure: unknown;
  for (let i = 0; i < PINGS; i++) {
    try {
      const p = await ping();
      total.push(p.total);
      if (p.app !== undefined) app.push(p.app);
      clock ||= !p.timed;
    } catch (err) {
      failure = err;
    }
  }
  if (!total.length) return out(T('error', { reason: reason(failure) }));
  const s = stats(total);
  out(
    [
      TP('ping-got', total.length, { total: PINGS }),
      T('ping-stats', { min: fmt(s.min), median: fmt(s.median), p90: fmt(s.p90), max: fmt(s.max) }),
      app.length ? T('ping-server', { ms: fmt(stats(app).median) }) : '',
      T(clock ? 'ping-src-clock' : 'ping-src'),
    ]
      .filter(Boolean)
      .join('\n'),
  );
}

function trace(_: string, out: Out): void {
  const nav = navEntry();
  if (!nav) return out(T('trace-none'));
  const rows = hops(nav, (name) => serverTiming(name, nav), location.protocol === 'https:', connectionReused(nav));
  out(T('trace-head') + '\n' + table(rows.map(([key, ms, note]) => [(key.startsWith('st-') ? '  ' : '') + T('trace-' + key), fmt(ms) + (note ? '  ' + T('note-' + note) : '')]), '  '));
}

const langs = (): string[] =>
  $$<HTMLLinkElement>('link[rel="alternate"][hreflang]')
    .map((l) => l.hreflang.toLowerCase())
    .filter((code) => code && code !== 'x-default');

function language(arg: string, out: Out): void {
  const code = arg.toLowerCase();
  const all = langs();
  if (code === L) return out(T('lang-same', { code }));
  if (code && all.includes(code)) {
    out(T('lang-go', { code }));
    return location.assign(`/${code}/`);
  }
  out(T(code ? 'lang-unknown' : 'lang-list', { code, list: all.join(', ') || L }));
}

async function sky(_: string, out: Out): Promise<void> {
  let heard = false;
  const off = on('sky', (running) => {
    heard = true;
    out(T(running ? 'sky-on' : 'sky-off'));
  });
  emit('sky:toggle', undefined);
  await new Promise((r) => setTimeout(r, 300));
  off();
  if (!heard) out(T('sky-none'));
}

function contact(_: string, out: Out): void {
  const section = $('#contact');
  if (!section) return out(T('contact-none'));
  restore = false;
  dialog!.close();
  section.scrollIntoView({ behavior: reducedMotion() ? 'auto' : 'smooth', block: 'start' });
  $('input:not([type="hidden"]):not([tabindex="-1"]), textarea, select', section)?.focus({ preventScroll: true });
}
