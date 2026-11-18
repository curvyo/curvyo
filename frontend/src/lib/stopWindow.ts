/**
 * The rows of a long stop list that are drawn (`specs/0124-gradient-fill` UX review U4): above
 * `STOP_WINDOW_FROM` stops the list scrolls in a box of `STOP_WINDOW_ROWS` rows and only the
 * rows in view, plus a margin, are rendered, so a gradient of any length keeps the panel fast.
 */

/** Stop lists longer than this are windowed. */
export const STOP_WINDOW_FROM = 40;
/** Rows shown in the scroll box. */
export const STOP_WINDOW_ROWS = 12;
/** One row's pitch in CSS pixels: a 28 px row and its 4 px gap. */
export const STOP_ROW_PX = 32;
/** Rows rendered above and below the visible ones. */
const MARGIN_ROWS = 4;

/** The rendered range `[first, end)` of `count` rows scrolled by `scrollTop` pixels. */
export interface StopWindow {
  first: number;
  end: number;
}

/** Which rows to render; every row when the list is not windowed. */
export function stopWindow(count: number, scrollTop: number): StopWindow {
  if (count <= STOP_WINDOW_FROM) {
    return { first: 0, end: count };
  }
  const top = Math.max(0, Math.floor(scrollTop / STOP_ROW_PX));
  const first = Math.max(0, Math.min(top, count - STOP_WINDOW_ROWS) - MARGIN_ROWS);
  const end = Math.min(count, top + STOP_WINDOW_ROWS + MARGIN_ROWS);
  return { first, end };
}

/** The `scrollTop` that brings row `index` into a box already scrolled by `scrollTop`. */
export function scrollToRow(index: number, scrollTop: number): number {
  const top = index * STOP_ROW_PX;
  const bottom = top + STOP_ROW_PX - (STOP_WINDOW_ROWS * STOP_ROW_PX);
  if (top < scrollTop) {
    return top;
  }
  if (bottom > scrollTop) {
    return bottom;
  }
  return scrollTop;
}
