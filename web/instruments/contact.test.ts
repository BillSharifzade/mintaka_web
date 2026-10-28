import { assert, test } from './testkit.ts';
import { emailOk, errorKey, messageChars, validate, type Brief } from './contact.ts';

const brief = (b: Partial<Brief>): Brief => ({
  kind: [],
  message: '',
  timeline: '',
  budget: '',
  name: 'Bill',
  email: 'bill@example.org',
  telegram: '',
  company: '',
  ...b,
});
const codes = (b: Partial<Brief>) => validate(brief(b)).map((p) => p.code);

test('a complete brief passes', () => {
  assert.deepEqual(codes({}), []);
  assert.deepEqual(
    codes({ kind: ['new-product', 'ai'], timeline: '1-3-months', budget: '10-50k', telegram: '@mintaka_tj', company: 'Orion' }),
    [],
  );
});

test('name: required, at most 120 characters once whitespace runs are collapsed', () => {
  assert.deepEqual(codes({ name: '   ' }), ['name-required']);
  assert.deepEqual(codes({ name: '\u0000\t' }), ['name-required'], 'control characters count as blank');
  assert.deepEqual(codes({ name: 'x'.repeat(120) }), []);
  assert.deepEqual(codes({ name: 'x'.repeat(121) }), ['name-too-long']);
  assert.deepEqual(codes({ name: `  ${'x'.repeat(120)}  ` }), [], 'measured after trimming');
  // The server's one_line(): "a \n\t b" is "a b" — so 60 + gap + 59 is 120 characters, not 125.
  assert.deepEqual(codes({ name: `${'a'.repeat(60)} \n\t  ${'b'.repeat(59)}` }), []);
  // Characters, not UTF-16 units: 120 astral symbols are 240 units but 120 characters.
  assert.deepEqual(codes({ name: '𝔐'.repeat(120) }), []);
  assert.deepEqual(codes({ name: 'ж'.repeat(120) }), []);
});

test('email or Telegram: at least one', () => {
  assert.deepEqual(codes({ email: '', telegram: '' }), ['contact-required']);
  assert.deepEqual(codes({ email: '  ', telegram: ' ' }), ['contact-required']);
  assert.deepEqual(codes({ email: '', telegram: 'durov_x' }), []);
});

test('email: exactly the server\'s Email newtype', () => {
  // The cases from crates/server/src/contact/fields.rs, test `emails`.
  for (const ok of ['a@b.tj', 'first.last+tag@sub.example.com', 'x_y@a-b.io']) assert.ok(emailOk(ok), ok);
  for (const bad of ['', 'a', 'a@b', '@b.tj', 'a@.tj', 'a@b..tj', 'a@-b.tj', 'a b@c.tj', 'a@b_c.tj', 'ä@b.tj']) {
    assert.ok(!emailOk(bad), bad);
  }
  assert.ok(emailOk(`${'a'.repeat(64)}@${'b'.repeat(63)}.tj`), '64-character local part, 63-character label');
  assert.ok(!emailOk(`${'a'.repeat(65)}@b.tj`), '65-character local part');
  assert.ok(!emailOk(`a@${'b'.repeat(64)}.tj`), '64-character label');
  // More of the same grammar: the WHATWG local-part characters, labels that end in a hyphen,
  // a trailing dot, two @, and the 254-character ceiling.
  assert.ok(emailOk("o'brien!#$%&*/=?^_`{|}~-@x.tj"));
  assert.ok(emailOk('.a..b.@x.tj'), 'dots anywhere in the local part, as WHATWG allows');
  for (const bad of ['a@b-.tj', 'a@b.tj.', 'a@b@c.tj', 'a@b.t j', 'a@xn--ü.tj']) assert.ok(!emailOk(bad), bad);
  const long = (n: number) => `${'a'.repeat(64)}@${['b'.repeat(63), 'c'.repeat(63), 'd'.repeat(63)].join('.')}`.slice(0, n);
  assert.equal(long(254).length, 254);
  assert.ok(emailOk(`${'a'.repeat(64)}@${'b'.repeat(63)}.${'c'.repeat(63)}.${'d'.repeat(61)}`), '254 characters');
  assert.ok(!emailOk(`${'a'.repeat(64)}@${'b'.repeat(63)}.${'c'.repeat(63)}.${'d'.repeat(62)}`), '255 characters');
  // In the brief: trimmed first, as the server does.
  assert.deepEqual(codes({ email: ' bill@mintaka.tj ' }), []);
  assert.deepEqual(codes({ email: 'bill@mintaka' }), ['email-invalid']);
});

