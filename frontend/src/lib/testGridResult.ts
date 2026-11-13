/**
 * The judge table of a test (`specs/0112-test-grid-and-materials` `ux-notes.md` 8.3): cell width,
 * names, the roving keys and the status of a choice. Pure functions; the component only draws
 * what they return.
 */
import type { CellPlace, JudgeAxis } from "./materialsView.ts";

/** Width of the row-header column; wider tables share 204 px. */
export const HEADER_WIDTH = 40;
export const TABLE_SHARE = 204;
export const CELL_ROW = 24;

/** A cell's width: 24 px up to 8 columns, then the 204 px shared (17 px at 12). */
export function cellWidth(columns: number): number {
  return columns <= 8 ? 24 : Math.floor(TABLE_SHARE / columns);
}

/** About the width of one 11 px tabular digit or point, px. */
const DIGIT_PX = 6.5;

/** Whether the header row shows only every other value: some label is wider than its cell. */
export function sparseHeader(values: readonly string[], width: number): boolean {
  return values.some((value) => value.length * DIGIT_PX > width);
}

/** A cell's accessible name: "60, 1950", with ", best cell" on the marked one. */
export function cellName(columns: JudgeAxis, rows: JudgeAxis, place: CellPlace, best: boolean): string {
  const column = columns.values[place.column] ?? "";
  const row = rows.values[place.row] ?? "";
  return `${column}, ${row}${best ? ", best cell" : ""}`;
}

/** The polite status of a choice with the units: "Best cell: 60 %, 1950 mm/min." */
export function bestStatus(columns: JudgeAxis, rows: JudgeAxis, place: CellPlace): string {
  return `Best cell: ${columns.values[place.column] ?? ""}${columns.unit}, ${rows.values[place.row] ?? ""}${rows.unit}.`;
}

/** The cell a key moves the roving focus to; `null` for a key that is no move. */
export function moveCell(at: CellPlace, key: string, ctrl: boolean, columns: number, rows: number): CellPlace | null {
  const last = { column: columns - 1, row: rows - 1 };
  const clamp = (value: number, max: number) => Math.min(max, Math.max(0, value));
  switch (key) {
    case "ArrowLeft":
      return { column: clamp(at.column - 1, last.column), row: at.row };
    case "ArrowRight":
      return { column: clamp(at.column + 1, last.column), row: at.row };
    case "ArrowUp":
      return { column: at.column, row: clamp(at.row - 1, last.row) };
    case "ArrowDown":
      return { column: at.column, row: clamp(at.row + 1, last.row) };
    case "Home":
      return ctrl ? { column: 0, row: 0 } : { column: 0, row: at.row };
    case "End":
      return ctrl ? last : { column: last.column, row: at.row };
    default:
      return null;
  }
}
