// The Print and cut views as the session sends them (`specs/0121-print-and-cut`): the
// Registration marks block of the Style tab and the Print and cut group of a laser page. Pure,
// so the tests load it without a bundler.

/** The Registration marks block (UX notes §3.3). */
export interface MarksView {
  count: number;
  /** The one entered mark's number, `null` at the object level. */
  mark: number | null;
  diameter: number;
  line: number;
  /** The lowest missing number for "Add mark n", `null` at 4 marks. */
  add: number | null;
}

/** One mark row of the group (UX notes §4.3). */
export interface MarkRowView {
  number: number;
  x: string | null;
  y: string | null;
  kind: "mark" | "gone" | "twice";
  note: string | null;
}

/** The fit block (UX notes §4.6). */
export interface FitView {
  moved: string;
  turned: string;
  scale: string;
  error: string;
  error_detail: string | null;
  warn: boolean;
}

/** The Print and cut group (UX notes §4.2). */
export interface PrintAndCutView {
  sheet: unknown;
  sheet_name: string;
  /** The Operations list's row of a marks object: "Registration marks (not cut)". */
  not_cut: string;
  sheets: { id: unknown; name: string; marks: number }[];
  marks_line: string;
  second_line: string | null;
  unreadable: string | null;
  summary: string;
  warn_summary: boolean;
  rows: MarkRowView[];
  state: string | null;
  fit: FitView | null;
  tolerance: number;
  show_tolerance: boolean;
  warnings: string[];
  apply: { label: string; reason: string | null } | null;
  not_usable: string | null;
  reset: { enabled: boolean; reason: string | null };
  placement_line: string | null;
}

/** One tag over the laser canvas (UX notes §5), in screen CSS px. */
export interface PrintAndCutLabel {
  kind: "mark" | "ring" | "fit";
  text: string;
  x: number;
  y: number;
}

/** The tags of `print_and_cut_labels_json`; nothing for malformed text. */
export function readPrintAndCutLabels(text: string): PrintAndCutLabel[] {
  const parsed = decodeJson<unknown>(text);
  if (!Array.isArray(parsed)) return [];
  return parsed.filter(
    (v): v is PrintAndCutLabel =>
      typeof v === "object" &&
      v !== null &&
      ["mark", "ring", "fit"].includes((v as PrintAndCutLabel).kind) &&
      typeof (v as PrintAndCutLabel).text === "string" &&
      typeof (v as PrintAndCutLabel).x === "number" &&
      typeof (v as PrintAndCutLabel).y === "number",
  );
}

/** The parsed JSON, or `null` when the session sent none. */
export function decodeJson<T>(text: string | undefined | null): T | null {
  if (!text) return null;
  try {
    return JSON.parse(text) as T;
  } catch {
    return null;
  }
}

/** The lowest and highest measured value in mm (UX notes P15). */
export const MEASURE_LIMIT_MM = 10000;

/**
 * A typed measure: a number with a decimal point or comma and an optional minus sign, within
 * ±10,000 mm; `null` for empty text, `undefined` for text that is not such a number.
 */
export function parseMeasure(text: string): number | null | undefined {
  const trimmed = text.trim();
  if (trimmed === "") return null;
  if (!/^-?\d*([.,]\d*)?$/.test(trimmed) || trimmed === "-" || trimmed === "." || trimmed === ",") {
    return undefined;
  }
  const value = Number(trimmed.replace(",", "."));
  if (!Number.isFinite(value) || Math.abs(value) > MEASURE_LIMIT_MM) return undefined;
  return value;
}

/** The field after `key` in the group's Enter order: X, Y of each mark row, then Tolerance, then
 * Apply (UX notes P14). `keys` lists the fields present, in order. */
export function nextField(keys: readonly string[], key: string): string | null {
  const at = keys.indexOf(key);
  return at < 0 || at + 1 >= keys.length ? null : keys[at + 1];
}

/** The mark whose Take head position gets focus after mark `taken` was read (UX notes P18): the
 * next row that is not measured, after `taken` and then from the top; `null` keeps the focus. */
export function nextUnmeasured(rows: readonly { number: number; x: string | null; kind: string }[], taken: number): number | null {
  const open = rows.filter((r) => r.kind === "mark" && r.x === null && r.number !== taken);
  return (open.find((r) => r.number > taken) ?? open[0])?.number ?? null;
}

/** Whether Take head position reads now (ADR 0081 §8). */
export type HeadPositionState = { enabled: true } | { enabled: false; cause: string };

/**
 * Whether Take head position reads now (spec criterion 8, ADR 0081 §8): in every state with a
 * position, except while the head moves (Running, Homing, a jog: "Machine is moving") and while
 * the controller names a state of its own (Door, Check with its simulated position, Sleep: as
 * Home does, `0118`).
 */
export function headPositionState(state: string | null, hasPosition: boolean, word: string | null = null): HeadPositionState {
  if (state === "Running" || state === "Homing" || word === "Jog") {
    return { enabled: false, cause: "Machine is moving" };
  }
  if (word !== null) {
    return { enabled: false, cause: `Controller state: ${word}` };
  }
  if (!hasPosition || state === null) {
    return { enabled: false, cause: "No position from the machine" };
  }
  return { enabled: true };
}

/** Whether the state line is "Marks too close to fit": 2 or more measured rows and no fit. */
export function stateIsWarning(view: Pick<PrintAndCutView, "state" | "rows">): boolean {
  const measured = view.rows.filter((r) => r.kind === "mark" && r.x !== null).length;
  return view.state !== null && measured >= 2;
}