test('telegram: @?[A-Za-z][A-Za-z0-9_]{4,31}', () => {
  for (const ok of ['durov', '@durov', 'a_1234', 'A'.repeat(32), `@${'z'.repeat(32)}`]) {
    assert.deepEqual(codes({ telegram: ok }), [], ok);
  }
  for (const bad of ['dur', '@abcd', '1durov', '_durov', 'du-rov', '@min-taka', 'd'.repeat(33), '@@durov', 't.me/durov', '@']) {
    assert.deepEqual(codes({ telegram: bad }), ['telegram-invalid'], bad);
  }
});

test('message: ≤ 5000 characters after the server\'s normalising', () => {
  assert.deepEqual(codes({ message: 'm'.repeat(5000) }), []);
  assert.deepEqual(codes({ message: 'm'.repeat(5001) }), ['message-too-long']);
  assert.deepEqual(codes({ message: 'é'.repeat(5001) }), ['message-too-long']);
  // CRLF is one character, the trailing newline is trimmed, NUL is dropped (test messages_count_crlf_as_one).
  assert.equal(messageChars('a\r\n'.repeat(2500)), 2 * 2500 - 1);
  assert.equal(messageChars('a\u0000b'), 2);
  assert.equal(messageChars('\ta\tb\n'), 3, 'inner tabs stay; the ends are trimmed');
  assert.deepEqual(codes({ message: 'm\r\n'.repeat(2499) + 'mm' }), [], '5000 once CRLF counts as one');
  assert.deepEqual(codes({ message: 'm\r\n'.repeat(2500) + 'm' }), ['message-too-long'], '5001');
  assert.deepEqual(codes({ message: '\u0000'.repeat(6000) + 'm' }), [], 'control characters do not count');
});

test('company: ≤ 120 characters on one line', () => {
  assert.deepEqual(codes({ company: 'c'.repeat(121) }), ['company-too-long']);
  assert.deepEqual(codes({ company: `${'c'.repeat(60)}\n\n\n${'d'.repeat(59)}` }), [], 'line breaks collapse to one space');
});

test('choices outside the lists are rejected', () => {
  assert.deepEqual(codes({ kind: ['new-product', 'crypto'] }), ['choice-invalid']);
  assert.deepEqual(codes({ timeline: 'yesterday' }), ['choice-invalid']);
  assert.deepEqual(codes({ budget: 'lt-1k' }), ['choice-invalid']);
});

test('problems come in the server\'s order (test reports_every_error_in_form_order)', () => {
  const all = validate(
    brief({ kind: ['rockets'], message: 'x'.repeat(5001), timeline: 'tomorrow', name: 'x'.repeat(121), email: 'not-an-email', telegram: '@abc', company: 'c'.repeat(121) }),
  );
  assert.deepEqual(
    all.map((p) => p.code),
    ['choice-invalid', 'message-too-long', 'choice-invalid', 'name-too-long', 'email-invalid', 'telegram-invalid', 'company-too-long'],
  );
  assert.deepEqual(codes({ name: '', email: '' }), ['name-required', 'contact-required']);
  assert.deepEqual(codes({ email: '', company: 'c'.repeat(121) }), ['contact-required', 'company-too-long'], 'contact before company');
});

test('errorKey maps codes to their e-* element by prefix', () => {
  assert.equal(errorKey('email-invalid'), 'email');
  assert.equal(errorKey('contact-required'), 'contact');
  assert.equal(errorKey('message-too-long'), 'message');
  assert.equal(errorKey('rate-limited'), 'rate');
  assert.equal(errorKey('pow-expired'), 'pow');
});
