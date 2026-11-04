/**
 * Buttons that lean towards a mouse pointer, their label a little further (a sense of depth). Mouse
 * only: touch and pen get the plain press feedback from CSS. The element's rest position is
 * measured on entry, so its own movement never feeds back into the pull.
 */

export function magnetic(): void {
  for (const el of document.querySelectorAll<HTMLElement>('[data-magnetic]')) {
    const inner = el.querySelector<HTMLElement>('.btn__in');
    const pull = 0.16;
    let rest: DOMRect | null = null;
    el.addEventListener('pointerenter', (e) => {
      if (e.pointerType !== 'mouse') return;
      el.style.translate = '';
      rest = el.getBoundingClientRect();
    });
    el.addEventListener('pointermove', (e) => {
      if (!rest || e.pointerType !== 'mouse') return;
      const x = e.clientX - (rest.left + rest.width / 2);
      const y = e.clientY - (rest.top + rest.height / 2);
      el.style.translate = `${(x * pull).toFixed(1)}px ${(y * pull * 1.2).toFixed(1)}px`;
      if (inner) inner.style.translate = `${(x * pull * 0.45).toFixed(1)}px ${(y * pull * 0.5).toFixed(1)}px`;
    });
    el.addEventListener('pointerleave', () => {
      rest = null;
      el.style.translate = '';
      if (inner) inner.style.translate = '';
    });
  }
  // A scroll moves the page under a still pointer: let go rather than pull from a stale position.
  addEventListener(
    'scroll',
    () => {
      for (const el of document.querySelectorAll<HTMLElement>('[data-magnetic]')) {
        if (el.style.translate) el.dispatchEvent(new PointerEvent('pointerleave'));
      }
    },
    { passive: true },
  );
}
