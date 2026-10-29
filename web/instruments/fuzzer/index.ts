// S5 · Attack our code. A coverage-guided fuzzer (Rust → WebAssembly, crates/fuzzer) attacks a
// small Rust parser of ours, in this browser, in three chapters:
//   1 Attack  — it learns the format from four meaningless bytes, lights Orion path by path and
//               finds the bug; we show Rust's own panic message, the shrunk input and the fix.
//   2 Fix     — the fixed build takes ten million inputs and doesn't break.
//   3 Compare — the same engine without coverage feedback gets nowhere.
// Plus "try it yourself". It runs on the main thread in ≤ 10 ms slices, only after a click and
// only while on screen. It only ever touches the embedded parser; nothing leaves the browser.

import { assetUrl } from '../../shared/assets.ts';
import { $, h, render } from '../../shared/dom.ts';
import { reducedMotion } from '../../shared/env.ts';
import { lang, t, tp } from '../../shared/i18n.ts';
import { ascii, count, hex, parseInput, rate, seconds, splitPanic } from './format.ts';
import { FuzzRun, Probe, compile, minimize, randomSeed } from './host.ts';
import type { Build, Crash, Discovery, Mode } from './host.ts';
import { P, POINTS, STARS, Sky } from './sky.ts';

type Chapter = 'attack' | 'fix' | 'compare';
type Pace = 'watch' | 'full';
type State = 'ready' | 'running' | 'done';

interface ChapterSpec {
  id: Chapter;
  build: Build;
  mode: Mode;
  pace: Pace;
  ms: number;
  execs?: number;
}

const CHAPTERS: readonly ChapterSpec[] = [
  { id: 'attack', build: 'buggy', mode: 'guided', pace: 'watch', ms: 60_000 },
  { id: 'fix', build: 'fixed', mode: 'guided', pace: 'full', ms: 12_000, execs: 10_000_000 },
  { id: 'compare', build: 'buggy', mode: 'blind', pace: 'full', ms: 4_000 },
];

const NEXT: Record<Chapter, string> = { attack: 'fuzzer-start', fix: 'fuzzer-fix-next', compare: 'fuzzer-blind-next' };

/** Inputs allowed after `ms` of Watch mode: slow enough to follow, then accelerating. */
const watchAllowance = (ms: number) => 4000 * (Math.pow(1.6, ms / 1000) - 1);

const EXAMPLES = {
  valid: 'ORI\\x02\\x01\\x28\\x1E\\x05\\x09N\\x05ORION',
  betelgeuse: 'ORI\\x01\\x01\\x28\\x1E\\x05\\x09',
};

export function mount(root: HTMLElement): void {
  const body = $('.instrument__body', root);
  const status = $('.instrument__status', root);
  if (!body) return;
  if (typeof WebAssembly !== 'object') {
    if (status) status.textContent = t('fuzzer-status-nowasm');
    return;
  }
  if (status) status.textContent = t('fuzzer-status-loading');
  compile(fetch(assetUrl('fuzzer.wasm')))
    .then((module) => {
      new FuzzerIsland(body, module);
      if (status) {
        status.textContent = t('fuzzer-status-ready');
        status.className = 'instrument__status badge badge--info';
      }
    })
    .catch((err: unknown) => {
      console.error('[mintaka] fuzzer', err);
      if (status) status.textContent = t('fuzzer-status-failed');
    });
}

class FuzzerIsland {
  private readonly module: WebAssembly.Module;
  private readonly probe: Probe;
  private readonly locale = lang();

  private readonly el: HTMLElement;
  private readonly sky: Sky;
  private readonly stats: Record<'inputs' | 'rate' | 'paths' | 'crashes' | 'time', HTMLElement>;
  private readonly chapters: HTMLLIElement[];
  private readonly primary: HTMLButtonElement;
  private readonly stopButton: HTMLButtonElement;
  private readonly paceGroup: HTMLElement;
  private readonly paceButtons: Record<Pace, HTMLButtonElement>;
  private readonly paceHint: HTMLElement;
  private readonly logList: HTMLOListElement;
  private readonly result: HTMLElement;
  private readonly announcer: HTMLElement;

  private state: State = 'ready';
  private chapter = 0;
  private run: FuzzRun | null = null;
  private pace: Pace = 'watch';
  private watchOffset = 0;
  private raf = 0;
  private lastFrame = 0;
  private activeMs = 0;
  private batch = 256;
  private visible = true;
  private crashes = 0;
  private guidedCrashAt: number | null = null;
  private samples: Array<[number, number]> = [];
  private lastPaint = 0;
  private lastAnnounce = 0;

