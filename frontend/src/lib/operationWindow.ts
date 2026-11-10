/**
 * The window of the Operations list (`specs/0101-laser-operations` `ux-notes.md` section 12): the
 * entries whose boxes meet the viewport, with an overscan, plus the focused and the open entry
 * (kept in the DOM even off screen). The spacers before and after carry the rest of the height, so
 * the scrollbar is exact from the first frame.
 */

/** The entries to render: `first` up to `last` (exclusive), and the heights above and below. */
export interface WindowRange {
  first: number;
  last: number;
  before: number;
  after: number;
}

/**
 * The window for entries of the given `heights` when the list is scrolled to `scrollTop` in a
 * viewport `viewport` high. `pinned` entries (the focused one, the open one) are always included.
 */
export function visibleRange(
  heights: readonly number[],
  scrollTop: number,
  viewport: number,
  overscan: number,
  focused: number | null,
  open: number | null,
): WindowRange {
  if (heights.length === 0) return { first: 0, last: 0, before: 0, after: 0 };
  const top = Math.max(0, scrollTop - overscan);
  const bottom = scrollTop + viewport + overscan;
  let offset = 0;
  let first = -1;
  let last = heights.length;
  for (let index = 0; index < heights.length; index += 1) {
    const next = offset + heights[index];
    if (first < 0 && next > top) first = index;
    if (offset >= bottom) {
      last = index;
      break;
    }
    offset = next;
  }
  if (first < 0) first = heights.length - 1;
  for (const pinned of [focused, open]) {
    if (pinned !== null && pinned >= 0 && pinned < heights.length) {
      first = Math.min(first, pinned);
      last = Math.max(last, pinned + 1);
    }
  }
  let before = 0;
  for (let index = 0; index < first; index += 1) before += heights[index];
  let after = 0;
  for (let index = last; index < heights.length; index += 1) after += heights[index];
  return { first, last, before, after };
}
