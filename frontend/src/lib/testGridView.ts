/**
 * The decoder of the Test grid bar's JSON (`specs/0112-test-grid-and-materials` `adrs.md` finding 3):
 * `curvyo-editor-wasm/src/session/test_grid/view_json.rs` builds it and every field is checked here
 * at runtime. A value of the wrong shape, or with an enum text this build does not know, decodes to
 * `null` and the bar shows nothing rather than a guess. The Rust fixtures `test_grid_view/*.json`
 * pin both sides.
 */
import { asId, type OperationId } from "./operationsView.ts";
import { asSetKind, isObject, type SetKind } from "./operationSettingsView.ts";

/** The four parameters a grid varies or shares. */
export const PARAMS = ["power", "speed", "passes", "interval"] as const;

/** One parameter. */
export type Param = (typeof PARAMS)[number];

/** A material of the bar's dropdown; the thickness is `null` for the picked name alone. */
export interface BarMaterial {
  id: string;
  name: string;
  thicknessMm: number | null;
}

/** One value of row 1 (a parameter on no axis). */
export interface SharedValue {
  param: Param;
  value: number;
}

/** One axis unit of row 2. */
export interface AxisFields {
  param: Param;
  offered: Param[];
  min: number;
  max: number;
  count: number;
}

/** A typed value the grid did not take, kept in its field with its cause. */
export interface BarInvalid {
  field: string;
  typed: string;
  cause: string;
}

/** The bar of the selected grid, or of the tool's draft (`group` and `id` are `null`). */
export interface TestGridBarView {
  group: OperationId | null;
  id: string | null;
  lasered: boolean;
  kind: SetKind;
  material: BarMaterial | null;
  materials: BarMaterial[];
  shared: SharedValue[];
  /** `null` unless the kind is Engrave. */
  crosshatch: boolean | null;
  cellMm: number;
  columns: AxisFields;
  rows: AxisFields;
  widthMm: number;
  heightMm: number;
  maxFeedMmMin: number;
  invalid: BarInvalid | null;
  fit: string | null;
}

function num(value: unknown): value is number {
  return typeof value === "number" && Number.isFinite(value);
}

function asParam(value: unknown): Param | null {
  return PARAMS.find((param) => param === value) ?? null;
}

function material(value: unknown): BarMaterial | null | undefined {
  if (value === null) return null;
  if (!isObject(value) || typeof value.id !== "string" || typeof value.name !== "string") return undefined;
  if (value.thickness_mm !== null && !num(value.thickness_mm)) return undefined;
  return { id: value.id, name: value.name, thicknessMm: value.thickness_mm as number | null };
}

function materials(value: unknown): BarMaterial[] | null {
  if (!Array.isArray(value)) return null;
  const out: BarMaterial[] = [];
  for (const raw of value) {
    const one = material(raw);
    if (one === null || one === undefined) return null;
    out.push(one);
  }
  return out;
}

function shared(value: unknown): SharedValue[] | null {
  if (!Array.isArray(value)) return null;
  const out: SharedValue[] = [];
  for (const raw of value) {
    if (!isObject(raw)) return null;
    const param = asParam(raw.param);
    if (param === null || !num(raw.value)) return null;
    out.push({ param, value: raw.value });
  }
  return out;
}

function axis(value: unknown): AxisFields | null {
  if (!isObject(value) || !Array.isArray(value.offered)) return null;
  const param = asParam(value.param);
  const offered = value.offered.map(asParam);
  if (
    param === null ||
    offered.some((one) => one === null) ||
    !num(value.min) ||
    !num(value.max) ||
    !Number.isSafeInteger(value.count) ||
    (value.count as number) < 0
  ) {
    return null;
  }
  return { param, offered: offered as Param[], min: value.min, max: value.max, count: value.count as number };
}

function invalid(value: unknown): BarInvalid | null | undefined {
  if (value === null) return null;
  if (
    !isObject(value) ||
    typeof value.field !== "string" ||
    typeof value.typed !== "string" ||
    typeof value.cause !== "string"
  ) {
    return undefined;
  }
  return { field: value.field, typed: value.typed, cause: value.cause };
}

/** The bar in `value` (already parsed JSON), or `null` (no bar shows, or a value that is not the view). */
export function decodeTestGridView(value: unknown): TestGridBarView | null {
  if (!isObject(value)) return null;
  const group = value.group === null ? null : asId(value.group);
  const kind = asSetKind(value.kind);
  const picked = material(value.material);
  const list = materials(value.materials);
  const values = shared(value.shared);
  const columns = axis(value.columns);
  const rows = axis(value.rows);
  const bad = invalid(value.invalid);
  if (
    (value.group !== null && group === null) ||
    (value.id !== null && typeof value.id !== "string") ||
    typeof value.lasered !== "boolean" ||
    kind === null ||
    picked === undefined ||
    list === null ||
    values === null ||
    (value.crosshatch !== null && typeof value.crosshatch !== "boolean") ||
    !num(value.cell_mm) ||
    columns === null ||
    rows === null ||
    !num(value.width_mm) ||
    !num(value.height_mm) ||
    !Number.isSafeInteger(value.max_feed_mm_min) ||
    bad === undefined ||
    (value.fit !== null && typeof value.fit !== "string")
  ) {
    return null;
  }
  return {
    group,
    id: value.id as string | null,
    lasered: value.lasered,
    kind,
    material: picked,
    materials: list,
    shared: values,
    crosshatch: value.crosshatch as boolean | null,
    cellMm: value.cell_mm,
    columns,
    rows,
    widthMm: value.width_mm,
    heightMm: value.height_mm,
    maxFeedMmMin: value.max_feed_mm_min as number,
    invalid: bad,
    fit: value.fit as string | null,
  };
}

/** The bar a session's JSON text holds; `null` for `null`, bad JSON or a value that is not the view. */
export function decodeTestGridText(text: string): TestGridBarView | null {
  try {
    return decodeTestGridView(JSON.parse(text));
  } catch {
    return null;
  }
}
