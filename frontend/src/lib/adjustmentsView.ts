/**
 * The one decoder of the Style tab's Adjustments group JSON (`specs/0135-image-optimisation`
 * criteria 1, 2, 9, 25 to 30, 59; `ux-notes.md` §3): `curvyo-editor-wasm/src/session/image_adjust/view.rs`
 * builds it, and every field is checked here at runtime. No cast: a value of the wrong shape
 * decodes to `null`, and the group shows nothing rather than a guess; `null` from the session is
 * the group absent (not exactly one image or image clone selected). The Rust contract test reads
 * the same fixtures (`curvyo-editor-wasm/tests/fixtures/adjustments_view/`).
 */
import { type DitherSettings, decodeDitherSettings, isObject } from "./operationSettingsView.ts";

/** The preset keys in strip order, Expert last (criterion 25). */
export const PRESET_KEYS = ["photo", "graphics", "black_white", "line_art", "dot_raster", "newspaper", "expert"] as const;

/** One preset key, as `pick_adjust_preset` takes it. */
export type PresetKey = (typeof PRESET_KEYS)[number];

/** The slice-1 step kinds, as `add_adjust_step` takes them. */
export const STEP_KINDS = ["denoise", "levels", "local_contrast", "contours", "sharpen"] as const;

/** One step kind, also its glyph's name. */
export type StepKindKey = (typeof STEP_KINDS)[number];

/** One cell of the Preset strip. */
export interface PresetCell {
  key: PresetKey;
  /** The visible cell text ("Dots"). */
  cell: string;
  /** The accessible name, the full name. */
  name: string;
  /** "<full name> · Dither: <mode>". */
  tooltip: string;
  pressed: boolean;
}

/** The Strength row (criterion 26): whole percent. */
export interface Strength {
  value: number;
  /** The refusal of a typed value out of range. */
  range: string;
}

/** One number field of a step row, in its shown unit. */
export interface StepField {
  /** The key `edit_adjust_field` takes. */
  key: string;
  label: string;
  /** "%", "mm", empty for Gamma. */
  unit: string;
  value: number;
  decimals: number;
  range: string;
}

/** One step row, manual or derived. */
export interface StepRow {
  /** The row's index in the stored stack; `null` for a derived row. */
  index: number | null;
  /** "unknown" for a step this build cannot run (unknown kind or damaged field). */
  kind: StepKindKey | "unknown";
  name: string;
  enabled: boolean;
  /** The preset runs instead of it (criterion 30). */
  presetInUse: boolean;
  /** It cannot run: trash only. */
  unknown: boolean;
  /** Line 2 folded. */
  line: string;
  /** Empty for an unknown or derived row. */
  fields: StepField[];
  up: boolean;
  down: boolean;
  accessibleName: string;
  /** A Levels row's histogram of its input (criterion 16): 256 bars, black to white, each 0 to
   * 255 of the tallest; `null` for other rows and an unreadable picture. */
  histogram: number[] | null;
}

/** The derived block of a preset (criterion 28). */
export interface Derived {
  /** "From preset Photo, Strength 50 %". */
  line: string;
  rows: StepRow[];
}

/** One button of the add toolbar. */
export interface AddButton {
  kind: StepKindKey;
  /** "Add Levels". */
  name: string;
}

/** The Adjustments group of the selected picture. */
export interface AdjustmentsView {
  /** Restore original is greyed, never hidden (criterion 59). */
  restoreEnabled: boolean;
  beforeAfterOpen: boolean;
  /** "Not engraved yet", "Numbers for Image 1, 0.10 mm" or `null`. */
  stateLine: string | null;
  /** The alert line when steps did not fit the memory of a run (criterion 56), or `null`. */
  memoryLine: string | null;
  presets: PresetCell[];
  /** The picture's own Dither strip and its mode's fields (`ux-notes.md` §3.10). */
  dither: DitherSettings;
  /** `null` with Expert (hidden, the value kept). */
  strength: Strength | null;
  /** `null` with Expert. */
  derived: Derived | null;
  /** Row 1 of the add toolbar; empty while a preset is active. */
  addTone: AddButton[];
  /** Row 2: none in slice 1. */
  addGeometry: AddButton[];
  steps: StepRow[];
  /** The clone's Adjustments override row; `null` for an image. */
  cloneRow: "original" | "own" | null;
  /** The notice of the last edit. */
  notice: string | null;
}

function finite(value: unknown): value is number {
  return typeof value === "number" && Number.isFinite(value);
}

function whole(value: unknown): value is number {
  return Number.isSafeInteger(value) && (value as number) >= 0;
}

function textOrNull(value: unknown): value is string | null {
  return value === null || typeof value === "string";
}

function oneOf<T extends string>(keys: readonly T[], value: unknown): value is T {
  return (keys as readonly unknown[]).includes(value);
}

