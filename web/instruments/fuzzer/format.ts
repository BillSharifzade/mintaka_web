// Formatting helpers for the fuzzer island. Pure functions, unit-tested.

/** "4F 52 49 02" — at most `max` bytes, then "…". */
export function hex(bytes: Uint8Array, max = Infinity): string {
  const shown = Array.from(bytes.subarray(0, Math.min(bytes.length, max)), (b) => b.toString(16).toUpperCase().padStart(2, '0'));
  return bytes.length > max ? `${shown.join(' ')} …` : shown.join(' ');
}

/** Printable ASCII as itself, everything else as "·". */
export function ascii(bytes: Uint8Array, max = Infinity): string {
  let out = '';
  for (const b of bytes.subarray(0, Math.min(bytes.length, max))) out += b >= 0x20 && b < 0x7f ? String.fromCharCode(b) : '·';
  return bytes.length > max ? `${out}…` : out;
}

/**
 * Parses what a visitor types: text with `\xNN` escapes ("ORI\x02\x01"), or plain hex when the
 * whole input looks like hex pairs ("4F 52 49 02"). Returns null for an invalid escape.
 */
export function parseInput(text: string, maxLen = 64): Uint8Array | null {
  const trimmed = text.trim();
  const out: number[] = [];
  if (/^([0-9a-f]{2}[\s,]*)+$/i.test(trimmed) && /\s/.test(trimmed)) {
    for (const pair of trimmed.match(/[0-9a-f]{2}/gi) ?? []) out.push(parseInt(pair, 16));
  } else {
    for (let i = 0; i < text.length; i++) {
      const c = text.charCodeAt(i);
      if (text[i] === '\\' && text[i + 1] === 'x') {
        const pair = text.slice(i + 2, i + 4);
        if (!/^[0-9a-f]{2}$/i.test(pair)) return null;
        out.push(parseInt(pair, 16));
        i += 3;
      } else if (text[i] === '\\' && text[i + 1] === '\\') {
        out.push(0x5c);
        i += 1;
      } else if (c < 0x80) {
        out.push(c);
      } else {
        const ch = String.fromCodePoint(text.codePointAt(i)!);
        for (const b of new TextEncoder().encode(ch)) out.push(b);
        i += ch.length - 1;
      }
    }
  }
  return Uint8Array.from(out.slice(0, maxLen));
}

/** Bytes back to the escaped form `parseInput` reads. */
export function escapeInput(bytes: Uint8Array): string {
  let out = '';
  for (const b of bytes) {
    if (b === 0x5c) out += '\\\\';
    else if (b >= 0x20 && b < 0x7f) out += String.fromCharCode(b);
    else out += `\\x${b.toString(16).toUpperCase().padStart(2, '0')}`;
  }
  return out;
}

/** 1284551 → "1,284,551" (locale-aware). */
export function count(n: number, locale?: string): string {
  return Math.round(n).toLocaleString(locale);
}

/** 2_140_000 → "2.1 M", 48_200 → "48 k", 900 → "900". */
export function rate(perSecond: number, locale?: string): string {
  const f = (v: number, d: number) => v.toLocaleString(locale, { maximumFractionDigits: d });
  if (perSecond >= 1e6) return `${f(perSecond / 1e6, 1)} M`;
  if (perSecond >= 1e4) return `${f(perSecond / 1e3, 0)} k`;
  return f(perSecond, 0);
}

/** Seconds with sensible precision: 0.004 s · 0.62 s · 12.4 s. */
export function seconds(ms: number, locale?: string): string {
  const s = ms / 1000;
  const digits = s < 0.1 ? 3 : s < 10 ? 2 : 1;
  return `${s.toLocaleString(locale, { minimumFractionDigits: digits, maximumFractionDigits: digits })} s`;
}

/** Splits "panicked at path:line:col:\nmessage" into its two parts. */
export function splitPanic(message: string): { location: string; text: string } {
  const m = /^panicked at (.+?):\n([\s\S]*)$/.exec(message.trim());
  return m ? { location: m[1]!, text: m[2]! } : { location: '', text: message.trim() };
}
