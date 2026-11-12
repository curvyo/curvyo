/**
 * The row logic of the Materials section (`specs/0028-material-test-library` `ux-notes.md`
 * section 10 and 13): accessible names, line 2, the armed texts, the order keys, the window above
 * 60 rows and the notice texts. Pure functions over the decoded view (`materialsView.ts`).
 */
import type { MaterialRow, LoadedMaterialRow } from "./materialsView.ts";
import { kindsText, thicknessText } from "./operationSettingsRows.ts";

/** A row is 40 px high (`0101` 4.2). */
export const MATERIAL_ROW_HEIGHT = 40;

/** Above this many rows the list is windowed (UX 15). */
export const MATERIAL_WINDOW_ABOVE = 60;

/** The longest material name a notice shows whole. */
export const NOTICE_NAME_MAX = 24;

/** A name cut with an ellipsis where a notice would run over 70 characters. */
export function shortName(name: string): string {
  const chars = Array.from(name);
  return chars.length <= NOTICE_NAME_MAX ? name : `${chars.slice(0, NOTICE_NAME_MAX - 1).join("")}…`;
}

/** The name a row shows on line 1. */
export function rowTitle(row: MaterialRow): string {
  switch (row.state) {
    case "loaded":
      return row.name;
    case "damaged":
      return "Damaged material";
    case "newer":
      return row.name ?? "Material";
  }
}

/** The row's accessible name (UX 10.2, 10.4, 14). */
export function materialRowName(row: MaterialRow): string {
  switch (row.state) {
    case "loaded":
      return `${row.name}, ${thicknessText(row.thicknessMm)}, ${kindsText(row.kinds)}`;
    case "damaged":
      return `Damaged material, ${row.reason}`;
    case "newer":
      return `${rowTitle(row)}, needs a newer Curvyo`;
  }
}

/** Line 2 without the symbols: "3.0 mm", or the reason, or "Needs a newer Curvyo". */
export function line2Text(row: MaterialRow): string {
  switch (row.state) {
    case "loaded":
      return thicknessText(row.thicknessMm);
    case "damaged":
      return row.reason;
    case "newer":
      return "Needs a newer Curvyo";
  }
}

/** Whether the row has a trash: a newer record is never touched. */
export function hasTrash(row: MaterialRow): boolean {
  return row.state !== "newer";
}

/** Whether a press on the row opens its fields. */
export function opensFields(row: MaterialRow): boolean {
  return row.state === "loaded";
}

/** The armed status of a material trash: "Press again to delete Birch 3 mm. 3 operations here use it." */
export function armText(row: MaterialRow): string {
  const name = rowTitle(row);
  const uses = row.state === "loaded" ? row.uses : 0;
  const base = `Press again to delete ${name}.`;
  if (uses === 0) return base;
  return `${base} ${uses === 1 ? "1 operation here uses it." : `${uses} operations here use it.`}`;
}

/** The armed status of a set trash. */
export function setArmText(kindWord: string, name: string): string {
  return `Press again to delete ${kindWord} set of ${name}.`;
}

/** The status after a delete. */
export function deletedText(name: string): string {
  return `${name} deleted.`;
}

/** What a key on a material row does (UX 10.2, 12). */
export type MaterialKeyAction = "move" | "rename" | "arm" | "remove" | "open" | "none";

/** The target index of a move key, `null` for a key that does not move. */
export function moveTarget(count: number, index: number, key: string): number | null {
  if (count < 2) return null;
  switch (key) {
    case "ArrowUp":
      return index > 0 ? index - 1 : null;
    case "ArrowDown":
      return index < count - 1 ? index + 1 : null;
    case "Home":
      return index > 0 ? 0 : null;
    case "End":
      return index < count - 1 ? count - 1 : null;
    default:
      return null;
  }
}

/** The key action on a row's name button: Alt+arrows move, F2 renames, Delete arms (and removes when armed), Enter opens. */
export function materialKeyAction(key: string, alt: boolean, armed: boolean): MaterialKeyAction {
  if (alt && (key === "ArrowUp" || key === "ArrowDown" || key === "Home" || key === "End")) return "move";
  if (key === "F2") return "rename";
  if (key === "Delete") return armed ? "remove" : "arm";
  if (key === "Enter") return armed ? "remove" : "open";
  return "none";
}

/** The height of the fields under a selected material: fixed, so the window is exact. */
export function fieldsHeight(sets: number, uses: number): number {
  return 32 + (uses > 0 ? 20 : 0) + sets * 104;
}

/** The window item heights: a row is 40 px, the selected one carries its fields. */
export function materialHeights(rows: readonly MaterialRow[], selectedId: string | null, fields: number): number[] {
  return rows.map((row) => MATERIAL_ROW_HEIGHT + (row.state === "loaded" && row.id === selectedId ? fields : 0));
}

/** The loaded rows only. */
export function loadedRows(rows: readonly MaterialRow[]): LoadedMaterialRow[] {
  return rows.filter((row): row is LoadedMaterialRow => row.state === "loaded");
}

/** The undo notice of a record that could not be written back (UX 13). */
export function undoFailureText(direction: "undo" | "redo", name: string | null, reason: string): string {
  return `Could not ${direction} ${name === null ? "the material" : shortName(name)}: ${reason}.`;
}

/** The notice of a failed save of a record (UX 10.3). */
export function saveFailureText(name: string | null, reason: string): string {
  return `Could not save ${name === null ? "the material" : shortName(name)}: ${reason}.`;
}

/** The "Used by 3 operations here" line, `null` at 0. */
export function usedByText(uses: number): string | null {
  if (uses === 0) return null;
  return uses === 1 ? "Used by 1 operation here" : `Used by ${uses} operations here`;
}

/** The arm target of a set's trash. */
export function setArmKey(id: string, kind: string): string {
  return `material-set:${id}:${kind}`;
}
