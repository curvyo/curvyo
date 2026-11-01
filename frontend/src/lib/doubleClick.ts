/** When two canvas presses read as one double-click: no React, so a test can drive it. */

/** The longest time between the two presses of a double-click, ms. */
export const DOUBLE_CLICK_MS = 400;
/** Canvas-relative CSS pixels (`canvas-navigation-and-selection`: all screen↔document conversion
 * lives in Rust), not document millimetres — comparing in screen space is exactly right for "did
 * the two presses land close enough to read as one double-click", independent of zoom. */
export const DOUBLE_CLICK_PX = 5;
/** A held-back second press that moves this far before its release is a drag, px: the session's
 * click threshold, 8 px for a touch or a pen (`0052` criteria 1 and 31). */
const DRAG_PX = 3;
const TOUCH_DRAG_PX = 8;

/** A press the next one is paired with: when and where it happened. */
export interface PressMark {
  time: number;
  x: number;
  y: number;
}

/** Whether a press at `now`, (`x`, `y`) is the second press of a double-click after `last`.
 * Alt-clicks step through overlapping objects (`specs/0014-advanced-selection/` criterion 5),
 * quickly and at one point: never a double-click. */
export function isSecondPress(
  last: PressMark | null,
  now: number,
  x: number,
  y: number,
  alt: boolean,
): boolean {
  return (
    !alt &&
    last !== null &&
    now - last.time < DOUBLE_CLICK_MS &&
    Math.hypot(x - last.x, y - last.y) < DOUBLE_CLICK_PX
  );
}

/** Whether a held-back second press at (`press.x`, `press.y`) became a drag with the pointer at
 * (`x`, `y`): a press that moves beyond the drag threshold is a drag, never a double-click. */
export function withheldPressIsDrag(
  press: { x: number; y: number; touch: boolean },
  x: number,
  y: number,
): boolean {
  return Math.hypot(x - press.x, y - press.y) >= (press.touch ? TOUCH_DRAG_PX : DRAG_PX);
}
