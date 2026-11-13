/**
 * The decoder of the Materials tab's JSON (`specs/0112-test-grid-and-materials` `adrs.md` finding
 * 3): `curvyo-editor-wasm/src/session/materials/view_json.rs` builds it and every field is checked
 * here at runtime. A value of the wrong shape, or with an enum text this build does not know,
 * decodes to `null` and the tab shows nothing rather than a guess. The Rust fixtures
 * `materials_view/*.json` pin both sides.
 */
import { asSetKind, isObject, type SetKind } from "./operationSettingsView.ts";

/** A laser as the tab names it: profile id and name. */
export interface Laser {
  id: string;
  name: string;
}

/** A readable material row. */
export interface LoadedMaterialRow {
  state: "loaded";
  id: string;
  name: string;
  thicknessMm: number;
  /** Line 2: "2 lasers · 3 sets · 4 tests", or "No sets". */
  line: string;
  tests: number;
}

/** A damaged record. */
export interface DamagedMaterialRow {
  state: "damaged";
  id: string;
  reason: string;
}

/** A record of a newer or older format, skipped in this build. */
export interface SkippedMaterialRow {
  state: "newer" | "older";
  id: string;
  name: string | null;
}

/** One row of the list. */
export type MaterialRow = LoadedMaterialRow | DamagedMaterialRow | SkippedMaterialRow;

/** The three numbers of a set. */
export interface SetNumbers {
  powerPct: number;
  speedMmMin: number;
  passes: number;
}

/** One set of a laser. */
export interface MaterialSet {
  kind: SetKind;
  numbers: SetNumbers;
  /** Engrave only. */
  intervalMm: number | null;
  /** "Test K7Q2M" or "Typed". */
  source: string;
}

/** One laser block of the unfolded material. */
export interface LaserBlock {
  laser: Laser;
  thisPage: boolean;
  open: boolean;
  sets: MaterialSet[];
  /** The kinds without a set, one "+ <kind> set" button each. */
  missing: SetKind[];
}

/** One test row. */
export interface TestRow {
  id: string;
  kind: SetKind;
  lasered: boolean;
  line: string;
  photo: boolean;
}

/** One axis of the judge table. */
export interface JudgeAxis {
  title: string;
  /** The unit after a value in a spoken text (" %", " mm/min", " ×", " mm"). */
  unit: string;
  values: string[];
}

/** A cell's place: column from the left, row from the top, both from 0. */
export interface CellPlace {
  column: number;
  row: number;
}

/** The place button of a test. */
export interface PlaceButton {
  onPage: boolean;
  label: string;
}

/** The open test. */
export interface TestDetail {
  id: string;
  kind: SetKind;
  head: string;
  stateLine: string;
  axesLine: string;
  columns: JudgeAxis;
  rows: JudgeAxis;
  best: CellPlace | null;
  setLine: string | null;
  note: string;
  /** The photo's file name, `null` without a photo. */
  photo: string | null;
  place: PlaceButton;
}

/** The unfolded material: its body, or its open test. */
export interface MaterialBody {
  id: string;
  name: string;
  thicknessMm: number;
  lasers: LaserBlock[];
  laserPickOpen: boolean;
  /** The library's profiles not yet in the material. */
  otherLasers: Laser[];
  tests: TestRow[];
  test: TestDetail | null;
}

/** The whole Materials tab of the active laser page. */
export interface MaterialsView {
  pageLaser: Laser | null;
  rows: MaterialRow[];
  unfolded: MaterialBody | null;
  formOpen: boolean;
  /** The default name of a new material, "Material 3". */
  nextName: string;
  loadNotices: string[];
}

function num(value: unknown): value is number {
  return typeof value === "number" && Number.isFinite(value);
}

function count(value: unknown): value is number {
  return Number.isSafeInteger(value) && (value as number) >= 0;
}

function strings(value: unknown): string[] | null {
  return Array.isArray(value) && value.every((one) => typeof one === "string") ? (value as string[]) : null;
}

function laser(value: unknown): Laser | null {
  return isObject(value) && typeof value.id === "string" && typeof value.name === "string"
    ? { id: value.id, name: value.name }
    : null;
}

function lasers(value: unknown): Laser[] | null {
  if (!Array.isArray(value)) return null;
  const out: Laser[] = [];
  for (const raw of value) {
    const one = laser(raw);
    if (one === null) return null;
    out.push(one);
  }
  return out;
}

function row(value: unknown): MaterialRow | null {
  if (!isObject(value) || typeof value.id !== "string") return null;
  const { id } = value;
  switch (value.state) {
    case "loaded":
      return typeof value.name === "string" && num(value.thickness_mm) && typeof value.line === "string" && count(value.tests)
        ? { state: "loaded", id, name: value.name, thicknessMm: value.thickness_mm, line: value.line, tests: value.tests }
        : null;
    case "damaged":
      return typeof value.reason === "string" ? { state: "damaged", id, reason: value.reason } : null;
    case "newer":
    case "older":
      return value.name === null || typeof value.name === "string"
        ? { state: value.state, id, name: value.name as string | null }
        : null;
    default:
      return null;
  }
}

