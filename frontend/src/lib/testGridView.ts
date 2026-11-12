/**
 * The decoder of the Grid block's JSON (`specs/0028-material-test-library` `adrs.md` section 6):
 * `curvyo-editor-wasm/src/session/test_grid/view_json.rs` builds it and every field is checked here
 * at runtime. A value of the wrong shape, or with an enum text this build does not know, decodes to
 * `null` and the block shows nothing rather than a guess. The Rust fixtures `test_grid_view/*.json`
 * pin both sides.
 */
import type { OperationId } from "./operationsView.ts";
import { asSetKind, isObject, type SetKind } from "./operationSettingsView.ts";

/** The cell fields of the block. */
export interface GridFields {
  columns: number;
  rows: number;
  powerFromPct: number;
  powerToPct: number;
  speedFromMmMin: number;
  speedToMmMin: number;
  cellMm: number;
  gapMm: number;
}

/** The label fields of the block. */
export interface GridLabels {
  on: boolean;
  kind: SetKind;
  heightMm: number;
  powerPct: number;
  speedMmMin: number;
  passes: number;
}

/** A value the grid did not take, kept in its field with its chip. */
export interface GridInvalid {
  field: string;
  typed: string;
  chip: string;
}

/** A cell's place: column from the left, row from the top, both from 0. */
export interface CellPlace {
  column: number;
  row: number;
}

/** The Worked line. */
export interface GridWorked {
  cell: CellPlace;
  powerPct: number;
  speedMmMin: number;
  text: string;
}

/** The Grid block of the grid the canvas selection or the pressed row names. */
export interface TestGridView {
  group: OperationId;
  /** Its row, `null` when it has none. */
  row: OperationId | null;
  name: string;
  machine: string;
  widthMm: number;
  heightMm: number;
  maxFeedMmMin: number;
  kind: SetKind;
  fields: GridFields;
  labels: GridLabels;
  invalid: GridInvalid | null;
  fit: string | null;
  handEdited: boolean;
  powers: number[];
  speeds: number[];
  worked: GridWorked | null;
  selectedCell: CellPlace | null;
}

function id(value: unknown): OperationId | null {
  return isObject(value) && typeof value.peer === "string" && Number.isSafeInteger(value.counter)
    ? { peer: value.peer, counter: value.counter as number }
    : null;
}

function num(value: unknown): value is number {
  return typeof value === "number" && Number.isFinite(value);
}

function whole(value: unknown): value is number {
  return Number.isSafeInteger(value) && (value as number) >= 0;
}

function place(value: unknown): CellPlace | null | undefined {
  if (value === null) return null;
  if (!isObject(value) || !whole(value.column) || !whole(value.row)) return undefined;
  return { column: value.column, row: value.row };
}

function fields(value: unknown): GridFields | null {
  if (!isObject(value)) return null;
  const v = value;
  if (
    !whole(v.columns) ||
    !whole(v.rows) ||
    !num(v.power_from_pct) ||
    !num(v.power_to_pct) ||
    !whole(v.speed_from_mm_min) ||
    !whole(v.speed_to_mm_min) ||
    !num(v.cell_mm) ||
    !num(v.gap_mm)
  ) {
    return null;
  }
  return {
    columns: v.columns,
    rows: v.rows,
    powerFromPct: v.power_from_pct,
    powerToPct: v.power_to_pct,
    speedFromMmMin: v.speed_from_mm_min,
    speedToMmMin: v.speed_to_mm_min,
    cellMm: v.cell_mm,
    gapMm: v.gap_mm,
  };
}

function labels(value: unknown): GridLabels | null {
  if (!isObject(value)) return null;
  const kind = asSetKind(value.kind);
  if (
    typeof value.on !== "boolean" ||
    kind === null ||
    !num(value.height_mm) ||
    !num(value.power_pct) ||
    !whole(value.speed_mm_min) ||
    !whole(value.passes)
  ) {
    return null;
  }
  return {
    on: value.on,
    kind,
    heightMm: value.height_mm,
    powerPct: value.power_pct,
    speedMmMin: value.speed_mm_min,
    passes: value.passes,
  };
}

function invalid(value: unknown): GridInvalid | null | undefined {
  if (value === null) return null;
  if (
    !isObject(value) ||
    typeof value.field !== "string" ||
    typeof value.typed !== "string" ||
    typeof value.chip !== "string"
  ) {
    return undefined;
  }
  return { field: value.field, typed: value.typed, chip: value.chip };
}

function worked(value: unknown): GridWorked | null | undefined {
  if (value === null) return null;
  if (!isObject(value)) return undefined;
  const cell = place(value.cell);
  if (!cell || !num(value.power_pct) || !whole(value.speed_mm_min) || typeof value.text !== "string") {
    return undefined;
  }
  return { cell, powerPct: value.power_pct, speedMmMin: value.speed_mm_min, text: value.text };
}

/** The block in `value` (already parsed JSON), or `null` (no grid shows, or a value that is not the view). */
export function decodeTestGridView(value: unknown): TestGridView | null {
  if (!isObject(value)) return null;
  const group = id(value.group);
  const row = value.row === null ? null : id(value.row);
  const kind = asSetKind(value.kind);
  const cells = fields(value.fields);
  const label = labels(value.labels);
  const bad = invalid(value.invalid);
  const selectedCell = place(value.selected_cell);
  const mark = worked(value.worked);
  const result = value.result;
  if (
    group === null ||
    (value.row !== null && row === null) ||
    typeof value.name !== "string" ||
    typeof value.machine !== "string" ||
    !num(value.width_mm) ||
    !num(value.height_mm) ||
    !whole(value.max_feed_mm_min) ||
    kind === null ||
    cells === null ||
    label === null ||
    bad === undefined ||
    (value.fit !== null && typeof value.fit !== "string") ||
    typeof value.hand_edited !== "boolean" ||
    selectedCell === undefined ||
    mark === undefined ||
    !isObject(result) ||
    !Array.isArray(result.powers) ||
    !result.powers.every(num) ||
    !Array.isArray(result.speeds) ||
    !result.speeds.every(whole)
  ) {
    return null;
  }
  return {
    group,
    row,
    name: value.name,
    machine: value.machine,
    widthMm: value.width_mm,
    heightMm: value.height_mm,
    maxFeedMmMin: value.max_feed_mm_min,
    kind,
    fields: cells,
    labels: label,
    invalid: bad,
    fit: value.fit as string | null,
    handEdited: value.hand_edited,
    powers: result.powers as number[],
    speeds: result.speeds as number[],
    worked: mark,
    selectedCell,
  };
}

/** The block a session's JSON text holds; `null` for `null`, bad JSON or a value that is not the view. */
export function decodeTestGridText(text: string): TestGridView | null {
  try {
    return decodeTestGridView(JSON.parse(text));
  } catch {
    return null;
  }
}

/** The add result of `press_add_test_grid`: the new row, or the refusal text. */
export type AddResult = { ok: true; row: OperationId } | { ok: false; text: string };

/** Decodes the add button's JSON answer; a malformed answer reads as a refusal with no text. */
export function decodeAddResult(text: string): AddResult {
  try {
    const value: unknown = JSON.parse(text);
    if (isObject(value) && value.ok === true) {
      const row = id(value.row);
      if (row !== null) return { ok: true, row };
    }
    if (isObject(value) && value.ok === false && typeof value.text === "string") {
      return { ok: false, text: value.text };
    }
  } catch {
    // falls through to the empty refusal
  }
  return { ok: false, text: "" };
}