  constructor(body: HTMLElement, module: WebAssembly.Module) {
    this.module = module;
    this.probe = new Probe(module);

    const skyPanel = h('div', { class: 'fz__sky' });
    const starNames = Object.fromEntries(Object.keys(STARS).map((name) => [name, t(`fuzzer-star-${name.toLowerCase()}`)]));
    this.sky = new Sky(skyPanel, starNames, !reducedMotion());

    const statList = h('dl', { class: 'fz__stats' });
    const stat = (key: string, initial: string) => {
      const value = h('dd', { class: 'fz__stat-value' }, initial);
      statList.append(h('div', { class: 'fz__stat' }, h('dt', { class: 'fz__stat-label' }, t(key)), value));
      return value;
    };
    this.stats = {
      inputs: stat('fuzzer-stat-inputs', '0'),
      rate: stat('fuzzer-stat-rate', '—'),
      paths: stat('fuzzer-stat-paths', `0 / ${POINTS}`),
      crashes: stat('fuzzer-stat-crashes', '0'),
      time: stat('fuzzer-stat-time', seconds(0, this.locale)),
    };
    skyPanel.append(statList);

    this.chapters = ['fuzzer-chapter-attack', 'fuzzer-chapter-fix', 'fuzzer-chapter-compare'].map((key, i) =>
      h('li', { class: 'fz__chapter' }, h('span', { class: 'fz__chapter-n', 'aria-hidden': 'true' }, String(i + 1)), t(key)),
    );

    this.primary = h('button', { type: 'button', class: 'btn btn--ghost btn--sm fz__go', onclick: () => this.onPrimary() });
    this.stopButton = h('button', { type: 'button', class: 'btn btn--quiet', onclick: () => this.finish(true) }, t('fuzzer-stop'));
    this.paceButtons = {
      watch: h('button', { type: 'button', class: 'chip', onclick: () => this.setPace('watch') }, t('fuzzer-speed-watch')),
      full: h('button', { type: 'button', class: 'chip', onclick: () => this.setPace('full') }, t('fuzzer-speed-full')),
    };
    this.paceGroup = h('div', { class: 'fz__pace', role: 'group', 'aria-label': t('fuzzer-speed') }, this.paceButtons.watch, this.paceButtons.full);
    this.paceHint = h('p', { class: 'fz__hint' }, t('fuzzer-watch-hint'));
    this.logList = h('ol', { class: 'fz__log', 'aria-label': t('fuzzer-log-label') });
    this.result = h('div', { class: 'fz__result' });
    this.announcer = h('p', { class: 'visually-hidden', role: 'status', 'aria-live': 'polite' });

    this.el = h(
      'div',
      { class: 'fz' },
      h(
        'div',
        { class: 'fz__layout' },
        skyPanel,
        h(
          'div',
          { class: 'fz__side' },
          h('ol', { class: 'fz__chapters' }, ...this.chapters),
          h('p', { class: 'fz__lead' }, t('fuzzer-lead')),
          h('div', { class: 'fz__controls' }, this.primary, this.stopButton, this.paceGroup),
          this.paceHint,
          this.logList,
          this.result,
          this.buildTry(),
        ),
      ),
      this.announcer,
    );
    render(body, this.el);

    new IntersectionObserver((entries) => {
      this.visible = entries.some((e) => e.isIntersecting);
    }).observe(this.el);
    // rAF stops in hidden tabs; don't count the gap as fuzzing time.
    document.addEventListener('visibilitychange', () => (this.lastFrame = performance.now()));

    this.renderControls();
  }

  private get spec(): ChapterSpec {
    return CHAPTERS[this.chapter]!;
  }

  // ── flow ────────────────────────────────────────────────────────────────

  private onPrimary(): void {
    if (this.run) return;
    if (this.state === 'done') {
      if (this.chapter === CHAPTERS.length - 1) {
        this.chapter = 0;
        this.guidedCrashAt = null;
      } else {
        this.chapter += 1;
      }
    }
    this.start();
  }

