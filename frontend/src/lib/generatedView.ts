// The Generator block's data (`specs/0141-parametric-generators` ADR 0080 §2, UX notes §3 to §6):
// the decoder of the session's JSON and the small text rules of its fields. Pure functions, so the
// tests load it without a bundler. The rules (clamps, derived values, notices) are Rust's.

/** One option of a choice row. */
export interface GeneratedOption {
  id: string;
  label: string;
}

/** What a row holds, with its bounds (`curvyo-extension-core` `ParamKind`). */
export type GeneratedKind =
  | { kind: "length"; min: number; max: number; step: number; decimals: number; automatic?: boolean }
  | { kind: "count"; min: number; max: number; automatic?: boolean }
  | { kind: "angle"; min: number; max: number; step: number; decimals: number }
  | { kind: "switch" }
  | { kind: "choice"; options: GeneratedOption[] }
  | { kind: "path" }
  | { kind: "text"; multiline: boolean }
  | { kind: "percent"; min: number; max: number }
  | { kind: "ratio"; min: number; max: number; step: number; decimals: number }
  | { kind: "font" };

/** The span a number's slider is drawn on. */
export interface GeneratedTrack {
  from: number;
  to: number;
  curve: "linear" | "log";
}

/** One row of the block with its stored value. */
export interface GeneratedRow {
  group: string;
  key: string;
  label: string;
  kind: GeneratedKind;
  /** The slider track (`Param.track`), `null` for the whole range, linear. */
  track: GeneratedTrack | null;
  /** The largest value a bound that depends on other values allows now, `null` when none. */
  limit: number | null;
  /** A word for a limit that is a shape ("full round"), reported by the generator. */
  state: string | null;
  /** For an automatic row stored as 0: the value the generator used, `null` otherwise. */
  used: number | null;
  /** Why the stored value cannot be drawn ("Enter a text to encode."), a lasting line under the
   * field while the object keeps its last geometry (`specs/0144` criterion 13); `null` when none. */
  problem: string | null;
  /** The schema default of the row, for the reset slot. */
  defaultValue: number | boolean | string;
  /** A number for a length, count, angle, percent or ratio; a boolean for a switch; an option id
   * for a choice; the text of a text row; the face name of a font row. */
  value: number | boolean | string;
}

/** The Symbology group of a family of kinds (`specs/0144` criterion 20, ADR 0092 §2). */
export interface GeneratedFamily {
  /** The group's heading ("Symbology"). */
  label: string;
  /** The stored kind the object has now. */
  current: string;
  /** The kinds of the family in manifest order. */
  kinds: { id: string; label: string }[];
}

/** One row of the Parts group (`specs/0147-rounded-box` criterion 15, UX notes §9). */
export interface GeneratedPart {
  /** The part's stable key (`lid`). */
  key: string;
  /** What the row calls it ("Lid"). */
  name: string;
  /** Whether the part has operations of its own. */
  detached: boolean;
  /** The names of the operations it runs with now ("Cut 3 mm"), empty when it has none. */
  operation: string;
  /** Detach is offered: the box runs with an operation to copy, or the part is detached. */
  canDetach: boolean;
}

/** The block of one selected generated object. */
export interface GeneratedView {
  /** The kind's name ("Gear"). */
  label: string;
  /** Whether the generator is available: an unknown kind shows no block. */
  known: boolean;
  rows: GeneratedRow[];
  /** The Parts group, empty for a kind with nothing to detach. */
  parts: GeneratedPart[];
  /** A code's fill is below 3 : 1 against the document background (`specs/0144` criterion 9). */
  contrastLow: boolean;
  /** The Symbology group, `null` for a kind without a family. */
  family: GeneratedFamily | null;
  /** The face the text is set in is not loaded yet: the rows wait (ADR 0092 §3.4). */
  loading: boolean;
  /** The face the text names is missing on this computer (the Font row says so). */
  fontMissing: boolean;
}

