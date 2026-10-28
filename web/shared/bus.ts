// A tiny typed event bus on `document`. The last value of every event is remembered; state-like
// events (`tier`) are read with `{ replay: true }` so a late subscriber still hears them.
// Command-like events (`proof:open`) are not replayed.

export type Tier = 'A' | 'B' | 'C' | 'D';

export interface Events {
  /** The hero engine settled on a render tier (A WebGPU · B WebGL2 · C Canvas 2D · D poster). */
  tier: Tier;
  /** Ask the proof console to open (hero "See the proof", footer link, terminal `proof`). */
  'proof:open': undefined;
  /** Ask the terminal to open. */
  'terminal:open': undefined;
  /** The contact form was sent; the hero/footer may celebrate. */
  'contact:sent': undefined;
  /** Terminal `sky`: pause or resume the hero star field. */
  'sky:toggle': undefined;
  /** The hero star field was paused (false) or resumed (true) — state, read with replay. */
  sky: boolean;
}

const last = new Map<keyof Events, unknown>();

export function emit<K extends keyof Events>(type: K, detail: Events[K]): void {
  last.set(type, detail);
  document.dispatchEvent(new CustomEvent(`mintaka:${type}`, { detail }));
}

/** Subscribe; returns an unsubscribe function. With `replay`, also hears the last value now. */
export function on<K extends keyof Events>(
  type: K,
  fn: (detail: Events[K]) => void,
  opts: { replay?: boolean } = {},
): () => void {
  const handler = (e: Event) => fn((e as CustomEvent<Events[K]>).detail);
  document.addEventListener(`mintaka:${type}`, handler);
  if (opts.replay && last.has(type)) fn(last.get(type) as Events[K]);
  return () => document.removeEventListener(`mintaka:${type}`, handler);
}

export function lastValue<K extends keyof Events>(type: K): Events[K] | undefined {
  return last.get(type) as Events[K] | undefined;
}
