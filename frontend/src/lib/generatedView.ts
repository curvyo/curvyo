// The Generator block's data (`specs/0141-parametric-generators` ADR 0079 §2, UX notes §3 to §6):
// the decoder of the session's JSON and the small text rules of its fields. Pure functions, so the
// tests load it without a bundler. The rules (clamps, derived values, notices) are Rust's.

/** One option of a choice row. */
export interface GeneratedOption {
  id: string;
  label: string;
}

/** What a row holds, with its bounds (`curvyo-extension-core` `ParamKind`). */
export type GeneratedKind =
  | { kind: "length"; min: number; max: number; step: number; decimals: number }
  | { kind: "count"; min: number; max: number }
  | { kind: "angle"; min: number; max: number; step: number; decimals: number }
  | { kind: "switch" }
  | { kind: "choice"; options: GeneratedOption[] };

/** One row of the block with its stored value. */
export interface GeneratedRow {
  group: string;
  key: string;
  label: string;
  kind: GeneratedKind;
  /** A number for a length, count or angle; a boolean for a switch; an option id for a choice. */
  value: number | boolean | string;
}

/** The block of one selected generated object. */
export interface GeneratedView {
  /** The kind's name ("Gear"). */
  label: string;
  /** Whether the generator is available: an unknown kind shows no block. */
  known: boolean;
  rows: GeneratedRow[];
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
    rows?: { group: string; param: { key: string; label: string; kind: GeneratedKind }; value: { value: unknown } }[];
  };
  if (typeof view.label !== "string" || view.known !== true || !Array.isArray(view.rows)) {
    return null;
  }
  return {
    label: view.label,
    known: true,
    rows: view.rows.map((row) => ({
      group: row.group,
      key: row.param.key,
      label: row.param.label,
      kind: row.param.kind,
      value: row.value.value as number | boolean | string,
    })),
  };
}

/** The bar's kinds, from `generator_kinds_json`. */
export interface GeneratorKindEntry {
  /** `<extension>/<kind>`. */
  id: string;
  label: string;
  /** The Lucide glyph name. */
  icon: string;
}

/** The decoder of `generator_kinds_json`. */
export function decodeKinds(text: string): GeneratorKindEntry[] {
  try {
    const raw: unknown = JSON.parse(text);
    return Array.isArray(raw) ? (raw as GeneratorKindEntry[]) : [];
  } catch {
    return [];
  }
}

/** A number as the field shows it: at most `decimals` places, trailing zeros trimmed. */
export function shownNumber(value: number, decimals: number): string {
  const text = value.toFixed(decimals);
  return text.includes(".") ? text.replace(/0+$/, "").replace(/\.$/, "") : text;
}

/** The unit inside the right edge of a row: millimetres whatever the display unit, degrees, none
 * for a count. */
export function unitOf(kind: GeneratedKind): string | undefined {
  return kind.kind === "length" ? "mm" : kind.kind === "angle" ? "°" : undefined;
}

/** The text a number row shows for its stored value. */
export function rowText(row: GeneratedRow): string {
  const { kind, value } = row;
  if (typeof value !== "number") {
    return "";
  }
  return kind.kind === "length" || kind.kind === "angle" ? shownNumber(value, kind.decimals) : String(value);
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
  if (kind.kind === "length" || kind.kind === "count" || kind.kind === "angle") {
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
  const base = kind.kind === "count" ? 1 : kind.kind === "length" || kind.kind === "angle" ? kind.step : 1;
  const scale = shift ? 10 : ctrl && kind.kind !== "count" ? 0.1 : 1;
  const next = current + (key === "ArrowUp" ? 1 : -1) * base * scale;
  const decimals = kind.kind === "length" || kind.kind === "angle" ? kind.decimals : 0;
  return shownNumber(next, decimals);
}
