/** The space kept between the bar's edge line and a glyph, px. */
const CLEARANCE = 3;

/**
 * Whether the value field's bar edge at `edge` (px from the field's left) runs through one of the
 * `texts`, each `[left, right]` in px: the edge then breaks around the text row so it never cuts
 * a label or a value (`docs/design-system.md`, "Fill bar").
 */
export function edgeCrossesText(edge: number, texts: readonly (readonly [number, number])[]): boolean {
  return texts.some(([left, right]) => edge >= left - CLEARANCE && edge <= right + CLEARANCE);
}