function set(value: unknown): MaterialSet | null {
  if (!isObject(value) || !isObject(value.numbers)) return null;
  const kind = asSetKind(value.kind);
  const n = value.numbers;
  if (
    kind === null ||
    !num(n.power_pct) ||
    !count(n.speed_mm_min) ||
    !count(n.passes) ||
    (value.interval_mm !== null && !num(value.interval_mm)) ||
    typeof value.source !== "string"
  ) {
    return null;
  }
  return {
    kind,
    numbers: { powerPct: n.power_pct, speedMmMin: n.speed_mm_min, passes: n.passes },
    intervalMm: value.interval_mm as number | null,
    source: value.source,
  };
}

function block(value: unknown): LaserBlock | null {
  if (!isObject(value) || !Array.isArray(value.sets) || !Array.isArray(value.missing)) return null;
  const one = laser(value.laser);
  const sets = value.sets.map(set);
  const missing = value.missing.map(asSetKind);
  if (
    one === null ||
    typeof value.this_page !== "boolean" ||
    typeof value.open !== "boolean" ||
    sets.some((s) => s === null) ||
    missing.some((k) => k === null)
  ) {
    return null;
  }
  return { laser: one, thisPage: value.this_page, open: value.open, sets: sets as MaterialSet[], missing: missing as SetKind[] };
}

function testRow(value: unknown): TestRow | null {
  if (!isObject(value)) return null;
  const kind = asSetKind(value.kind);
  if (
    typeof value.id !== "string" ||
    kind === null ||
    typeof value.lasered !== "boolean" ||
    typeof value.line !== "string" ||
    typeof value.photo !== "boolean"
  ) {
    return null;
  }
  return { id: value.id, kind, lasered: value.lasered, line: value.line, photo: value.photo };
}

function axis(value: unknown): JudgeAxis | null {
  if (!isObject(value) || typeof value.title !== "string" || typeof value.unit !== "string") return null;
  const values = strings(value.values);
  return values === null ? null : { title: value.title, unit: value.unit, values };
}

function cell(value: unknown): CellPlace | null | undefined {
  if (value === null) return null;
  if (!isObject(value) || !count(value.column) || !count(value.row)) return undefined;
  return { column: value.column, row: value.row };
}

function detail(value: unknown): TestDetail | null | undefined {
  if (value === null) return null;
  if (!isObject(value)) return undefined;
  const kind = asSetKind(value.kind);
  const columns = axis(value.columns);
  const rows = axis(value.rows);
  const best = cell(value.best);
  const place = value.place;
  if (
    typeof value.id !== "string" ||
    kind === null ||
    typeof value.head !== "string" ||
    typeof value.state_line !== "string" ||
    typeof value.axes_line !== "string" ||
    columns === null ||
    rows === null ||
    best === undefined ||
    (value.set_line !== null && typeof value.set_line !== "string") ||
    typeof value.note !== "string" ||
    (value.photo !== null && typeof value.photo !== "string") ||
    !isObject(place) ||
    typeof place.on_page !== "boolean" ||
    typeof place.label !== "string"
  ) {
    return undefined;
  }
  return {
    id: value.id,
    kind,
    head: value.head,
    stateLine: value.state_line,
    axesLine: value.axes_line,
    columns,
    rows,
    best,
    setLine: value.set_line as string | null,
    note: value.note,
    photo: value.photo as string | null,
    place: { onPage: place.on_page, label: place.label },
  };
}

function body(value: unknown): MaterialBody | null | undefined {
  if (value === null) return null;
  if (!isObject(value) || !Array.isArray(value.lasers) || !Array.isArray(value.tests)) return undefined;
  const blocks = value.lasers.map(block);
  const others = lasers(value.other_lasers);
  const tests = value.tests.map(testRow);
  const open = detail(value.test);
  if (
    typeof value.id !== "string" ||
    typeof value.name !== "string" ||
    !num(value.thickness_mm) ||
    blocks.some((b) => b === null) ||
    typeof value.laser_pick_open !== "boolean" ||
    others === null ||
    tests.some((t) => t === null) ||
    open === undefined
  ) {
    return undefined;
  }
  return {
    id: value.id,
    name: value.name,
    thicknessMm: value.thickness_mm,
    lasers: blocks as LaserBlock[],
    laserPickOpen: value.laser_pick_open,
    otherLasers: others,
    tests: tests as TestRow[],
    test: open,
  };
}

/** The tab in `value` (already parsed JSON), or `null` (another page, or any value that is not the view). */
export function decodeMaterialsView(value: unknown): MaterialsView | null {
  if (!isObject(value) || !Array.isArray(value.rows)) return null;
  const page = value.page_laser === null ? null : laser(value.page_laser);
  const rows = value.rows.map(row);
  const notices = strings(value.load_notices);
  const unfolded = body(value.unfolded);
  if (
    (value.page_laser !== null && page === null) ||
    rows.some((r) => r === null) ||
    typeof value.form_open !== "boolean" ||
    typeof value.next_name !== "string" ||
    notices === null ||
    unfolded === undefined
  ) {
    return null;
  }
  return {
    pageLaser: page,
    rows: rows as MaterialRow[],
    unfolded,
    formOpen: value.form_open,
    nextName: value.next_name,
    loadNotices: notices,
  };
}

/** The tab a session's JSON text holds; `null` for `null`, bad JSON or another page. */
export function decodeMaterialsText(text: string): MaterialsView | null {
  try {
    return decodeMaterialsView(JSON.parse(text));
  } catch {
    return null;
  }
}
