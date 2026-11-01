// The Rows, Columns and Gap fields of the Grid group
// (`specs/0061-align-and-distribute/` criterion 19 and the UX notes "Field
// behaviour"): parsing, the linked counts and the refusals. The placement
// is the session's; these are the field rules of the UI.

import { parseGap } from "./alignText.ts";

/** The refusal of a count that is not a whole number. */
export const GRID_WHOLE_NUMBER = "Enter a whole number";

/** The refusal of a count below one. */
export const GRID_TOO_SMALL = "Must be 1 or more.";

/** The refusal of two typed counts whose cells cannot hold every object. */
export function GRID_NOT_ENOUGH_CELLS(count: number): string {
  return `Not enough cells for ${count} ${count === 1 ? "object" : "objects"}.`;
}

/** The three fields as typed; an empty count is derived, an empty gap is 0. */
export interface GridFields {
  rows: string;
  columns: string;
  gap: string;
}

/** A refusal per field: the message to show, or "" to mark the field only. */
export type GridErrors = Partial<Record<keyof GridFields, string>>;

/** What a press with these fields does: the counts and the gap, or why not. */
export type GridPlan =
  | { ok: true; rows: number; columns: number; gapMm: number; derived: { rows: number; columns: number } }
  | { ok: false; errors: GridErrors };

/** The nearest-to-square grid for `count` objects: columns = ceil(sqrt(n)),
 * rows = ceil(n / columns). */
export function nearestSquareGrid(count: number): { rows: number; columns: number } {
  const n = Math.max(1, count);
  const columns = Math.ceil(Math.sqrt(n));
  return { rows: Math.ceil(n / columns), columns };
}

/** Reads a typed count: a whole number of at least one. */
export function parseCount(text: string): { ok: true; value: number } | { ok: false; message: string } {
  const cleaned = text.trim().replaceAll("−", "-");
  if (!/^-?\d+$/.test(cleaned)) {
    return { ok: false, message: GRID_WHOLE_NUMBER };
  }
  const value = Number(cleaned);
  if (value < 1) {
    return { ok: false, message: GRID_TOO_SMALL };
  }
  return { ok: true, value };
}

/** The counts and the gap a press with `fields` uses for `count` objects:
 * a typed count drives and the empty one holds all objects
 * (ceil(n / typed)); neither typed is the nearest-to-square grid. */
export function gridPlan(count: number, fields: GridFields): GridPlan {
  const errors: GridErrors = {};
  const rows = fields.rows.trim() === "" ? null : parseCount(fields.rows);
  const columns = fields.columns.trim() === "" ? null : parseCount(fields.columns);
  const gap = parseGap(fields.gap);
  if (rows && !rows.ok) {
    errors.rows = rows.message;
  }
  if (columns && !columns.ok) {
    errors.columns = columns.message;
  }
  if (!gap.ok) {
    errors.gap = gap.message;
  }
  if (Object.keys(errors).length > 0) {
    return { ok: false, errors };
  }
  const n = Math.max(1, count);
  const typedRows = rows?.ok ? rows.value : null;
  const typedColumns = columns?.ok ? columns.value : null;
  let shape = nearestSquareGrid(n);
  if (typedRows !== null && typedColumns !== null) {
    if (typedRows * typedColumns < n) {
      return { ok: false, errors: { columns: GRID_NOT_ENOUGH_CELLS(n), rows: "" } };
    }
    shape = { rows: typedRows, columns: typedColumns };
  } else if (typedColumns !== null) {
    shape = { rows: Math.ceil(n / typedColumns), columns: typedColumns };
  } else if (typedRows !== null) {
    shape = { rows: typedRows, columns: Math.ceil(n / typedRows) };
  }
  const gapMm = gap.ok && gap.mm !== null ? gap.mm : 0;
  return { ok: true, rows: shape.rows, columns: shape.columns, gapMm, derived: shape };
}
