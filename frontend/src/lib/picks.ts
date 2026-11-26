/**
 * The four picks of a material test (`specs/0146-material-test-picks`): their stored names, words
 * and the texts built from them. Shared by the pick bar, the table, the pick lines and the pick
 * pick of an operation.
 */

/** The picks in display order; Best is the main choice, the others fall back on it. */
export const PICK_KINDS = ["best", "second_best", "fastest", "deepest"] as const;

/** One pick. */
export type PickKind = (typeof PICK_KINDS)[number];

/** The word shown for each pick. */
export const PICK_LABEL: Record<PickKind, string> = {
  best: "Best",
  second_best: "Second best",
  fastest: "Fastest",
  deepest: "Deepest",
};

/** The pick a stored text names, `null` for any other value. */
export function asPickKind(value: unknown): PickKind | null {
  return PICK_KINDS.find((kind) => kind === value) ?? null;
}

/** The tooltip and description of a pick the material has no set of (criterion 15). */
export function noPickText(kind: PickKind): string {
  return `No ${PICK_LABEL[kind]} set for this laser`;
}

/** The pick colour token of a pick; Second best reuses Best and is told apart by its dashed frame. */
export const PICK_COLOUR: Record<PickKind, string> = {
  best: "var(--pick-best)",
  second_best: "var(--pick-second)",
  fastest: "var(--pick-fastest)",
  deepest: "var(--pick-deepest)",
};

/** The words of a cell's picks for its name: ", Best, Fastest". */
export function kindsText(kinds: readonly PickKind[]): string {
  return kinds.map((kind) => `, ${PICK_LABEL[kind]}`).join("");
}

/** Where an arrow key moves in the 2 x 2 grid, `null` when it stays or the key is none. */
export function moveInGrid(index: number, key: string): number | null {
  const column = index % 2;
  switch (key) {
    case "ArrowLeft":
      return column === 1 ? index - 1 : null;
    case "ArrowRight":
      return column === 0 ? index + 1 : null;
    case "ArrowUp":
      return index >= 2 ? index - 2 : null;
    case "ArrowDown":
      return index < 2 ? index + 2 : null;
    case "Home":
      return 0;
    case "End":
      return PICK_KINDS.length - 1;
    default:
      return null;
  }
}

