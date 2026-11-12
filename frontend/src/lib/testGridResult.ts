/**
 * The result table and the Save as material form of the Grid block
 * (`specs/0028-material-test-library` `ux-notes.md` 7, 9): cell size, names, the roving keys, the
 * form's validation and its sentence. Pure functions; the components only draw what they return.
 */
import type { CellPlace } from "./testGridView.ts";

/** Width of the row-header column and of the 8-column table; wider tables share 204 px (7.1). */
export const HEADER_WIDTH = 40;
export const TABLE_SHARE = 204;
export const CELL_ROW = 24;

/** A cell's width: 24 px up to 8 columns, then the 204 px shared. */
export function cellWidth(columns: number): number {
  return columns <= 8 ? 24 : Math.floor(TABLE_SHARE / columns);
}

/** A power as the header row shows it: one decimal at most ("27.3"). */
export function powerLabel(power: number): string {
  return String(Math.round(power * 10) / 10);
}

/** About the width of one 11 px tabular digit or point, px. */
const DIGIT_PX = 6.5;

/** Whether the header row shows only every other power: some label is wider than its cell
 * (from 9 columns, ux-notes 7.1), so neighbouring labels would run into each other. */
export function sparseHeader(powers: readonly number[], width: number): boolean {
  return powers.some((power) => powerLabel(power).length * DIGIT_PX > width);
}

/** The polite status of a press in the table (ux-notes 13). */
export function markStatus(power: number, speed: number, marked: boolean): string {
  return marked ? `Cell marked: ${powerLabel(power)} % power, ${speed} mm/min.` : "Mark removed.";
}

/** A cell's accessible name: "60 % power, 3000 mm/min", with ", Worked" on the marked one. */
export function cellName(power: number, speed: number, marked: boolean): string {
  return `${Math.round(power * 10) / 10} % power, ${speed} mm/min${marked ? ", Worked" : ""}`;
}

/** The cell a key moves the roving focus to; `null` for a key that is no move (ux-notes 12). */
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

/** The Save as material form's draft. */
export interface SaveDraft {
  /** `""` for a new material, else the id of the existing one. */
  existing: string;
  name: string;
  thickness: string;
  passes: string;
}

/** The field a validation refused and its chip (ux-notes 9). */
export interface SaveRefusal {
  field: "name" | "thickness" | "passes";
  text: string;
}

/** The number a typed field names, `NaN` for none. */
function number(text: string): number {
  const trimmed = text.trim().replace(",", ".");
  return trimmed === "" ? Number.NaN : Number(trimmed);
}

/** The first invalid field of the draft, `null` when it may be saved. The session checks again. */
export function validateSave(draft: SaveDraft): SaveRefusal | null {
  if (draft.existing === "") {
    if (draft.name.trim() === "") return { field: "name", text: "Enter a name." };
    const thickness = number(draft.thickness);
    if (draft.thickness.trim() === "") return { field: "thickness", text: "Enter a thickness." };
    if (!(thickness >= 0.1 && thickness <= 100)) return { field: "thickness", text: "Thickness is 0.1 to 100 mm." };
  }
  const passes = number(draft.passes);
  if (!Number.isInteger(passes) || passes < 1 || passes > 20) return { field: "passes", text: "Passes are 1 to 20." };
  return null;
}

/** The thickness the session takes (`NaN` stays `NaN`; an existing material has none). */
export function thicknessOf(draft: SaveDraft): number {
  return draft.existing === "" ? number(draft.thickness) : 0;
}

/** The passes the session takes. */
export function passesOf(draft: SaveDraft): number {
  return number(draft.passes);
}

/** The "tested ..." part of a set's source line: "tested 2026-11-10". */
export function sourceWords(source: string): string {
  const head = source.split(",")[0] ?? source;
  return head.charAt(0).toLowerCase() + head.slice(1);
}

/** "Replaces the Cut set (60 %, 3000 mm/min, tested 2026-11-10)." (ux-notes 9, row 3). */
export function replacesSentence(kindWord: string, power: number, speed: number, source: string): string {
  return `Replaces the ${kindWord} set (${Math.round(power * 10) / 10} %, ${speed} mm/min, ${sourceWords(source)}).`;
}

/** The polite status after a save: "Birch 3 mm saved." (the material's name, ux-notes 9). */
export function savedStatus(name: string): string {
  return `${name} saved.`;
}
