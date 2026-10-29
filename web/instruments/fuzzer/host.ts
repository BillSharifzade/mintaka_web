// Host for the `fuzzer` WebAssembly module (crates/fuzzer): runs fuzzing sessions, catches the
// buggy parser's crash as a trap, reads Rust's own panic message, and minimises crashing inputs.
// Plain WebAssembly and typed arrays — no bindgen glue — so it runs the same in `node --test`.

export type Mode = 'guided' | 'blind';
export type Build = 'buggy' | 'fixed';

export interface Discovery {
  /** 1-based execution number at which these points were first reached. */
  exec: number;
  points: number[];
  input: Uint8Array;
}

export interface Crash {
  exec: number;
  input: Uint8Array;
  /** Rust's panic message, e.g. "panicked at crates/fuzzer/src/target.rs:129:42:\nrange end index …". */
  message: string;
}

export interface StepResult {
  discoveries: Discovery[];
  crash: Crash | null;
}

/** Parse outcome codes, mirroring `ParseError` in crates/fuzzer/src/target.rs (0 = parsed). */
export const PARSE_CODES = [
  'ok',
  'bad-magic',
  'bad-version',
  'no-stars',
  'too-many-stars',
  'truncated',
  'reserved-flags',
  'unknown-section',
  'bad-line',
] as const;
export type ParseCode = (typeof PARSE_CODES)[number];

export interface ExecResult {
  code: ParseCode | 'crash';
  points: number[];
  message: string | null;
}

interface Exports {
  memory: WebAssembly.Memory;
  fz_init(seedLo: number, seedHi: number, mode: number, build: number): void;
  fz_run(budget: number): number;
  fz_execs(): number;
  fz_cov_lo(): number;
  fz_cov_hi(): number;
  fz_corpus_len(): number;
  fz_event_count(): number;
  fz_event_exec(i: number): number;
  fz_event_bits_lo(i: number): number;
  fz_event_bits_hi(i: number): number;
  fz_event_input(i: number): number;
  fz_event_input_len(i: number): number;
  fz_current(): number;
  fz_current_len(): number;
  fz_panic(): number;
  fz_panic_len(): number;
  fz_scratch(): number;
  fz_exec(len: number, build: number): number;
  fz_last_cov_lo(): number;
  fz_last_cov_hi(): number;
  fz_points(): number;
  fz_max_len(): number;
}

const decoder = new TextDecoder();

function instantiate(module: WebAssembly.Module): Exports {
  return new WebAssembly.Instance(module, {}).exports as unknown as Exports;
}

function bytesAt(x: Exports, ptr: number, len: number): Uint8Array {
  return new Uint8Array(x.memory.buffer, ptr, len).slice();
}

function panicMessage(x: Exports): string {
  return decoder.decode(bytesAt(x, x.fz_panic(), x.fz_panic_len()));
}

/** Point ids set in a 64-bit mask given as two 32-bit halves. */
export function maskToPoints(lo: number, hi: number): number[] {
  const out: number[] = [];
  for (let b = 0; b < 32; b++) if ((lo >>> b) & 1) out.push(b);
  for (let b = 0; b < 32; b++) if ((hi >>> b) & 1) out.push(32 + b);
  return out;
}

/** Two random 32-bit halves of a seed. */
export function randomSeed(): [number, number] {
  const s = new Uint32Array(2);
  crypto.getRandomValues(s);
  return [s[0]!, s[1]!];
}

/** Compiles the module once; instances are cheap after that. */
export async function compile(source: Response | Promise<Response> | BufferSource): Promise<WebAssembly.Module> {
  if (source instanceof Response || source instanceof Promise) {
    const response = await source;
    if ('compileStreaming' in WebAssembly && response.headers.get('content-type')?.startsWith('application/wasm')) {
      return WebAssembly.compileStreaming(response);
    }
    return WebAssembly.compile(await response.arrayBuffer());
  }
  return WebAssembly.compile(source);
}

/** One fuzzing session on its own instance. After a crash the instance is spent. */
export class FuzzRun {
  readonly mode: Mode;
  readonly build: Build;
  readonly points: number;
  crash: Crash | null = null;
  private readonly x: Exports;
  private seen = 0;

  constructor(module: WebAssembly.Module, opts: { mode: Mode; build: Build; seed?: [number, number] }) {
    this.mode = opts.mode;
    this.build = opts.build;
    this.x = instantiate(module);
    this.points = this.x.fz_points();
    const [lo, hi] = opts.seed ?? randomSeed();
    this.x.fz_init(lo, hi, opts.mode === 'blind' ? 1 : 0, opts.build === 'fixed' ? 1 : 0);
  }

