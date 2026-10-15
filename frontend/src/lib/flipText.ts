// The words of the two flip buttons and the keys H and V
// (`specs/0056-mirror-and-negative-scale/` criteria 1 to 4 and the UX notes). The flip itself
// and its no-op test are the session's; these are the fixed lines of the UI.

export type FlipAxis = "horizontal" | "vertical";

/** The accessible name, the tooltip's name line (with the key while the Proposal of criterion
 * 4 stands) and its rule line. */
export const FLIP_TEXT: Record<FlipAxis, { name: string; key: string; rule: string }> = {
  horizontal: {
    name: "Flip horizontally",
    key: "H",
    rule: "Mirror the selection left to right about the middle of its box.",
  },
  vertical: {
    name: "Flip vertically",
    key: "V",
    rule: "Mirror the selection top to bottom about the middle of its box.",
  },
};

/** The third, muted tooltip line while nothing is selected. */
export const FLIP_DISABLED_LINE = "Select an object first.";

/** The polite announcement after a flip (criterion 2). */
export const FLIP_ANNOUNCEMENT: Record<FlipAxis, string> = {
  horizontal: "Flipped horizontally",
  vertical: "Flipped vertically",
};

/** The notice of a flip that changes nothing (criterion 3). */
export const FLIP_UNCHANGED_NOTICE = "The selection looks the same flipped. Nothing changed.";

/** The notice and the zero-size notice stay 3 s (Action notice). */
export const FLIP_NOTICE_MS = 3000;

/** The axis a key outcome of H or V names, or `null` for every other outcome. */
export function flipKeyAxis(outcome: string): FlipAxis | null {
  if (outcome === "flip-horizontal") {
    return "horizontal";
  }
  if (outcome === "flip-vertical") {
    return "vertical";
  }
  return null;
}
