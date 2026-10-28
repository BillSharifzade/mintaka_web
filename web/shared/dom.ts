// Safe DOM construction. The page enforces Trusted Types (`require-trusted-types-for 'script'`),
// so `innerHTML`, `outerHTML`, `insertAdjacentHTML` and `document.write` throw. Build nodes with
// `h()` / `s()` and set text with `textContent`. Inline `style` attributes are blocked by CSP;
// set dynamic values through the CSSOM: `el.style.setProperty('--x', '12px')`.

export type Child = Node | string | number | null | undefined | false | readonly Child[];
type Listener = (ev: any) => void;
export type Attrs = Record<string, string | number | boolean | null | undefined | Listener>;

const SVG_NS = 'http://www.w3.org/2000/svg';

function apply(el: Element, attrs: Attrs | null | undefined): void {
  if (!attrs) return;
  for (const [key, value] of Object.entries(attrs)) {
    if (value === null || value === undefined || value === false) continue;
    if (key === 'style') throw new Error('inline style attributes are blocked by CSP; use el.style.setProperty()');
    if (typeof value === 'function') {
      if (!key.startsWith('on')) throw new Error(`function given for attribute "${key}"`);
      el.addEventListener(key.slice(2).toLowerCase(), value);
    } else if (value === true) {
      el.setAttribute(key, '');
    } else {
      el.setAttribute(key === 'className' ? 'class' : key, String(value));
    }
  }
}

function append(el: Node, children: readonly Child[]): void {
  for (const child of children) {
    if (child === null || child === undefined || child === false) continue;
    if (Array.isArray(child)) append(el, child);
    else if (child instanceof Node) el.appendChild(child);
    else el.appendChild(document.createTextNode(String(child)));
  }
}

/** Create an HTML element: `h('button', { class: 'btn', onclick: go }, 'Go')`. */
export function h<K extends keyof HTMLElementTagNameMap>(
  tag: K,
  attrs?: Attrs | null,
  ...children: Child[]
): HTMLElementTagNameMap[K] {
  const el = document.createElement(tag);
  apply(el, attrs);
  append(el, children);
  return el;
}

/** Create an SVG element: `s('circle', { cx: 4, cy: 4, r: 2 })`. */
export function s<K extends keyof SVGElementTagNameMap>(
  tag: K,
  attrs?: Attrs | null,
  ...children: Child[]
): SVGElementTagNameMap[K] {
  const el = document.createElementNS(SVG_NS, tag);
  apply(el, attrs);
  append(el, children);
  return el;
}

/** A measured value, `<span class="num">21<span class="num__unit">µs</span></span>`, from format.ts. */
export function num(f: { value: string; unit: string }, cls?: string): HTMLSpanElement {
  return h('span', { class: cls ? `num ${cls}` : 'num' }, f.value, f.unit ? h('span', { class: 'num__unit' }, f.unit) : null);
}

/** Replace all children of `el`. */
export function render(el: Element, ...children: Child[]): void {
  el.replaceChildren();
  append(el, children);
}

export const $ = <T extends Element = HTMLElement>(sel: string, root: ParentNode = document): T | null =>
  root.querySelector<T>(sel);

export const $$ = <T extends Element = HTMLElement>(sel: string, root: ParentNode = document): T[] =>
  Array.from(root.querySelectorAll<T>(sel));

/** True when keyboard focus is in a text field (single-key shortcuts must stay quiet then). */
export function isTyping(target: EventTarget | null): boolean {
  if (!(target instanceof HTMLElement)) return false;
  return target.isContentEditable || /^(INPUT|TEXTAREA|SELECT)$/.test(target.tagName);
}