/** The decoder of `generated_panel_json`: `null` for no block. */
export function decodeGeneratedView(text: string): GeneratedView | null {
  let raw: unknown;
  try {
    raw = JSON.parse(text);
  } catch {
    return null;
  }
  if (typeof raw !== "object" || raw === null) {
    return null;
  }
  const view = raw as {
    label?: unknown;
    known?: unknown;
    contrast_low?: unknown;
    family?: GeneratedFamily | null;
    loading?: unknown;
    font_missing?: unknown;
    parts?: { key: string; name: string; detached?: boolean; operation?: string; can_detach?: boolean }[];
    rows?: {
      group: string;
      param: {
        key: string;
        label: string;
        kind: GeneratedKind;
        track?: GeneratedTrack | null;
        default?: { value: unknown };
      };
      value: { value: unknown };
      used?: { value: unknown } | null;
      state?: string | null;
      limit?: { value: unknown } | null;
      problem?: string | null;
    }[];
  };
  if (typeof view.label !== "string" || view.known !== true || !Array.isArray(view.rows)) {
    return null;
  }
  return {
    label: view.label,
    known: true,
    contrastLow: view.contrast_low === true,
    family: view.family ?? null,
    loading: view.loading === true,
    fontMissing: view.font_missing === true,
    parts: (view.parts ?? []).map((part) => ({
      key: part.key,
      name: part.name,
      detached: part.detached === true,
      operation: part.operation ?? "",
      canDetach: part.can_detach === true,
    })),
    rows: view.rows.map((row) => ({
      group: row.group,
      key: row.param.key,
      label: row.param.label,
      kind: row.param.kind,
      track: row.param.track ?? null,
      used: typeof row.used?.value === "number" ? row.used.value : null,
      state: typeof row.state === "string" ? row.state : null,
      limit: typeof row.limit?.value === "number" ? row.limit.value : null,
      problem: typeof row.problem === "string" ? row.problem : null,
      defaultValue: fontName(row.param.default?.value ?? row.value.value, row.param.kind),
      value: fontName(row.value.value, row.param.kind),
    })),
  };
}

/** A font value (`null` or the stored face) as the name a row shows; any other value unchanged. */
function fontName(value: unknown, kind: GeneratedKind): number | boolean | string {
  if (kind.kind !== "font") {
    return value as number | boolean | string;
  }
  const face = value as { family?: string; style?: string } | null;
  return face?.family ? `${face.family} ${face.style ?? ""}`.trim() : "Source Sans 3 Regular";
}

/** The bar's kinds, from `generator_kinds_json`. */
export interface GeneratorKindEntry {
  /** `<extension>/<kind>`. */
  id: string;
  /** The object's name ("Path box"). */
  label: string;
  /** What the segment shows: the command of a kind built from an outline ("Box from path"). */
  segment: string;
  /** Built from the selected outline, not placed at the pointer. */
  source: boolean;
  /** The Lucide glyph name. */
  icon: string;
  /** The family the entry stands for ("code"), `null` for a kind that stands alone. */
  family?: string | null;
  /** The kind it places waits for the included sans (ADR 0092 §3.4). */
  loading?: boolean;
}

/** The decoder of `generator_kinds_json`. */
export function decodeKinds(text: string): GeneratorKindEntry[] {
  try {
    const raw: unknown = JSON.parse(text);
    return Array.isArray(raw)
      ? (raw as GeneratorKindEntry[]).map((kind) => ({ ...kind, segment: kind.segment ?? kind.label }))
      : [];
  } catch {
    return [];
  }
}

/**
 * How the canvas notice slot shows a generator notice: a refusal (it ends "Nothing was changed.")
 * in the refusal look for 8 s, anything else (a warning about a built box, an opened file's
 * unknown generator) as a status for 5 s (`specs/0149-path-box` UX notes §1.2 and §6; design
 * system, "Action notice").
 */
export function canvasNoticeLook(text: string): { kind: "status" | "alert"; ms: number } {
  return text.endsWith("Nothing was changed.") ? { kind: "alert", ms: 8000 } : { kind: "status", ms: 5000 };
}