  private start(): void {
    const c = this.spec;
    const seed = randomSeed();
    this.run = new FuzzRun(this.module, { mode: c.mode, build: c.build, seed });
    this.pace = c.pace;
    this.watchOffset = 0;
    this.activeMs = 0;
    this.batch = 256;
    this.crashes = 0;
    this.samples = [];
    this.lastFrame = performance.now();
    this.sky.reset();
    this.sky.mark([]);
    this.result.replaceChildren();
    this.logList.replaceChildren();
    this.log('info', t('fuzzer-log-start', {
      build: t(c.build === 'fixed' ? 'fuzzer-build-fixed' : 'fuzzer-build-buggy'),
      mode: t(c.mode === 'guided' ? 'fuzzer-mode-guided' : 'fuzzer-mode-blind'),
      seed: seed[0].toString(16).padStart(8, '0'),
    }));
    this.state = 'running';
    this.renderControls();
    this.say(t('fuzzer-announce-start'), true);
    this.raf = requestAnimationFrame(this.frame);
  }

  private readonly frame = (now: number): void => {
    const run = this.run;
    if (!run) return;
    const c = this.spec;
    const dt = Math.min(now - this.lastFrame, 100);
    this.lastFrame = now;
    if (this.visible) {
      this.activeMs += dt;
      let allowed = this.pace === 'watch' ? watchAllowance(this.activeMs) + this.watchOffset : Infinity;
      if (c.execs !== undefined) allowed = Math.min(allowed, c.execs);
      const deadline = performance.now() + (this.pace === 'full' ? 10 : 6);
      while (performance.now() < deadline && run.execs < allowed) {
        const t0 = performance.now();
        const { discoveries, crash } = run.step(Math.min(this.batch, allowed - run.execs));
        const took = performance.now() - t0;
        if (took < 1) this.batch = Math.min(this.batch * 2, 1 << 20);
        else if (took > 4) this.batch = Math.max(64, this.batch >> 1);
        for (const d of discoveries) this.onDiscovery(d);
        if (crash) {
          this.onCrash(crash);
          return;
        }
      }
    }
    this.paint();
    if ((c.execs !== undefined && run.execs >= c.execs) || this.activeMs >= c.ms) {
      this.finish();
      return;
    }
    this.raf = requestAnimationFrame(this.frame);
  };

  private onDiscovery(d: Discovery): void {
    this.sky.light(d.points, true);
    this.log('new', d.points.map((p) => t(`fuzzer-point-${p}`)).join(' · '), d.input);
    const now = performance.now();
    if (now - this.lastAnnounce > 2500 && this.run) {
      this.lastAnnounce = now;
      this.say(t('fuzzer-announce-paths', { paths: this.run.covered.length, total: this.run.points }));
    }
  }

  private onCrash(crash: Crash): void {
    this.crashes = 1;
    if (this.run?.mode === 'guided') this.guidedCrashAt = crash.exec;
    // A trapped run never records its last coverage. Replay the crashing input on the fixed
    // build — identical up to the bug — to light the path it took into the name section.
    this.sky.light(this.probe.run(crash.input, 'fixed').points, false);
    this.sky.crash(P.nameLen);
    this.log('crash', splitPanic(crash.message).text, crash.input);
    const min = minimize(crash.input, (b) => this.probe.crashes(b));
    this.result.replaceChildren(this.crashCard(crash, min));
    this.say(t('fuzzer-announce-crash', { inputs: crash.exec }), true);
    this.finish();
  }

  private finish(stopped = false): void {
    cancelAnimationFrame(this.raf);
    const run = this.run;
    if (!run) return;
    this.paint(true);
    const c = this.spec;
    const paths = run.covered.length;
    this.log('info', t('fuzzer-log-done', { inputs: count(run.execs, this.locale), paths: `${paths}/${run.points}` }));
    if (!stopped && !run.crash) {
      if (c.id === 'fix') this.result.replaceChildren(this.fixedCard(run));
      if (c.id === 'compare') this.result.replaceChildren(this.blindCard(run));
    }
    this.say(tp('fuzzer-announce-done', this.crashes, { inputs: run.execs, paths }));
    this.run = null;
    // A stopped or crash-less attack restarts the same chapter.
    this.state = c.id === 'attack' && this.crashes === 0 ? 'ready' : 'done';
    this.renderControls();
  }

  private setPace(pace: Pace): void {
    // Back to Watch after a burst: continue the curve from where the run is, don't stall.
    if (pace === 'watch' && this.run) this.watchOffset = Math.max(0, this.run.execs - watchAllowance(this.activeMs));
    this.pace = pace;
    this.renderControls();
  }

