// The console challenge (spec §7.5, docs/CONTRACTS.md §8.3): four puzzles, one per pillar star,
// played from DevTools. The server makes every puzzle and checks every answer; this module carries
// the run token, explains each puzzle and, after the fourth, opens the application form in the
// Team section. Loaded only when someone types a challenge command (or comes back with a pass).

import { $, h, render } from '../shared/dom.ts';
import { lang, loadStrings, t } from '../shared/i18n.ts';
import { reducedMotion } from '../shared/env.ts';
import { track } from '../shared/rum.ts';

export type Puzzle =
  | { star: 'rigel'; grid: string }
  | { star: 'saiph'; a: string; b: string }
  | { star: 'betelgeuse'; tcp_info: string }
  | { star: 'meissa'; trials: [number, number, number][] };

export interface State {
  correct?: boolean;
  run: string;
  /** 1–4, or 5 once every stage is solved. */
  stage: number;
  puzzle: Puzzle | null;
}

export const STAGES = 4;
const RUN = 'mintaka-ctf';
const SENT = 'mintaka-ctf-sent';
const STAR_NAMES = { rigel: 'Rigel', saiph: 'Saiph', betelgeuse: 'Betelgeuse', meissa: 'Meissa' } as const;

/**
 * The stage a run token carries — byte 16 of `nonce[16] ‖ stage ‖ tag[16]` — read without asking
 * the server (which alone can tell whether the token is genuine).
 */
export function stageOf(token: string): number | null {
  if (!token.startsWith('r1.')) return null;
  const b64 = token.slice(3).replace(/-/g, '+').replace(/_/g, '/');
  try {
    const raw = atob(b64 + '='.repeat((4 - (b64.length % 4)) % 4));
    return raw.length === 33 ? raw.charCodeAt(16) : null;
  } catch {
    return null;
  }
}

// The run lives in this browser only, and only once the visitor has started the challenge.
const store = {
  get(key: string): string | null {
    try {
      return localStorage.getItem(key);
    } catch {
      return null;
    }
  },
  set(key: string, value: string | null): void {
    try {
      if (value === null) localStorage.removeItem(key);
      else localStorage.setItem(key, value);
    } catch {
      /* private mode: the run lasts as long as the page */
    }
  },
};

let current: State | null = null;

class Failure extends Error {}

async function call(path: 'start' | 'puzzle' | 'flag', body: Record<string, string>): Promise<State> {
  let res: Response;
  try {
    res = await fetch(`/api/ctf/${path}`, {
      method: 'POST',
      cache: 'no-store',
      headers: { 'content-type': 'application/json', accept: 'application/json' },
      body: JSON.stringify(body),
    });
  } catch {
    throw new Failure('failed');
  }
  if (res.status === 429) throw new Failure('limited');
  const data = (await res.json().catch(() => null)) as (State & { error?: string }) | null;
  if (!res.ok || !data) throw new Failure(data?.error === 'bad-run' ? 'bad-run' : 'failed');
  return data;
}

function keep(state: State): State {
  current = state;
  store.set(RUN, state.run);
  return state;
}

/** The run in progress: from memory, from this browser, or a new one. */
async function ensure(fresh = false): Promise<State> {
  if (current && !fresh) return current;
  const saved = fresh ? null : store.get(RUN);
  if (saved) {
    try {
      return keep(await call('puzzle', { run: saved }));
    } catch (e) {
      // A token from another server key (or a damaged one) starts over; anything else is real.
      if (!(e instanceof Failure && e.message === 'bad-run')) throw e;
    }
  }
  return keep(await call('start', {}));
}

/** This module's text (a lazy bundle); says so in the console if it can't be fetched. */
async function text(): Promise<boolean> {
  if (await loadStrings('ctf')) return true;
  console.log(t('ctf-failed'));
  return false;
}

function say(error: unknown): void {
  const reason = error instanceof Failure && error.message === 'limited' ? 'limited' : 'failed';
  console.log(t(`ctf-${reason}`));
}

function show(state: State): void {
  if (!state.puzzle) {
    console.log(t('ctf-done'));
    openForm(state.run, false);
    return;
  }
  const star = state.puzzle.star;
  console.log(`%c${t('ctf-stage', { n: state.stage, title: t(`ctf-${star}-title`) })}`, 'font-weight: bold');
  console.log(t(`ctf-${star}`));
  console.log(t('ctf-commands'));
}

