// S13 · Start a project. Progressive enhancement of the server-rendered form (docs/CONTRACTS.md
// §9.1) — enhance, never replace. Without JavaScript the form posts itself and the server
// redirects. With it: the proof of work (§8.1) is solved in a worker while the visitor types, the
// fields are checked by the server's own rules (§8.2) with specific messages, the brief goes as
// JSON, and the answer appears in place. `validate` is pure and tested in contact.test.ts.

import { assetUrl } from '../shared/assets.ts';
import { emit } from '../shared/bus.ts';
import { $, h, render } from '../shared/dom.ts';
import { duration } from '../shared/format.ts';
import { lang, t, tp } from '../shared/i18n.ts';
import { worker } from '../shared/tt.ts';

export interface Brief {
  kind: string[];
  message: string;
  timeline: string;
  budget: string;
  name: string;
  email: string;
  telegram: string;
  company: string;
}

export interface Problem {
  field: string;
  code: string;
}

// The server's newtypes, mirrored rule for rule (crates/server/src/contact/fields.rs): the client
// must never refuse what the server would accept. Rust's `trim`, `is_whitespace` and
// `is_control` are Unicode White_Space and Cc; lengths are Unicode scalar values.
const KINDS = ['new-product', 'performance', 'security-audit', 'design', 'ai', 'not-sure'];
const TIMELINES = ['asap', '1-3-months', 'later', 'exploring'];
const BUDGETS = ['undecided', 'lt-10k', '10-50k', '50-150k', 'gt-150k'];
/** `Email`: the WHATWG type=email grammar plus a dot in the domain, ≤ 254 characters. */
const EMAIL_LOCAL = /^[A-Za-z0-9.!#$%&'*+/=?^_`{|}~-]{1,64}$/;
const EMAIL_LABEL = /^[A-Za-z0-9](?:[A-Za-z0-9-]{0,61}[A-Za-z0-9])?$/;
const TELEGRAM = /^@?[A-Za-z][A-Za-z0-9_]{4,31}$/;
const trim = (text: string): string => text.replace(/^\p{White_Space}+|\p{White_Space}+$/gu, '');
const chars = (text: string): number => [...text].length;
/** Runs of whitespace and control characters become one space, trimmed (`Name`, `Company`). */
const oneLine = (text: string): string => text.split(/[\p{White_Space}\p{Cc}]+/u).filter(Boolean).join(' ');

export function emailOk(email: string): boolean {
  const at = email.lastIndexOf('@');
  const labels = email.slice(at + 1).split('.');
  return (
    at > 0 && email.length <= 254 && EMAIL_LOCAL.test(email.slice(0, at)) && labels.length >= 2 && labels.every((l) => EMAIL_LABEL.test(l))
  );
}

/** `Message`: CR/CRLF become LF, then trim, then control characters other than LF and tab go. */
export const messageChars = (raw: string): number =>
  [...trim(raw.replace(/\r\n?/g, '\n'))].filter((c) => c === '\n' || c === '\t' || !/\p{Cc}/u.test(c)).length;

/** The server's rules (§8.2), with its errors in its order (the no-JS flow reports the first). */
export function validate(b: Brief): Problem[] {
  const out: Problem[] = [];
  const add = (field: string, code: string) => out.push({ field, code });
  const name = oneLine(b.name);
  const email = trim(b.email);
  const telegram = trim(b.telegram);
  const timeline = trim(b.timeline);
  const budget = trim(b.budget);
  if (b.kind.some((k) => !KINDS.includes(trim(k)))) add('kind', 'choice-invalid');
  if (trim(b.message) && messageChars(b.message) > 5000) add('message', 'message-too-long');
  if (timeline && !TIMELINES.includes(timeline)) add('timeline', 'choice-invalid');
  if (budget && !BUDGETS.includes(budget)) add('budget', 'choice-invalid');
  if (!name) add('name', 'name-required');
  else if (chars(name) > 120) add('name', 'name-too-long');
  if (email && !emailOk(email)) add('email', 'email-invalid');
  if (telegram && !TELEGRAM.test(telegram)) add('telegram', 'telegram-invalid');
  if (!email && !telegram) add('contact', 'contact-required');
  if (chars(oneLine(b.company)) > 120) add('company', 'company-too-long');
  return out;
}

/** The `e-*` element an error code belongs to, by its prefix (`email-invalid` → `email`). */
export const errorKey = (code: string): string => code.slice(0, code.indexOf('-'));

/** Codes with their own message under a field; every other code goes to the status line. */
const FIELD_KEYS = ['message', 'name', 'email', 'telegram', 'company', 'contact'];

interface Solved {
  challenge: string;
  counter: number;
  ms: number;
  tried: number;
}

type FromWorker =
  | { type: 'progress'; tried: number }
  | { type: 'done'; counter: number; tried: number; ms: number }
  | { type: 'error'; message: string };

type Answer = { ok: true } | { ok: false; errors: Problem[] };

const num = (value: string, unit: string): HTMLElement =>
  h('span', { class: 'num' }, value, unit && h('span', { class: 'num__unit' }, unit));

/** tp() whose placeholders become nodes, so measured values keep their .num markup. */
function tpn(key: string, n: number, parts: Record<string, Node>): (Node | string)[] {
  const marks: Record<string, string> = {};
  for (const k of Object.keys(parts)) marks[k] = `\u0000${k}\u0000`;
  return tp(key, n, marks)
    .split('\u0000')
    .map((piece, i) => (i % 2 ? (parts[piece] ?? piece) : piece));
}

/** A field's name for the error summary: its label without the "(optional)" note. */
function fieldName(form: HTMLFormElement, key: string): string {
  const label = form.querySelector(`label[for="f-${key}"]`);
  if (!label) return key;
  const text = Array.from(label.childNodes, (n) => (n instanceof Element && n.classList.contains('field__optional') ? '' : n.textContent));
  return text.join('').trim();
}

export function mount(root: HTMLElement): void {
  if (!(root instanceof HTMLFormElement)) return;
  const form = root;
  const status = $('#contact-status', form);
  const submit = $<HTMLButtonElement>('button[type="submit"], button:not([type])', form);
  const label = submit ? Array.from(submit.childNodes) : [];
  // Quiet on purpose: not a live region, never focused — it is there for whoever reads the form.
  const powLine = h('p', { class: 'contact-pow source', hidden: true });
  submit?.after(powLine);

  const byId = (id: string) => $(`#${id}`, form);
  const say = (text: string) => {
    if (status) status.textContent = text;
  };
  const message = (code: string) => {
    const key = `contact-error-${code}`;
    const text = t(key);
    return text === key ? t('contact-error-server-error') : text;
  };

  // "Add an email or a Telegram handle" belongs to both inputs.
  for (const input of [byId('f-email'), byId('f-telegram')]) {
    const ids = (input?.getAttribute('aria-describedby') ?? '').split(/\s+/).filter(Boolean);
    if (input && byId('e-contact') && !ids.includes('e-contact')) input.setAttribute('aria-describedby', [...ids, 'e-contact'].join(' '));
  }

  // ---- proof of work: fetched on the first focus, solved off the main thread ----
  let job: Promise<Solved | null> | null = null;
  let solver: Worker | null = null;

  const solve = async (): Promise<Solved | null> => {
    const r = await fetch('/api/pow', { cache: 'no-store', headers: { accept: 'application/json' } });
    if (!r.ok) return null;
    const c = (await r.json()) as { challenge: string; difficulty: number };
    return new Promise<Solved | null>((resolve) => {
      const w = (solver = worker(assetUrl('pow.worker'), 'pow'));
      const end = (v: Solved | null) => {
        w.terminate();
        if (solver === w) solver = null;
        resolve(v);
      };
      w.onmessage = ({ data }: MessageEvent<FromWorker>) => {
        if (data.type === 'done') end({ challenge: c.challenge, counter: data.counter, ms: data.ms, tried: data.tried });
        else if (data.type === 'error') end(null);
      };
      w.onerror = () => end(null);
      w.postMessage({ challenge: c.challenge, difficulty: c.difficulty });
    });
  };

  const startPow = (): Promise<Solved | null> => {
    if (job) return job;
    const mine = (job = solve().catch(() => null));
    void mine.then((r) => {
      if (!r || job !== mine) return;
      const f = duration(r.ms);
      const n = num(new Intl.NumberFormat(lang()).format(r.tried), '');
      render(powLine, ...tpn('contact-pow', r.tried, { ms: num(f.value, f.unit), n }));
      powLine.hidden = false;
    });
    return mine;
  };

  /** The solved proof, waiting for it if needed — but never more than 10 s, and never twice. */
  const proof = async (): Promise<Solved | null> => {
    const slow = setTimeout(() => say(t('contact-waiting')), 300);
    let late = 0;
    const result = await Promise.race([
      startPow(),
      new Promise<null>((resolve) => (late = window.setTimeout(resolve, 10_000, null))),
    ]);
    clearTimeout(slow);
    clearTimeout(late);
    job = null; // a challenge is single use: the next attempt gets a fresh one
    if (!result) solver?.terminate(); // failed or too slow: send without it (stricter rate limit)
    return result;
  };

  // ---- reading, checking and showing ----
  const read = (): { data: FormData; brief: Brief } => {
    const data = new FormData(form);
    const get = (k: string) => {
      const v = data.get(k);
      return typeof v === 'string' ? v : '';
    };
    const brief: Brief = {
      kind: data.getAll('kind').filter((v): v is string => typeof v === 'string'),
      message: get('message'),
      timeline: get('timeline'),
      budget: get('budget'),
      name: get('name'),
      email: get('email'),
      telegram: get('telegram'),
      company: get('company'),
    };
    return { data, brief };
  };

  // aria-invalid follows the messages on screen; email and Telegram also answer for #e-contact.
  const sync = () => {
    const shown = (key: string) => byId(`e-${key}`)?.hidden === false;
    for (const key of FIELD_KEYS) {
      const input = byId(`f-${key}`);
      if (!input) continue;
      const bad = shown(key) || ((key === 'email' || key === 'telegram') && shown('contact'));
      if (bad) input.setAttribute('aria-invalid', 'true');
      else input.removeAttribute('aria-invalid');
    }
  };

  const hide = (key: string) => {
    const box = byId(`e-${key}`);
    if (box) {
      box.hidden = true;
      box.textContent = '';
    }
  };

  const show = (problems: Problem[]) => {
    FIELD_KEYS.forEach(hide);
    const names: string[] = [];
    const lines: string[] = [];
    for (const p of problems) {
      const key = errorKey(p.code);
      const box = FIELD_KEYS.includes(key) ? byId(`e-${key}`) : null;
      const text = message(p.code);
      if (!box) {
        if (!lines.includes(text)) lines.push(text);
        continue;
      }
      box.textContent = text;
      box.hidden = false;
      names.push(key === 'contact' ? t('contact-contact') : fieldName(form, key));
    }
    sync();
    const summary = names.length ? t('contact-invalid', { fields: new Intl.ListFormat(lang(), { type: 'conjunction' }).format(names) }) : '';
    say([summary, ...lines].filter(Boolean).join(' '));
    $('[aria-invalid="true"]', form)?.focus();
  };

  // Once a message is on screen, fixing that field clears it (new messages wait for the next send).
  form.addEventListener('input', (e) => {
    const id = e.target instanceof HTMLElement ? e.target.id : '';
    if (!id.startsWith('f-') || !$('[aria-invalid="true"]', form)) return;
    const key = id.slice(2);
    const still = new Set(validate(read().brief).map((p) => errorKey(p.code)));
    for (const k of key === 'email' || key === 'telegram' ? [key, 'contact'] : [key]) if (!still.has(k)) hide(k);
    sync();
  });

  // ---- sending ----
  const post = async (body: Record<string, unknown>): Promise<Answer> => {
    let res: Response;
    try {
      res = await fetch(form.getAttribute('action') || '/api/contact', {
        method: 'POST',
        headers: { 'content-type': 'application/json', accept: 'application/json' },
        body: JSON.stringify(body),
      });
    } catch {
      return { ok: false, errors: [{ field: 'form', code: 'network' }] };
    }
    if (res.ok) return { ok: true };
    let errors: Problem[] = [];
    try {
      errors = ((await res.json()) as { errors?: Problem[] }).errors ?? [];
    } catch {
      /* not JSON: fall back to the status code */
    }
    if (!errors.length) {
      errors = [{ field: 'form', code: res.status === 429 ? 'rate-limited' : res.status === 413 ? 'too-large' : 'server-error' }];
    }
    return { ok: false, errors };
  };

  const payload = (data: FormData, b: Brief, pow: Solved | null): Record<string, unknown> => {
    const body: Record<string, unknown> = {
      lang: data.get('lang') || lang(),
      kind: b.kind,
      name: b.name.trim(),
      website: data.get('website') ?? '',
    };
    if (b.message.trim()) body['message'] = b.message;
    for (const k of ['timeline', 'budget', 'email', 'telegram', 'company'] as const) {
      const v = b[k].trim();
      if (v) body[k] = v;
    }
    if (pow) {
      body['pow_challenge'] = pow.challenge;
      body['pow_counter'] = pow.counter;
    }
    return body;
  };

  const sent = () => {
    const done = h(
      'div',
      { class: 'contact-done', tabindex: '-1' },
      h('p', { class: 'contact-done__title' }, t('contact-sent')),
      h('p', null, t('contact-sent-reply')),
    );
    for (const el of Array.from(form.children)) if (el !== status && el instanceof HTMLElement) el.hidden = true;
    say('');
    form.insertBefore(done, status);
    done.focus();
    emit('contact:sent', undefined);
  };

  let busy = false;
  const setBusy = (on: boolean) => {
    if (!submit) return;
    if (on) {
      submit.textContent = t('contact-sending');
      submit.setAttribute('aria-disabled', 'true');
    } else {
      submit.replaceChildren(...label);
      submit.removeAttribute('aria-disabled');
    }
  };

  form.addEventListener('submit', async (e) => {
    e.preventDefault();
    if (busy) return;
    const { data, brief } = read();
    const problems = validate(brief);
    if (problems.length) return show(problems);
    show([]);
    busy = true;
    setBusy(true);
    say(t('contact-sending'));
    try {
      // A rejected proof (expired while the visitor wrote) is redone once, without asking them.
      for (let attempt = 0; ; attempt++) {
        const pow = await proof();
        const answer = await post(payload(data, brief, pow));
        if (answer.ok) return sent();
        if (pow && attempt === 0 && answer.errors.some((x) => x.code.startsWith('pow-'))) continue;
        show(answer.errors);
        void startPow(); // ready for the next try
        return;
      }
    } finally {
      busy = false;
      setBusy(false);
    }
  });

  form.addEventListener('focusin', () => void startPow(), { once: true });
  // Our checks give specific, announced messages; the server checks again either way.
  form.noValidate = true;
}
