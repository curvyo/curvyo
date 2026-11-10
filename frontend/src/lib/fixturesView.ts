/**
 * The one decoder of the Fixtures block's JSON (`specs/0102-laser-fixtures` `adrs.md` section 7):
 * `curvyo-editor-wasm/src/session/fixtures/view_json.rs` builds it, and every field is checked
 * here at runtime. No cast: a value of the wrong shape decodes to `null`, and the block shows
 * nothing rather than a guess. A kind this build does not know is `unknown` (criterion 32). The
 * Rust contract test reads the same fixtures (`curvyo-editor-wasm/tests/fixtures/fixtures_view/`).
 */

/** The five kinds a row can have, and `unknown` for a kind this build does not know. */
export type FixtureKind = "grid" | "holder" | "clamp" | "stop" | "screw" | "unknown";

/** A slot's shape as the session names it. */
export type SlotShapeName = "rect" | "circle";

/** The grid's tape switches, one per edge (criterion 15). */
export interface TapesValues {
  bottom: boolean;
  left: boolean;
  top: boolean;
  right: boolean;
}

/** The grid's fields in mm and lines (criterion 15). */
export interface GridValues {
  spacingMm: number;
  majorEvery: number;
  tapes: TapesValues;
  originXMm: number;
  originYMm: number;
}

/** One slot's row (criteria 19, 20). */
export interface SlotRow {
  id: string;
  name: string;
  shape: SlotShapeName;
  cxMm: number;
  cyMm: number;
  wMm: number;
  hMm: number;
  /** Whether it lies past the board's edge ("Outside the board"). */
  outside: boolean;
}

/** One fixture's row and its fields. */
export interface FixtureRow {
  /** The node id as its full id text: the calls take it back as it is. */
  id: string;
  name: string;
  kind: FixtureKind;
  active: boolean;
  visible: boolean;
  /** The grid's fields; holder, clamp, stop and screw have none: their handles size them. */
  grid: GridValues | null;
  slots: SlotRow[];
  canAddSlot: boolean;
}

/** The whole Fixtures block of the active laser page. */
export interface FixturesView {
  rows: FixtureRow[];
  canAddGrid: boolean;
  selected: string | null;
  selectedSlot: string | null;
  armed: string | null;
  damageNotices: string[];
  snappingFixtures: boolean;
  /** What a screen reader says of the selected fixture, with its sizes; `null` without one. */
  selectionAnnouncement: string | null;
}

type Json = Record<string, unknown>;

function isObject(value: unknown): value is Json {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function isNumber(value: unknown): value is number {
  return typeof value === "number" && Number.isFinite(value);
}

function asCount(value: unknown): number | null {
  return Number.isSafeInteger(value) && (value as number) >= 0 ? (value as number) : null;
}

function asOptionalText(value: unknown): string | null | undefined {
  if (value === null) return null;
  return typeof value === "string" ? value : undefined;
}

function asKind(value: unknown): FixtureKind {
  return value === "grid" ||
    value === "holder" ||
    value === "clamp" ||
    value === "stop" ||
    value === "screw"
    ? value
    : "unknown";
}

function asShape(value: unknown): SlotShapeName | null {
  return value === "rect" || value === "circle" ? value : null;
}

function asTapes(value: unknown): TapesValues | null {
  if (!isObject(value)) return null;
  const { bottom, left, top, right } = value;
  if (
    typeof bottom !== "boolean" ||
    typeof left !== "boolean" ||
    typeof top !== "boolean" ||
    typeof right !== "boolean"
  ) {
    return null;
  }
  return { bottom, left, top, right };
}

function asGrid(value: unknown): GridValues | null | undefined {
  if (value === null) return null;
  if (!isObject(value)) return undefined;
  const majorEvery = asCount(value.major_every);
  const tapes = asTapes(value.tapes);
  if (
    majorEvery === null ||
    tapes === null ||
    !isNumber(value.spacing_mm) ||
    !isNumber(value.origin_x_mm) ||
    !isNumber(value.origin_y_mm)
  ) {
    return undefined;
  }
  return {
    spacingMm: value.spacing_mm,
    majorEvery,
    tapes,
    originXMm: value.origin_x_mm,
    originYMm: value.origin_y_mm,
  };
}

function asSlot(value: unknown): SlotRow | null {
  if (!isObject(value)) return null;
  const shape = asShape(value.shape);
  if (
    shape === null ||
    typeof value.id !== "string" ||
    typeof value.name !== "string" ||
    !isNumber(value.cx_mm) ||
    !isNumber(value.cy_mm) ||
    !isNumber(value.w_mm) ||
    !isNumber(value.h_mm) ||
    typeof value.outside !== "boolean"
  ) {
    return null;
  }
  return {
    id: value.id,
    name: value.name,
    shape,
    cxMm: value.cx_mm,
    cyMm: value.cy_mm,
    wMm: value.w_mm,
    hMm: value.h_mm,
    outside: value.outside,
  };
}

function asRow(value: unknown): FixtureRow | null {
  if (!isObject(value)) return null;
  const grid = asGrid(value.grid);
  if (
    typeof value.id !== "string" ||
    typeof value.name !== "string" ||
    typeof value.active !== "boolean" ||
    typeof value.visible !== "boolean" ||
    typeof value.can_add_slot !== "boolean" ||
    grid === undefined ||
    !Array.isArray(value.slots)
  ) {
    return null;
  }
  const slots: SlotRow[] = [];
  for (const raw of value.slots) {
    const slot = asSlot(raw);
    if (slot === null) return null;
    slots.push(slot);
  }
  return {
    id: value.id,
    name: value.name,
    kind: asKind(value.kind),
    active: value.active,
    visible: value.visible,
    grid,
    slots,
    canAddSlot: value.can_add_slot,
  };
}

/**
 * The view in `value` (already parsed JSON), or `null`: `null` itself (a page that is not an
 * active laser page), or any value that is not the view.
 */
export function decodeFixturesView(value: unknown): FixturesView | null {
  if (!isObject(value) || !Array.isArray(value.rows)) return null;
  const rows: FixtureRow[] = [];
  for (const raw of value.rows) {
    const row = asRow(raw);
    if (row === null) return null;
    rows.push(row);
  }
  const selected = asOptionalText(value.selected);
  const selectedSlot = asOptionalText(value.selected_slot);
  const armed = asOptionalText(value.armed);
  const selectionAnnouncement =
    value.selection_announcement === undefined ? null : asOptionalText(value.selection_announcement);
  if (
    selectionAnnouncement === undefined ||
    typeof value.can_add_grid !== "boolean" ||
    selected === undefined ||
    selectedSlot === undefined ||
    armed === undefined ||
    typeof value.snapping_fixtures !== "boolean" ||
    !Array.isArray(value.damage_notices) ||
    !value.damage_notices.every((text) => typeof text === "string")
  ) {
    return null;
  }
  return {
    rows,
    canAddGrid: value.can_add_grid,
    selected,
    selectedSlot,
    armed,
    damageNotices: value.damage_notices as string[],
    snappingFixtures: value.snapping_fixtures,
    selectionAnnouncement,
  };
}

/** The view a JSON text holds; `null` for the text `null`, bad JSON or any other value. */
export function decodeFixturesText(text: string): FixturesView | null {
  try {
    return decodeFixturesView(JSON.parse(text));
  } catch {
    return null;
  }
}