/** `mintaka.ctf()`: start, or show where the run stands. */
export async function ctf(fresh = false): Promise<void> {
  if (!(await text())) return;
  try {
    const state = await ensure(fresh);
    if (state.stage === 1 && !fresh) console.log(t('ctf-intro'));
    show(state);
  } catch (e) {
    say(e);
  }
}

/** `mintaka.restart()`: forget this run and start a new one. */
export function restart(): Promise<void> {
  current = null;
  store.set(RUN, null);
  store.set(SENT, null);
  return ctf(true);
}

/** `mintaka.puzzle()`: this stage's data, for the visitor's own code. */
export async function puzzle(): Promise<Puzzle | null> {
  try {
    return (await ensure()).puzzle;
  } catch (e) {
    say(e);
    return null;
  }
}

/** `mintaka.flag(answer)`: check an answer; a right one shows the next stage. */
export async function flag(answer: unknown): Promise<boolean> {
  if (!(await text())) return false;
  try {
    const before = await ensure();
    const after = keep(await call('flag', { run: before.run, flag: String(answer) }));
    if (!after.correct) {
      console.log(t('ctf-wrong'));
      return false;
    }
    track('ctf_stage', before.stage as 1 | 2 | 3 | 4);
    if (before.puzzle) console.log(t('ctf-right', { star: STAR_NAMES[before.puzzle.star] }));
    show(after);
    return true;
  } catch (e) {
    say(e);
    return false;
  }
}

/** `mintaka.hint()`. */
export async function hint(): Promise<void> {
  if (!(await text())) return;
  try {
    const state = await ensure();
    console.log(state.puzzle ? t(`ctf-${state.puzzle.star}-hint`) : t('ctf-done'));
  } catch (e) {
    say(e);
  }
}

/** `mintaka.apply()`: the application form, once every stage is solved. */
export async function apply(): Promise<void> {
  if (!(await text())) return;
  const saved = current?.run ?? store.get(RUN);
  if (!saved || (stageOf(saved) ?? 0) <= STAGES) {
    console.log(t('ctf-not-done'));
    return;
  }
  openForm(saved, true);
}

/** On page load: a visitor who finished earlier finds the form waiting (until it has been sent). */
export async function resume(): Promise<void> {
  const saved = store.get(RUN);
  if (saved && (stageOf(saved) ?? 0) > STAGES && !store.get(SENT) && (await loadStrings('ctf'))) openForm(saved, false);
}

// ---- the application form, in #careers ----

/** Field ids the contact API reports errors against, and where they show on this form. */
const FIELDS = ['name', 'email', 'telegram', 'contact', 'message'] as const;

function openForm(pass: string, focus: boolean): void {
  const careers = $('#careers');
  if (!careers) return;
  const form = $<HTMLFormElement>('#careers-form', careers) ?? careers.appendChild(buildForm(pass));
  if (!focus) return;
  form.scrollIntoView({ behavior: reducedMotion() ? 'auto' : 'smooth', block: 'start' });
  $<HTMLInputElement>('#a-name', form)?.focus({ preventScroll: true });
}

