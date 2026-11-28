// Which groups the Select bar shows for a selection (`specs/0159-ui-consistency`, Frage 433 B):
// a kind group only while every selected object is of its kind, and in the narrow window class
// the Align and Distribute groups only from two selected objects.

/** The groups that act on one kind of object, each with the object kinds it accepts. */
const GROUP_KINDS = {
  /** Radius, Link corners and Remove rounding. */
  rectangle: ["rectangle"],
  /** Points. */
  points: ["polygon", "star"],
  /** Ratio. */
  ratio: ["star"],
  /** The ellipse kind and the arc fields. */
  ellipse: ["ellipse"],
  /** Curve and Rounded. */
  curve: ["ellipse", "polygon", "star"],
  /** Randomized and Re-roll seed. */
  randomized: ["polygon", "star"],
  /** Font and Size, Alignment, Style, On path, the expand button. */
  text: ["text"],
  /** Object to path. */
  objectToPath: ["rectangle", "ellipse", "polygon", "star", "text", "sketch", "generated"],
  /** Box from path and Recognise text. */
  path: ["path"],
} as const satisfies Record<string, readonly string[]>;

export type KindGroup = keyof typeof GROUP_KINDS;

/** The bar's usable width (inside its side padding), px, below which the window is in the narrow
 * class: the width at which Scale, Flip and Guide, Tile, the two Align groups and Distribute with
 * its Gap no longer fit in one row (60 + 92 + 28 + 92 + 92 + 200 px and five 25 px gaps;
 * Proposal). An 800 px window with the panel open (324 px) is in it. */
export const NARROW_SELECT_BAR_PX = 690;

/** Reads the session's comma-separated kind codes (`selection_kinds`). */
export function readSelectionKinds(codes: string): string[] {
  return codes === "" ? [] : codes.split(",");
}

/** Whether `group` is in the bar for a selection of these kinds: something is selected and every
 * selected object is of a kind the group acts on. The group's own flag (does any object need it,
 * is it enabled) still comes from the session. */
export function kindGroupShown(group: KindGroup, kinds: readonly string[]): boolean {
  const accepted: readonly string[] = GROUP_KINDS[group];
  return kinds.length > 0 && kinds.every((kind) => accepted.includes(kind));
}

/** Whether the window is in the narrow class, for the bar's usable width in px. */
export function isNarrowSelectBar(usableWidthPx: number): boolean {
  return usableWidthPx < NARROW_SELECT_BAR_PX;
}

/** The arrange groups in the bar: Align and Distribute in every selection state, in the narrow
 * class only from two selected objects; Grid from two (`specs/0061-align-and-distribute/`
 * criterion 18). */
export function arrangeGroupsShown(
  selectedCount: number,
  narrow: boolean,
): { align: boolean; distribute: boolean; grid: boolean } {
  const many = selectedCount >= 2;
  const arrange = many || !narrow;
  return { align: arrange, distribute: arrange, grid: many };
}
