/**
 * The unfolded operation row (`specs/0111-laser-operation-tools` criterion 17, `ux-notes.md`
 * 6.1 and 12): which rows can unfold, when the body shows, which press asks the session to toggle
 * the fold, and the height of the one tall window item. Pure functions over the decoded row; the
 * session holds the unfolded row, the list keeps the row selection.
 */
import { operationIdKey, type OperationRow } from "./operationsView.ts";
import type { PressMods } from "./rowSelection.ts";

/** One operation row: 40 px (`0101` ux-notes 4.2); 48 px with a coarse pointer, by CSS. */
export const ROW_HEIGHT = 40;

/** The body's height before it has been measured, px (a Power row, Speed, Passes, the pick, the
 * hairline and the lower part). */
export const UNFOLDED_BODY_ESTIMATE = 300;

/** Whether a row has a body: a known operation with settings and no test grid row. */
export function canUnfold(row: OperationRow): boolean {
  return row.kind !== "unknown" && row.grid === null && row.settings !== null;
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

/** The window item's height: the fixed row, plus the measured body (or its estimate) when open. */
export function entryHeight(open: boolean, measuredBody: number | null): number {
  return open ? ROW_HEIGHT + (measuredBody ?? UNFOLDED_BODY_ESTIMATE) : ROW_HEIGHT;
}
