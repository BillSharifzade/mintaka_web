// Node's test runner, assertions and SHA-256 for the worker-C unit tests (*.test.ts next to the
// islands). The project has no @types/node, so the modules are loaded by a name the type checker
// does not try to resolve, and typed here with only what the tests use. Browser code never
// imports this file.

type Body = () => void | Promise<void>;

export interface Assert {
  equal(actual: unknown, expected: unknown, message?: string): void;
  notEqual(actual: unknown, expected: unknown, message?: string): void;
  deepEqual(actual: unknown, expected: unknown, message?: string): void;
  ok(value: unknown, message?: string): void;
  throws(fn: () => unknown, expected?: RegExp, message?: string): void;
}

const load = (name: string): Promise<any> => import(name);

export const test: (name: string, body: Body) => void = (await load('node:test')).test;
export const assert: Assert = (await load('node:assert/strict')).default;

const crypto = await load('node:crypto');

/** Reference SHA-256 (node:crypto). */
export function nodeSha256(data: Uint8Array): Uint8Array {
  return new Uint8Array(crypto.createHash('sha256').update(data).digest());
}

/** `actual` within `eps` of `expected`. */
export function near(actual: number, expected: number, eps = 1e-9, what = ''): void {
  assert.ok(Math.abs(actual - expected) <= eps, `${what} ${actual} is not within ${eps} of ${expected}`);
}

export const hex = (b: Uint8Array): string => Array.from(b, (x) => x.toString(16).padStart(2, '0')).join('');