/** A number as the field shows it: at most `decimals` places, trailing zeros trimmed. */
export function shownNumber(value: number, decimals: number): string {
  const text = value.toFixed(decimals);
  return text.includes(".") ? text.replace(/0+$/, "").replace(/\.$/, "") : text;
}

/** The unit inside the right edge of a row: millimetres whatever the display unit, degrees, a
 * percent sign, none for a count. */
export function unitOf(kind: GeneratedKind): string | undefined {
  if (kind.kind === "ratio") {
    return ": 1";
  }
  return kind.kind === "length" ? "mm" : kind.kind === "angle" ? "°" : kind.kind === "percent" ? "%" : undefined;
}

/** The unit of a row: the Joint spread count is a percentage. */
export function unitOfRow(row: GeneratedRow): string {
  return row.key === "joint_spread" ? "%" : (unitOf(row.kind) ?? "");
}

/** The text a number row shows for its stored value. */
export function rowText(row: GeneratedRow): string {
  const { kind } = row;
  const value = row.used ?? row.value;
  if (typeof value !== "number") {
    return "";
  }
  if (kind.kind === "length" || kind.kind === "angle" || kind.kind === "ratio") {
    return shownNumber(value, kind.decimals);
  }
  // A signed percentage shows its plus: "+5", "0", "-5" (Bar width adjust).
  return kind.kind === "percent" && kind.min < 0 && value > 0 ? `+${value}` : String(value);
}

/** A typed text as a number: a decimal comma is a point; `null` for anything else. */
export function parseNumber(text: string): number | null {
  const trimmed = text.trim().replace(",", ".");
  if (trimmed === "" || !/^[-+]?(\d+\.?\d*|\.\d+)$/.test(trimmed)) {
    return null;
  }
  const number = Number(trimmed);
  return Number.isFinite(number) ? number : null;
}

/** The value and bounds of a number row for a spin button, `null` for a switch or a choice. */
export function rowRange(row: GeneratedRow): { now: number; min: number; max: number } | null {
  const { kind } = row;
  if (typeof row.value !== "number") {
    return null;
  }
  if (
    kind.kind === "length" ||
    kind.kind === "count" ||
    kind.kind === "angle" ||
    kind.kind === "percent" ||
    kind.kind === "ratio"
  ) {
    return { now: row.value, min: kind.min, max: kind.max };
  }
  return null;
}

/** An arrow key on a number row: the text stepped by the row's step (Shift ten times, Ctrl a
 * tenth; a count steps by 1 and ignores Ctrl). `null` for another key. */
export function stepRowText(
  row: GeneratedRow,
  text: string,
  key: string,
  shift: boolean,
  ctrl: boolean,
): string | null {
  const current = parseNumber(text) ?? (typeof row.value === "number" ? row.value : null);
  if (current === null || (key !== "ArrowUp" && key !== "ArrowDown")) {
    return null;
  }
  const { kind } = row;
  const base =
    kind.kind === "count" ? 1 : kind.kind === "length" || kind.kind === "angle" || kind.kind === "ratio" ? kind.step : 1;
  const scale = shift ? 10 : ctrl && kind.kind !== "count" ? 0.1 : 1;
  const next = current + (key === "ArrowUp" ? 1 : -1) * base * scale;
  const decimals =
    kind.kind === "length" || kind.kind === "angle" || kind.kind === "ratio" ? kind.decimals : 0;
  return shownNumber(next, decimals);
}

/** The grid names of the value field's modifiers. */
export type SliderGrid = "normal" | "coarse" | "fine";

/** The smallest and largest value a number row may hold. */
function bounds(row: GeneratedRow): { min: number; max: number } {
  const { kind } = row;
  return kind.kind === "length" ||
    kind.kind === "count" ||
    kind.kind === "angle" ||
    kind.kind === "percent" ||
    kind.kind === "ratio"
    ? { min: kind.min, max: kind.max }
    : { min: 0, max: 1 };
}

/** The track a number row is drawn on: its own, or its whole range, linear. */
function trackOf(row: GeneratedRow): GeneratedTrack {
  const { min, max } = bounds(row);
  return row.track ?? { from: min, to: max, curve: "linear" };
}

