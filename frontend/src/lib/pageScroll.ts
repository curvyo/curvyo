// Keeps the app page from scrolling: the app is one fixed screen, and a focused field past the
// view (the text edit sink at a caret below the canvas, customer report 2026-10-24) must never make
// the webview scroll the shell to reveal it.

/** The attribute that marks a box which must never scroll (the app root). */
export const SCROLL_LOCK_ATTRIBUTE = "data-scroll-lock";

/** The text edit sink's point in the canvas box: the caret, held inside the box's `width` x
 * `height` CSS pixels so the 1 x 1 px sink is never outside the visible canvas. No caret, an
 * unmeasured box or a non-number coordinate gives the box's first pixel on that axis. */
export function clampSinkPoint(
  caret: { x: number; y: number } | null,
  width: number,
  height: number,
): { x: number; y: number } {
  const clamp = (value: number, size: number) =>
    size < 1 || Number.isNaN(value) ? 0 : Math.min(Math.max(value, 0), size - 1);
  return caret ? { x: clamp(caret.x, width), y: clamp(caret.y, height) } : { x: 0, y: 0 };
}

/** Resets any scroll of the document or of a box marked with `SCROLL_LOCK_ATTRIBUTE` the moment
 * it happens (a webview scrolls even an `overflow: hidden` box to reveal a focused field or the
 * caret). Returns the function that removes the guard. */
export function lockPageScroll(doc: Document): () => void {
  const reset = (event: Event) => {
    const target = event.target;
    if (target === doc) {
      doc.scrollingElement?.scrollTo(0, 0);
    } else if (target instanceof Element && target.hasAttribute(SCROLL_LOCK_ATTRIBUTE)) {
      target.scrollTo(0, 0);
    }
  };
  doc.addEventListener("scroll", reset, { capture: true, passive: true });
  return () => doc.removeEventListener("scroll", reset, { capture: true });
}
