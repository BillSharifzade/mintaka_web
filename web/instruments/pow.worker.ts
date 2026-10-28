// Proof-of-work worker for the contact form (its own esbuild entry, started by contact.ts with
// worker(assetUrl('pow.worker'))), so solving never competes with the visitor's typing.
//
// In: `{ challenge, difficulty }`. Out: `{ type: 'progress', tried }` about four times a second,
// then `{ type: 'done', counter, tried, ms }` or `{ type: 'error', message }`.

import { fromBase64url, solve } from './pow.ts';

interface Job {
  challenge: string;
  difficulty: number;
}

// The worker's global scope (the project's TypeScript lib is "dom", not "webworker").
interface Scope {
  onmessage: ((e: MessageEvent<Job>) => void) | null;
  postMessage(message: unknown): void;
}

const scope = globalThis as unknown as Scope;

scope.onmessage = ({ data }) => {
  const t0 = performance.now();
  let said = t0;
  try {
    const counter = solve(fromBase64url(data.challenge), data.difficulty, (tried) => {
      const now = performance.now();
      if (now - said < 250) return;
      said = now;
      scope.postMessage({ type: 'progress', tried });
    });
    scope.postMessage({ type: 'done', counter, tried: counter + 1, ms: performance.now() - t0 });
  } catch (err) {
    scope.postMessage({ type: 'error', message: String(err) });
  }
};
