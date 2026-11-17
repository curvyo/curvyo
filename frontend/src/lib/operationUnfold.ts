/**
 * The unfolded operation row (`specs/0111-laser-operation-tools` criterion 17, `ux-notes.md`
 * 6.1 and 12): which rows can unfold, when the body shows, which press asks the session to toggle
 * the fold, and the height of the one tall window item. Pure functions over the decoded row; the
 * session holds the unfolded row, the list keeps the row selection.
 */
import { operationIdKey, type OperationRow } from "./operationsView.ts";
import type { PressMods } from "./rowSelection.ts";

/** One operation row: 56 px, three lines (`specs/0029` UX 6.1, P25); 64 px with a coarse pointer. */
export const ROW_HEIGHT = 56;
export const ROW_HEIGHT_COARSE = 64;
/** The fourth line of Engrave + Outline: "Outline …", 14 px. */
export const OUTLINE_LINE = 14;

/** The height of a folded row: Engrave + Outline 70 px (78 coarse), the others 56 (64). */
export function rowHeight(row: OperationRow, coarse: boolean): number {
  const base = coarse ? ROW_HEIGHT_COARSE : ROW_HEIGHT;
  return row.kind === "engrave_outline" ? base + OUTLINE_LINE : base;
}

/** The body's height before it has been measured, px (a Power row, Speed, Passes, the pick, the
 * hairline and the lower part). */
export const UNFOLDED_BODY_ESTIMATE = 300;

/** Whether a row has a body: a known operation with settings. */
export function canUnfold(row: OperationRow): boolean {
  return row.kind !== "unknown" && row.settings !== null;
}

/** Whether the row draws its body: the session holds it unfolded and it is the one selected row
 * (several selected rows unfold nothing, `0028` M10). */
export function bodyOpen(row: OperationRow, selectedKeys: readonly string[]): boolean {
  return (
    canUnfold(row) && row.unfolded && selectedKeys.length === 1 && selectedKeys[0] === operationIdKey(row.id)
  );
}

/**
 * Whether a press on the row calls the session's fold toggle. A plain press on a folded row
 * unfolds it (any other folds), on the open row folds it. A row the session still holds unfolded
 * but that is not the selected one needs no call: selecting it shows its body again. A Ctrl or
 * Shift press only extends the selection.
 */
export function pressUnfolds(row: OperationRow, selectedKeys: readonly string[], mods: PressMods): boolean {
  if (mods.ctrl || mods.shift || !canUnfold(row)) return false;
  if (!row.unfolded) return true;
  return bodyOpen(row, selectedKeys);
}

/** The window item's height: the fixed row (`base`), plus the measured body (or its estimate) when open. */
export function entryHeight(open: boolean, measuredBody: number | null, base: number = ROW_HEIGHT): number {
  return open ? base + (measuredBody ?? UNFOLDED_BODY_ESTIMATE) : base;
}
