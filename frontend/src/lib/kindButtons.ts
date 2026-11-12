/**
 * The words of the Select bar's kind buttons (`specs/0111-laser-operation-tools` criterion 2,
 * `ux-notes.md` 3.2): the kind word is the name and the whole tooltip; the session sends the
 * description of what a press does. Pure functions.
 */
import { KIND_WORDS } from "./operationRows.ts";
import type { OperationKind } from "./operationsView.ts";

/** The accessible name and tooltip of a kind button: the kind word, "Engrave + Outline". */
export function kindButtonName(kind: Exclude<OperationKind, "unknown">): string {
  return KIND_WORDS[kind];
}

/**
 * Whether the Select bar draws anything for `kinds` inside an entered group: the bar holds only
 * the Operation group there (`ux-notes.md` 4.2), and with no member selected nothing at all.
 */
export function enteredBarShown(kinds: readonly unknown[] | null): boolean {
  return kinds !== null && kinds.length > 0;
}

/**
 * Whether a key press on a kind button leaves the entered group: Escape inside one
 * (`specs/0111-laser-operation-tools` criterion 11). The focus stays on a pressed button
 * (`ux-notes.md` 3.2), where the canvas does not see the key, so the bar routes it.
 */
export function leavesEnteredGroup(key: string, entered: boolean): boolean {
  return entered && key === "Escape";
}