  // ── rendering ───────────────────────────────────────────────────────────

  private renderControls(): void {
    const running = this.state === 'running';
    const next = this.state === 'done' ? CHAPTERS[this.chapter + 1] : this.spec;
    this.primary.textContent = t(next ? NEXT[next.id] : 'fuzzer-again');
    this.primary.hidden = running;
    this.stopButton.hidden = !running;
    for (const pace of ['watch', 'full'] as const) this.paceButtons[pace].setAttribute('aria-pressed', String(this.pace === pace));
    this.paceGroup.hidden = !running;
    this.paceHint.hidden = !(running && this.pace === 'watch');
    this.el.dataset['state'] = this.state;
    this.chapters.forEach((li, i) => {
      li.classList.toggle('is-done', i < this.chapter || (this.state === 'done' && i === this.chapter));
      if (i === this.chapter) li.setAttribute('aria-current', 'step');
      else li.removeAttribute('aria-current');
    });
  }

  private paint(force = false): void {
    const now = performance.now();
    if (!force && now - this.lastPaint < 80) return;
    this.lastPaint = now;
    const run = this.run;
    const execs = run?.execs ?? 0;
    this.samples.push([this.activeMs, execs]);
    while (this.samples.length > 2 && this.activeMs - this.samples[0]![0] > 600) this.samples.shift();
    const [t0, e0] = this.samples[0]!;
    const perSecond = this.activeMs - t0 > 50 ? ((execs - e0) / (this.activeMs - t0)) * 1000 : 0;
    this.stats.inputs.textContent = count(execs, this.locale);
    this.stats.rate.textContent = run ? rate(perSecond, this.locale) : '—';
    this.stats.paths.textContent = `${run?.covered.length ?? 0} / ${run?.points ?? POINTS}`;
    this.stats.crashes.textContent = String(this.crashes);
    this.stats.crashes.parentElement?.classList.toggle('is-alert', this.crashes > 0);
    this.stats.time.textContent = seconds(this.activeMs, this.locale);
  }

  private log(kind: 'info' | 'new' | 'crash', text: string, input?: Uint8Array): void {
    const tag = kind === 'crash' ? t('fuzzer-log-crash') : kind === 'new' ? t('fuzzer-log-new') : '·';
    const item = h(
      'li',
      { class: `fz__log-item fz__log-item--${kind}` },
      h('span', { class: 'fz__log-time' }, seconds(this.activeMs, this.locale)),
      h('span', { class: 'fz__log-tag' }, tag),
      h('span', { class: 'fz__log-text' }, text),
      input ? h('span', { class: 'fz__log-bytes' }, `${hex(input, 8)}  ${ascii(input, 12)}`) : null,
    );
    this.logList.append(item);
    while (this.logList.children.length > 80) this.logList.firstElementChild?.remove();
    this.logList.scrollTop = this.logList.scrollHeight;
  }

  private say(text: string, now = false): void {
    if (now) this.lastAnnounce = performance.now();
    this.announcer.textContent = text;
  }

  private crashCard(crash: Crash, min: Uint8Array): HTMLElement {
    const { location, text } = splitPanic(crash.message);
    const line = Number(location.split(':').at(-2)) || 0;
    return h(
      'section',
      { class: 'fz__card fz__card--crash', 'aria-label': t('fuzzer-log-crash') },
      h('h4', { class: 'fz__card-title' }, t('fuzzer-crash-title', { inputs: crash.exec, time: seconds(this.activeMs, this.locale) })),
      h('p', { class: 'fz__card-label' }, t('fuzzer-crash-panic')),
      h('pre', { class: 'fz__code' }, `panicked at ${location}:\n${text}`),
      h('p', { class: 'fz__card-label' }, t('fuzzer-crash-min', { from: crash.input.length, to: min.length })),
      h('p', { class: 'fz__bytes' }, h('code', null, hex(min)), h('code', { class: 'fz__bytes-ascii' }, ascii(min))),
      h('p', null, t('fuzzer-crash-why', { line: String(line) })),
      h('p', { class: 'fz__card-label' }, t('fuzzer-crash-fix')),
      h(
        'pre',
        { class: 'fz__code fz__code--diff' },
        h('span', { class: 'fz__del' }, `${line} - Build::Buggy => &data[start..start + len],`),
        '\n',
        h('span', { class: 'fz__add' }, `${line + 2} + Build::Fixed => data.get(start..start + len).ok_or(ParseError::Truncated)?,`),
      ),
      h('p', { class: 'fz__note' }, t('fuzzer-crash-safe')),
    );
  }