function buildForm(pass: string): HTMLFormElement {
  const error = (name: string) => h('p', { class: 'field__error', id: `ae-${name}`, hidden: true });
  const field = (name: string, label: string, input: HTMLElement, hint?: string) =>
    h(
      'div',
      { class: 'field' },
      h('label', { class: 'field__label', for: `a-${name}` }, label),
      hint ? h('p', { class: 'field__hint', id: `ah-${name}` }, hint) : null,
      input,
      error(name),
    );
  const input = (name: string, attrs: Record<string, string>) =>
    h('input', { class: 'input', id: `a-${name}`, name, 'aria-describedby': `ae-${name}`, ...attrs });

  const status = h('p', { class: 'careers-form__status', role: 'status', 'aria-live': 'polite' });
  const send = h('button', { class: 'btn btn--ghost', type: 'submit' }, t('ctf-apply-send'));
  const form = h(
    'form',
    { class: 'careers-form', id: 'careers-form', 'aria-labelledby': 'careers-form-title', novalidate: true },
    h('h3', { class: 'careers-form__title', id: 'careers-form-title' }, t('ctf-apply-title')),
    h('p', { class: 'careers-form__lead' }, t('ctf-apply-lead')),
    field('name', t('ctf-apply-name'), input('name', { required: '', maxlength: '120', autocomplete: 'name' })),
    h('p', { class: 'field__hint', id: 'ah-contact' }, t('ctf-apply-rule')),
    field('email', t('ctf-apply-email'), input('email', { type: 'email', maxlength: '254', autocomplete: 'email' })),
    field('telegram', t('ctf-apply-telegram'), input('telegram', { maxlength: '33', autocomplete: 'off', spellcheck: 'false', placeholder: '@' })),
    error('contact'),
    field(
      'message',
      t('ctf-apply-work'),
      h('textarea', { class: 'input', id: 'a-message', name: 'message', rows: '5', maxlength: '5000', 'aria-describedby': 'ah-message ae-message' }),
      t('ctf-apply-work-hint'),
    ),
    status,
    send,
  );
  for (const name of ['email', 'telegram']) $(`#a-${name}`, form)?.setAttribute('aria-describedby', `ah-contact ae-${name} ae-contact`);

  const showErrors = (errors: { field: string; code: string }[]) => {
    for (const name of FIELDS) {
      const p = $(`#ae-${name}`, form);
      const problem = errors.find((e) => e.field === name);
      if (p) {
        p.hidden = !problem;
        p.textContent = problem ? message(problem.code) : '';
      }
      // "Email or Telegram" belongs to both inputs.
      for (const id of name === 'contact' ? ['email', 'telegram'] : [name]) {
        if (problem || !errors.some((e) => e.field === id)) $(`#a-${id}`, form)?.toggleAttribute('aria-invalid', !!problem);
      }
    }
    const general = errors.find((e) => !(FIELDS as readonly string[]).includes(e.field));
    status.textContent = general ? message(general.code) : '';
    const first = errors[0];
    if (first) $<HTMLElement>(`#a-${first.field === 'contact' ? 'email' : first.field}`, form)?.focus();
  };

  let busy = false;
  form.addEventListener('submit', async (e) => {
    e.preventDefault();
    if (busy) return;
    const data = new FormData(form);
    const value = (k: string) => String(data.get(k) ?? '').trim();
    const local: { field: string; code: string }[] = [];
    if (!value('name')) local.push({ field: 'name', code: 'name-required' });
    if (!value('email') && !value('telegram')) local.push({ field: 'contact', code: 'contact-required' });
    if (local.length) return showErrors(local);

    busy = true;
    send.setAttribute('aria-disabled', 'true');
    status.textContent = t('ctf-apply-sending');
    // No proof of work here: applications ride the contact API's stricter limit for briefs
    // without one (3 an hour per network), which is plenty for people applying.
    const body: Record<string, unknown> = { lang: lang(), kind: [], name: value('name'), website: '', ctf_pass: pass };
    for (const k of ['email', 'telegram', 'message']) if (value(k)) body[k] = value(k);
    try {
      const res = await fetch('/api/contact', {
        method: 'POST',
        headers: { 'content-type': 'application/json', accept: 'application/json' },
        body: JSON.stringify(body),
      }).catch(() => null);
      if (res?.ok) {
        store.set(SENT, '1');
        const done = h('p', { class: 'careers-form__done', tabindex: '-1' }, t('ctf-apply-sent'));
        render(form, done);
        done.focus();
        return;
      }
      const errors = ((await res?.json().catch(() => null)) as { errors?: { field: string; code: string }[] } | null)?.errors;
      showErrors(errors?.length ? errors : [{ field: 'form', code: !res ? 'network' : res.status === 429 ? 'rate-limited' : 'server-error' }]);
    } finally {
      busy = false;
      send.removeAttribute('aria-disabled');
      if (status.textContent === t('ctf-apply-sending')) status.textContent = '';
    }
  });
  return form;
}

/** The contact form's own wording for each error code (contact.ftl). */
function message(code: string): string {
  const key = `contact-error-${code}`;
  const text = t(key);
  return text === key ? t('contact-error-server-error') : text;
}