/** Every element of `value` decoded by `decode`; `null` when one is wrong or it is no list. */
function list<T>(value: unknown, decode: (v: unknown) => T | null): T[] | null {
  if (!Array.isArray(value)) return null;
  const out: T[] = [];
  for (const raw of value) {
    const item = decode(raw);
    if (item === null) return null;
    out.push(item);
  }
  return out;
}

function asPresetCell(value: unknown): PresetCell | null {
  if (
    !isObject(value) ||
    !oneOf(PRESET_KEYS, value.key) ||
    typeof value.cell !== "string" ||
    typeof value.name !== "string" ||
    typeof value.tooltip !== "string" ||
    typeof value.pressed !== "boolean"
  ) {
    return null;
  }
  return { key: value.key, cell: value.cell, name: value.name, tooltip: value.tooltip, pressed: value.pressed };
}

function asStrength(value: unknown): Strength | null {
  if (!isObject(value) || !whole(value.value) || value.value > 100 || typeof value.range !== "string") return null;
  return { value: value.value, range: value.range };
}

function asStepField(value: unknown): StepField | null {
  if (
    !isObject(value) ||
    typeof value.key !== "string" ||
    typeof value.label !== "string" ||
    typeof value.unit !== "string" ||
    !finite(value.value) ||
    !whole(value.decimals) ||
    typeof value.range !== "string"
  ) {
    return null;
  }
  return {
    key: value.key,
    label: value.label,
    unit: value.unit,
    value: value.value,
    decimals: value.decimals,
    range: value.range,
  };
}

function asStepRow(value: unknown): StepRow | null {
  if (!isObject(value)) return null;
  const fields = list(value.fields, asStepField);
  const kind = value.kind;
  if (
    !(value.index === null || whole(value.index)) ||
    !(kind === "unknown" || oneOf(STEP_KINDS, kind)) ||
    typeof value.name !== "string" ||
    typeof value.enabled !== "boolean" ||
    typeof value.preset_in_use !== "boolean" ||
    typeof value.unknown !== "boolean" ||
    typeof value.line !== "string" ||
    fields === null ||
    typeof value.up !== "boolean" ||
    typeof value.down !== "boolean" ||
    typeof value.accessible_name !== "string" ||
    !(value.histogram === null || isHistogram(value.histogram))
  ) {
    return null;
  }
  return {
    index: value.index,
    kind,
    name: value.name,
    enabled: value.enabled,
    presetInUse: value.preset_in_use,
    unknown: value.unknown,
    line: value.line,
    fields,
    up: value.up,
    down: value.down,
    accessibleName: value.accessible_name,
    histogram: value.histogram,
  };
}

/** 256 bars of 0 to 255. */
function isHistogram(value: unknown): value is number[] {
  return (
    Array.isArray(value) &&
    value.length === 256 &&
    value.every((bar) => whole(bar) && (bar as number) <= 255)
  );
}

function asDerived(value: unknown): Derived | null {
  if (!isObject(value) || typeof value.line !== "string") return null;
  const rows = list(value.rows, asStepRow);
  return rows === null ? null : { line: value.line, rows };
}

function asAddButton(value: unknown): AddButton | null {
  if (!isObject(value) || !oneOf(STEP_KINDS, value.kind) || typeof value.name !== "string") return null;
  return { kind: value.kind, name: value.name };
}

/** The group from its parsed JSON; `null` when absent or of the wrong shape. */
export function decodeAdjustmentsView(value: unknown): AdjustmentsView | null {
  if (!isObject(value)) return null;
  const presets = list(value.presets, asPresetCell);
  const dither = decodeDitherSettings(value.dither);
  const strength = value.strength === null ? null : asStrength(value.strength);
  const derived = value.derived === null ? null : asDerived(value.derived);
  const addTone = list(value.add_tone, asAddButton);
  const addGeometry = list(value.add_geometry, asAddButton);
  const steps = list(value.steps, asStepRow);
  const cloneRow = value.clone_row;
  if (
    typeof value.restore_enabled !== "boolean" ||
    typeof value.before_after_open !== "boolean" ||
    !textOrNull(value.state_line) ||
    !textOrNull(value.memory_line) ||
    presets === null ||
    dither === undefined ||
    (value.strength !== null && strength === null) ||
    (value.derived !== null && derived === null) ||
    addTone === null ||
    addGeometry === null ||
    steps === null ||
    !(cloneRow === null || cloneRow === "original" || cloneRow === "own") ||
    !textOrNull(value.notice)
  ) {
    return null;
  }
  return {
    restoreEnabled: value.restore_enabled,
    beforeAfterOpen: value.before_after_open,
    stateLine: value.state_line,
    memoryLine: value.memory_line,
    presets,
    dither,
    strength,
    derived,
    addTone,
    addGeometry,
    steps,
    cloneRow,
    notice: value.notice,
  };
}

/** The group from `adjustments_view_json`; `null` when absent or of the wrong shape. */
export function decodeAdjustmentsText(text: string): AdjustmentsView | null {
  try {
    return decodeAdjustmentsView(JSON.parse(text));
  } catch {
    return null;
  }
}