  private fixedCard(run: FuzzRun): HTMLElement {
    return h(
      'section',
      { class: 'fz__card fz__card--safe' },
      h('h4', { class: 'fz__card-title' }, t('fuzzer-fixed-title', { inputs: run.execs })),
      h('p', null, t('fuzzer-fixed-body', { paths: run.covered.length, total: run.points })),
    );
  }

  private blindCard(run: FuzzRun): HTMLElement {
    const values = {
      paths: run.covered.length,
      total: run.points,
      inputs: run.execs,
      time: seconds(this.activeMs, this.locale),
      guided: this.guidedCrashAt ?? 0,
    };
    return h(
      'section',
      { class: 'fz__card' },
      h('h4', { class: 'fz__card-title' }, t('fuzzer-blind-title', values)),
      h('p', null, t(this.guidedCrashAt ? 'fuzzer-blind-body' : 'fuzzer-blind-body-no-guided', values)),
    );
  }

  // ── try it yourself ─────────────────────────────────────────────────────

  private buildTry(): HTMLElement {
    const input = h('input', { id: 'fz-try-input', class: 'fz__try-input', type: 'text', spellcheck: 'false', autocomplete: 'off', value: EXAMPLES.valid });
    const out = h('p', { class: 'fz__try-out', 'aria-live': 'polite' });
    const radios = (['buggy', 'fixed'] as const).map((b) => {
      const radio = h('input', { type: 'radio', name: 'fz-try-build', value: b, class: 'visually-hidden' });
      radio.checked = b === 'buggy';
      return { b, radio, label: h('label', { class: 'chip' }, radio, t(b === 'buggy' ? 'fuzzer-build-buggy' : 'fuzzer-build-fixed')) };
    });
    const go = () => {
      const bytes = parseInput(input.value);
      if (!bytes) {
        out.dataset['kind'] = 'invalid';
        render(out, t('fuzzer-try-invalid'));
        return;
      }
      const build = radios.find((r) => r.radio.checked)?.b ?? 'buggy';
      const r = this.probe.run(bytes, build);
      // For a crash, show the path the input took up to the bug (the fixed build's coverage).
      this.sky.mark(r.code === 'crash' ? this.probe.run(bytes, 'fixed').points : r.points, r.code === 'crash');
      out.dataset['kind'] = r.code === 'crash' ? 'crash' : r.code === 'ok' ? 'ok' : 'rejected';
      if (r.code === 'crash') {
        const { location, text } = splitPanic(r.message ?? '');
        render(
          out,
          h('strong', null, t('fuzzer-try-crashed')),
          ' ',
          h('a', { href: '#careers' }, t('fuzzer-try-careers')),
          h('code', { class: 'fz__code fz__try-panic' }, `panicked at ${location}:\n${text}`),
        );
      } else if (r.code === 'ok') {
        render(out, t('fuzzer-try-ok', { paths: r.points.length }));
      } else {
        render(out, t('fuzzer-try-rejected', { reason: t(`fuzzer-reason-${r.code}`), paths: r.points.length }));
      }
    };
    const example = (key: string, value: string) =>
      h('button', { type: 'button', class: 'chip', onclick: () => ((input.value = value), go()) }, t(key));
    input.addEventListener('keydown', (e) => {
      if (e.key === 'Enter') go();
    });
    return h(
      'details',
      { class: 'fz__try' },
      h('summary', null, t('fuzzer-try-title')),
      h('p', { class: 'fz__try-lead' }, t('fuzzer-try-lead')),
      h('p', { class: 'fz__try-format' }, t('fuzzer-try-format')),
      h('label', { class: 'fz__try-label', for: 'fz-try-input' }, t('fuzzer-try-label')),
      input,
      h(
        'div',
        { class: 'fz__try-row' },
        h('div', { class: 'fz__try-builds', role: 'radiogroup', 'aria-label': t('fuzzer-try-build') }, ...radios.map((r) => r.label)),
        h('button', { type: 'button', class: 'btn btn--ghost btn--sm', onclick: go }, t('fuzzer-try-run')),
      ),
      h('div', { class: 'fz__try-examples' }, example('fuzzer-try-example-valid', EXAMPLES.valid), example('fuzzer-try-example-betelgeuse', EXAMPLES.betelgeuse)),
      out,
    );
  }
}
