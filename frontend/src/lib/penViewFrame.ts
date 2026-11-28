// The geometry of the pen window's frame in the main window (`specs/0158-pen-display` slice 2b
// criterion 6, UX 10.5): the corner brackets of a screen rectangle, pure.

/** The bracket legs, px. */
const LEG = 16;

/** The bracket path of `[left, top, right, bottom]`: four corners with 16 px legs, shorter on a
 * frame too small for them. */
export function bracketPath([x0, y0, x1, y1]: readonly number[]): string {
  const leg = Math.max(0, Math.min(LEG, (x1 - x0) / 2, (y1 - y0) / 2));
  return [
    `M${x0} ${y0 + leg}V${y0}H${x0 + leg}`,
    `M${x1 - leg} ${y0}H${x1}V${y0 + leg}`,
    `M${x1} ${y1 - leg}V${y1}H${x1 - leg}`,
    `M${x0 + leg} ${y1}H${x0}V${y1 - leg}`,
  ].join("");
}
