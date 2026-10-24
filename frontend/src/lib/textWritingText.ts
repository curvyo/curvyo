// The words and keys of the Writing mode and Glyph orientation groups
// (`specs/0079-text-layout-controls` criteria 20, 21; UX notes §10.2). No React, so node tests can
// check them; every value comes from the session.

/** The three writing modes, in the group's order (the session's codes). */
export const WRITING_MODE_CODES = ["horizontal", "vertical-rl", "vertical-lr"] as const;
export type WritingModeCode = (typeof WRITING_MODE_CODES)[number];

/** The three glyph orientations, in the group's order. */
export const ORIENTATION_CODES = ["auto", "upright", "sideways"] as const;
export type OrientationCode = (typeof ORIENTATION_CODES)[number];

/** Each button's accessible name and tooltip (criterion 20). */
export const WRITING_MODE_NAMES: Record<WritingModeCode, string> = {
  horizontal: "Horizontal",
  "vertical-rl": "Vertical, right to left",
  "vertical-lr": "Vertical, left to right",
};

export const ORIENTATION_NAMES: Record<OrientationCode, string> = {
  auto: "Auto orientation",
  upright: "Upright",
  sideways: "Sideways",
};

/** The session's writing mode code, or `null` when the subject texts differ (""). */
export function parseWritingMode(code: string): WritingModeCode | null {
  return (WRITING_MODE_CODES as readonly string[]).includes(code) ? (code as WritingModeCode) : null;
}

/** The session's glyph orientation code, or `null` when the subject texts differ. */
export function parseOrientation(code: string): OrientationCode | null {
  return (ORIENTATION_CODES as readonly string[]).includes(code) ? (code as OrientationCode) : null;
}

/** The orientation buttons are greyed while every subject text is horizontal (criterion 21);
 * with mixed writing modes some are vertical, so they act. */
export function orientationGreyed(mode: WritingModeCode | null): boolean {
  return mode === "horizontal";
}

/** The tooltip's second line: "Only for vertical text" on a greyed button, "Mixed" while the
 * subject texts differ, else none. */
export function writingTooltipLine(pressed: string | null, greyed: boolean): string | null {
  if (greyed) {
    return "Only for vertical text";
  }
  return pressed ? null : "Mixed";
}

/** Where an arrow, Home or End moves focus in a group of `count` buttons: no wrap, focus only
 * (UX notes §1, the Alignment group's keys). */
export function choiceFocusMove(key: string, index: number, count: number): number | null {
  const last = count - 1;
  switch (key) {
    case "ArrowLeft":
    case "ArrowUp":
      return Math.max(0, index - 1);
    case "ArrowRight":
    case "ArrowDown":
      return Math.min(last, index + 1);
    case "Home":
      return 0;
    case "End":
      return last;
    default:
      return null;
  }
}