  get execs(): number {
    return this.x.fz_execs();
  }

  get corpus(): number {
    return this.x.fz_corpus_len();
  }

  get covered(): number[] {
    return maskToPoints(this.x.fz_cov_lo(), this.x.fz_cov_hi());
  }

  /** Executes up to `budget` inputs. Returns new discoveries, and the crash if one happened. */
  step(budget: number): StepResult {
    if (this.crash) return { discoveries: [], crash: null };
    let crash: Crash | null = null;
    try {
      this.x.fz_run(Math.max(1, Math.floor(budget)));
    } catch (error) {
      if (!(error instanceof WebAssembly.RuntimeError)) throw error;
      // Only atomics-backed getters below: safe to call on a trapped instance.
      crash = {
        exec: this.x.fz_execs(),
        input: bytesAt(this.x, this.x.fz_current(), this.x.fz_current_len()),
        message: panicMessage(this.x),
      };
      this.crash = crash;
    }
    return { discoveries: this.drain(), crash };
  }

  private drain(): Discovery[] {
    const out: Discovery[] = [];
    const count = this.x.fz_event_count();
    for (; this.seen < count; this.seen++) {
      const i = this.seen;
      out.push({
        exec: this.x.fz_event_exec(i),
        points: maskToPoints(this.x.fz_event_bits_lo(i), this.x.fz_event_bits_hi(i)),
        input: bytesAt(this.x, this.x.fz_event_input(i), this.x.fz_event_input_len(i)),
      });
    }
    return out;
  }
}

/** Runs single inputs through the parser — for "try it yourself" and for minimisation. */
export class Probe {
  private readonly module: WebAssembly.Module;
  private x: Exports | null = null;
  readonly maxLen: number;

  constructor(module: WebAssembly.Module) {
    this.module = module;
    this.maxLen = this.instance().fz_max_len();
  }

  private instance(): Exports {
    this.x ??= instantiate(this.module);
    return this.x;
  }

  run(input: Uint8Array, build: Build): ExecResult {
    const x = this.instance();
    const bytes = input.subarray(0, this.maxLen);
    new Uint8Array(x.memory.buffer, x.fz_scratch(), bytes.length).set(bytes);
    try {
      const code = x.fz_exec(bytes.length, build === 'fixed' ? 1 : 0);
      return {
        code: PARSE_CODES[code] ?? 'bad-magic',
        points: maskToPoints(x.fz_last_cov_lo(), x.fz_last_cov_hi()),
        message: null,
      };
    } catch (error) {
      if (!(error instanceof WebAssembly.RuntimeError)) throw error;
      const message = panicMessage(x);
      this.x = null; // a trapped instance is never reused
      return { code: 'crash', points: [], message };
    }
  }

  crashes(input: Uint8Array, build: Build = 'buggy'): boolean {
    return this.run(input, build).code === 'crash';
  }
}

function without(input: Uint8Array, at: number, size: number): Uint8Array<ArrayBuffer> {
  const out = new Uint8Array(input.length - size);
  out.set(input.subarray(0, at), 0);
  out.set(input.subarray(at + size), at);
  return out;
}

/**
 * Shrinks a failing input while it keeps failing, until nothing more can go: remove chunks of
 * halving size, then lower each byte to 0x00 or 0x01 where the failure doesn't care (which can
 * unlock further removals, e.g. a count dropping from 2 to 1). Deterministic for a deterministic
 * predicate; at most `maxTests` predicate calls.
 */
export function minimize(input: Uint8Array, fails: (candidate: Uint8Array) => boolean, maxTests = 4000): Uint8Array {
  let current: Uint8Array = input.slice();
  let tests = 0;
  const check = (candidate: Uint8Array) => ++tests <= maxTests && candidate.length > 0 && fails(candidate);
  for (let changed = true; changed && tests < maxTests; ) {
    changed = false;
    for (let size = Math.max(1, current.length >> 1); size >= 1; size >>= 1) {
      for (let at = 0; at < current.length; ) {
        const candidate = without(current, at, Math.min(size, current.length - at));
        if (check(candidate)) {
          current = candidate;
          changed = true;
        } else at += size;
      }
    }
    for (let i = 0; i < current.length; i++) {
      for (const value of [0x00, 0x01]) {
        if (current[i]! <= value) break;
        const candidate = current.slice();
        candidate[i] = value;
        if (check(candidate)) {
          current = candidate;
          changed = true;
          break;
        }
      }
    }
  }
  return current;
}