/** The position along the field, 0 to 1 on the track, of a value; beyond the ends for a value past
 * the track (the overshoot rule). Linear, or Width's curve `v = a + (b - a)(100^p - 1) / 99`. */
export function positionOf(row: GeneratedRow, value: number): number {
  const { from, to, curve } = trackOf(row);
  const t = (value - from) / (to - from);
  if (curve === "linear") {
    return t;
  }
  const inner = 1 + 99 * t;
  // Below the track's left end the curve reaches 1 - 1 / 99 of the span under it, then stops.
  return inner > 0 ? Math.log(inner) / Math.log(100) : -1;
}

/** The step of one grid: the row's step (a count steps by whole numbers), Shift ten times, Ctrl a
 * tenth, a count ignores Ctrl. */
function stepOf(row: GeneratedRow, grid: SliderGrid): number {
  const { kind } = row;
  if (kind.kind === "count") {
    return grid === "coarse" ? 10 : 1;
  }
  const base = kind.kind === "length" || kind.kind === "angle" || kind.kind === "ratio" ? kind.step : 1;
  return base * (grid === "coarse" ? 10 : grid === "fine" ? 0.1 : 1);
}

/** `value` rounded to the grid's step and held inside the row's bounds. */
function onGrid(row: GeneratedRow, value: number, grid: SliderGrid): number {
  const step = stepOf(row, grid);
  const { min, max } = bounds(row);
  const rounded = Math.round(value / step) * step;
  const decimals = Math.max(0, Math.ceil(-Math.log10(step)) + 1);
  return Math.min(max, Math.max(min, Number(rounded.toFixed(decimals))));
}

/** The value at position `p` of the field on `grid`. */
export function valueAt(row: GeneratedRow, p: number, grid: SliderGrid): number {
  const { from, to, curve } = trackOf(row);
  const t = curve === "linear" ? p : (100 ** p - 1) / 99;
  return onGrid(row, from + (to - from) * t, grid);
}

/** `steps` arrow-key steps from `value` on `grid`. */
export function stepValue(row: GeneratedRow, value: number, steps: number, grid: SliderGrid): number {
  const step = stepOf(row, grid);
  const { min, max } = bounds(row);
  const decimals = Math.max(0, Math.ceil(-Math.log10(step)) + 1);
  return Math.min(max, Math.max(min, Number((value + steps * step).toFixed(decimals))));
}

/** The positions a drag may reach: down to the row's minimum and up to its maximum. */
export function dragSpan(row: GeneratedRow): { min: number; limit: number } {
  const { min, max } = bounds(row);
  const { from } = trackOf(row);
  return {
    min: min < from ? Math.max(-1, positionOf(row, min)) : 0,
    limit: Math.max(1, positionOf(row, max)),
  };
}

/** The unit spelled out for `aria-valuetext`. */
export function unitWordsOf(row: GeneratedRow): string {
  if (row.key === "joint_spread") {
    return "percent";
  }
  if (row.kind.kind === "percent") {
    return "percent";
  }
  if (row.kind.kind === "ratio") {
    return "to one";
  }
  return row.kind.kind === "length" ? "millimetres" : row.kind.kind === "angle" ? "degrees" : row.label.toLowerCase();
}

/** The ending of `aria-valuetext` for a row the generator fills in: "automatic" for slit values,
 * "follows wall thickness" for a part's thickness; empty for a row with its own value. */
export function automaticWords(row: GeneratedRow): string {
  if (row.state !== null) {
    return `, ${row.state}`;
  }
  if (row.used === null) {
    return "";
  }
  return row.key.endsWith("_thickness") ? ", follows wall thickness" : ", automatic";
}

/** The end of the slider for `aria-valuemax`: the track's end, or the live limit when that is lower. */
export function sliderEnd(row: GeneratedRow): number {
  const end = row.track?.to ?? (rowRange(row)?.max ?? 0);
  return row.limit === null ? end : Math.min(end, row.limit);
}
