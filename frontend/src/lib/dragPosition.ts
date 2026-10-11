/** Where a value-field drag is on the scale (`specs/0017-style-panel-rework`
 * criteria 38 and 44): no React, so a test can drive it. */

/** What a drag knows about itself. */
export interface DragFrame {
  /** The edited objects differed when the press happened. Latched there: the
   * preview makes them equal after the first tick, but the drag stays absolute. */
  mixed: boolean;
  /** The position and pointer x the factor is applied from (relative drags). */
  baseP: number;
  baseX: number;
  factor: number;
  /** The field's left edge and width, px. */
  left: number;
  width: number;
  /** The largest position: 1 at the end of the track, more for a field whose typed range goes
   * beyond it (a width, a count), where overshooting the right edge keeps the value growing. */
  limit?: number;
}

/** The position `p`, 0 to 1, for the pointer at `clientX`. A mixed drag takes
 * the pointer's own position in the field; any other moves from the value at
 * the press, scaled by the modifier, clamped without re-basing at the ends. */
export function dragPosition(frame: DragFrame, clientX: number): number {
  const raw = frame.mixed
    ? (clientX - frame.left) / frame.width
    : frame.baseP + (frame.factor * (clientX - frame.baseX)) / frame.width;
  return Math.min(frame.limit ?? 1, Math.max(0, raw));
}

/** The position `p`, 0 to 1, a click at `x` px from the left edge of a field `width` px wide
 * sets (`specs/0047-transform-polish/` Part B, criterion 33): the pointer's own place on the
 * scale, clamped to the field. A width that is not positive or an `x` that is not a number is
 * the start of the scale. */
export function clickPosition(x: number, width: number): number {
  if (!(width > 0) || Number.isNaN(x)) {
    return 0;
  }
  return Math.min(1, Math.max(0, x / width));
}
