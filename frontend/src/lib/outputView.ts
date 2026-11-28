/**
 * The one decoder of the output group's, the play bar's and the ring labels' JSON
 * (`specs/0029-laser-job-preview-and-output`; UX notes sections 5 and 6): `curvyo-editor-wasm/src/session/output/view_json.rs` and
 * `play.rs` build it, and every field is checked here at runtime. No cast: a value of the wrong
 * shape decodes to `null`, and the tab shows nothing rather than a guess. The Rust contract test
 * reads the same fixtures (`curvyo-editor-wasm/tests/fixtures/output_view/`).
 */
import { asId, type OperationId } from "./operationsView.ts";

/** The three Start values, as the session stores them (`set_start`). */
export type StartValue = "machine_origin" | "head_position" | "home_then_origin" | "user_origin";
const START_VALUES: readonly StartValue[] = ["machine_origin", "head_position", "home_then_origin", "user_origin"];

/** One option of the Start pick. */
export interface StartOption {
  value: StartValue;
  label: string;
}

/** The Start pick (criterion 38): one value, three plain options, never greyed. */
export interface StartView {
  /** `null` while the stored value is damaged (criterion 4 e). */
  value: StartValue | null;
  /** The stored value is damaged: the job reads Machine origin, the pick shows "Damaged" and a
   * pick rewrites it. */
  damaged: boolean;
  options: StartOption[];
  /** The User origin row while User origin is picked, in bed mm (`specs/0118` W43); else `null`. */
  userOrigin: { x: number; y: number } | null;
}

/** One material of the fixture jobs' pick. */
export interface FixtureOption {
  id: string;
  name: string;
  pickable: boolean;
  /** "No Score set" when not pickable. */
  note: string | null;
}

/** The fixture jobs' material pick, shown while a fixture job runs (UX 6.2). */
export interface FixturePick {
  material: string | null;
  name: string | null;
  options: FixtureOption[];
}

/** What a check's button does. */
export type CheckTarget =
  | { kind: "none" }
  | {
      kind: "items";
      ids: OperationId[];
      /** The operation whose row unfolds too (`specs/0123` criterion 48), or `null`. */
      unfold: OperationId | null;
    }
  | {
      kind: "operations";
      ids: OperationId[];
      /** The unfolded row's field the line focuses (`dither`, `lowest_power`; `specs/0123`
       * UX notes 4), `null` for the row itself. */
      focus: "dither" | "lowest_power" | null;
    }
  | { kind: "fixture_pick" }
  | { kind: "start_pick" };

/** One check line (criteria 4, 5; UX notes 6.4): `before`, the button, `after`. */
export interface Check {
  severity: "block" | "warn";
  text: string;
  before: string;
  button: string;
  after: string;
  count: number;
  target: CheckTarget;
}

/** The total's fixture line (`specs/0103` criterion 17): the count is a button that selects the
 * fixtures of the running jobs. */
export interface FixtureLine {
  text: string;
  /** "1 fixture job". */
  button: string;
  /** " runs.". */
  after: string;
  ids: OperationId[];
}

/** The total's work line (`specs/0101` criterion 17): "Runs 3 operations, " then the button
 * "16 items" that selects the running items, then ".". */
export interface RunsLine {
  text: string;
  before: string;
  button: string;
  after: string;
}

/** The total (criterion 10). */
export interface Total {
  /** "About 9 min 51 s"; `null` where the format has no time (Design Space). */
  time: string | null;
  /** "Runs 3 operations, 16 items.", `null` when no work operation runs. */
  runs: RunsLine | null;
  /** "1 fixture job runs.", `null` when no fixture job runs. */
  fixtures: FixtureLine | null;
  lengths: string;
}

/** The Export button (criteria 1, 4, 27). */
export interface ExportButton {
  enabled: boolean;
  /** The first block's text while disabled. */
  tooltip: string | null;
  fileType: string;
  extension: string;
}

/** Why Send to machine is disabled (`0125` S2 to S5, S8; UX notes §3). */
export type SendCause =
  | "blocked"
  | "browser"
  | "format_not_spoken"
  | "not_paired"
  | "no_longer_paired"
  | "key_changed"
  | "sending"
  | "list_full";
const SEND_CAUSES: readonly SendCause[] = [
  "blocked",
  "browser",
  "format_not_spoken",
  "not_paired",
  "no_longer_paired",
  "key_changed",
  "sending",
  "list_full",
];

/** The Send to machine button, present only for a profile that names a connector (S1). */
export interface SendButton {
  enabled: boolean;
  cause: SendCause | null;
  /** The words of `blocked` (Export's text) and `format_not_spoken`. */
  text: string | null;
  profileId: string;
  address: string;
  /** A press also starts the job: the core's will-it-start rule over the last status (`0134`
   * SS9). */
  starts: boolean;
  /** Changes whenever the job a press would send changes (`0134` UX P15). */
  job: number;
}

/** The output group at the end of the Operations tab of a laser page (UX notes 6.3). */
export interface OutputGroup {
  /** "Diode 10 W · GRBL[ · air assist]". */
  machine: string;
  /** The Start pick; `null` on a family page, which has none (ADR 0098 decision 3). */
  start: StartView | null;
  checks: Check[];
  total: Total | null;
  export: ExportButton;
  /** `null` without a connector address on the profile. */
  send: SendButton | null;
  toolpathView: boolean;
  /** The Frame button (`specs/0118` 2b); `null` on a family page. */
  frame: FrameButton | null;
}

/** What the page says of the Frame button: whether it frames, and the cause if not. */
export interface FrameButton {
  enabled: boolean;
  cause: string | null;
}

/** The ring of a run in the Toolpath view: the row's digit, or `null` for the fixture glyph. */
export interface RingLabel {
  digit: number | null;
  x: number;
  y: number;
}

/** The play bar (criterion 37). */
export interface PlayView {
  /** 0 to 1 of the job's time. */
  fraction: number;
  playing: boolean;
  readout: string;
  enabled: boolean;
  /** The fraction one arrow press moves (one stitch on an embroidery page); `null` elsewhere. */
  step: number | null;
  /** The thread at the head, "#RRGGBB" (embroidery); `null` elsewhere. */
  colour: string | null;
}

type Json = Record<string, unknown>;

function isObject(value: unknown): value is Json {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function asCount(value: unknown): number | null {
  return Number.isSafeInteger(value) && (value as number) >= 0 ? (value as number) : null;
}

/** A text or `null`; `undefined` for anything else. */
function asOptionalText(value: unknown): string | null | undefined {
  if (value === null) return null;
  return typeof value === "string" ? value : undefined;
}

/** Each item decoded by `decode`; `null` when one fails or `value` is no array. */
function asList<T>(value: unknown, decode: (item: unknown) => T | null): T[] | null {
  if (!Array.isArray(value)) return null;
  const out: T[] = [];
  for (const item of value) {
    const decoded = decode(item);
    if (decoded === null) return null;
    out.push(decoded);
  }
  return out;
}

function asStart(value: unknown): StartView | null {
  if (!isObject(value) || typeof value.damaged !== "boolean") return null;
  const current = value.value === null ? null : START_VALUES.find((word) => word === value.value);
  const options = asList(value.options, (item) => {
    if (!isObject(item) || typeof item.label !== "string") return null;
    const word = START_VALUES.find((candidate) => candidate === item.value);
    return word === undefined ? null : { value: word, label: item.label };
  });
  if (current === undefined || options === null) return null;
  const origin = value.user_origin;
  const userOrigin =
    isObject(origin) && typeof origin.x === "number" && typeof origin.y === "number" ? { x: origin.x, y: origin.y } : null;
  return { value: current, damaged: value.damaged, options, userOrigin };
}

function asOption(value: unknown): FixtureOption | null {
  if (!isObject(value)) return null;
  const note = asOptionalText(value.note);
  if (typeof value.id !== "string" || typeof value.name !== "string" || typeof value.pickable !== "boolean" || note === undefined) {
    return null;
  }
  return { id: value.id, name: value.name, pickable: value.pickable, note };
}

/** The fixture pick of the Fixture jobs section: `null` for `null`, `undefined` for a broken value. */
export function asPick(value: unknown): FixturePick | null | undefined {
  if (value === null) return null;
  if (!isObject(value)) return undefined;
  const material = asOptionalText(value.material);
  const name = asOptionalText(value.name);
  const options = asList(value.options, asOption);
  if (material === undefined || name === undefined || options === null) return undefined;
  return { material, name, options };
}

export function asTarget(value: unknown): CheckTarget | null {
  if (!isObject(value)) return null;
  switch (value.kind) {
    case "none":
      return { kind: "none" };
    case "fixture_pick":
      return { kind: "fixture_pick" };
    case "start_pick":
      return { kind: "start_pick" };
    case "items": {
      const ids = asList(value.ids, asId);
      const unfold = value.unfold === null ? null : asId(value.unfold);
      if (ids === null || (value.unfold !== null && unfold === null)) return null;
      return { kind: "items", ids, unfold };
    }
    case "operations": {
      const ids = asList(value.ids, asId);
      const focus = value.focus;
      if (ids === null || (focus !== null && focus !== "dither" && focus !== "lowest_power")) return null;
      return { kind: "operations", ids, focus };
    }
    default:
      return null;
  }
}

function asCheck(value: unknown): Check | null {
  if (!isObject(value)) return null;
  const count = asCount(value.count);
  const target = asTarget(value.target);
  if (
    (value.severity !== "block" && value.severity !== "warn") ||
    typeof value.text !== "string" ||
    typeof value.before !== "string" ||
    typeof value.button !== "string" ||
    typeof value.after !== "string" ||
    count === null ||
    target === null
  ) {
    return null;
  }
  return {
    severity: value.severity,
    text: value.text,
    before: value.before,
    button: value.button,
    after: value.after,
    count,
    target,
  };
}

function asFixtureLine(value: unknown): FixtureLine | null | undefined {
  if (value === null) return null;
  if (!isObject(value) || typeof value.text !== "string" || typeof value.button !== "string" || typeof value.after !== "string") {
    return undefined;
  }
  const ids = asList(value.ids, asId);
  return ids === null ? undefined : { text: value.text, button: value.button, after: value.after, ids };
}

function asRunsLine(value: unknown): RunsLine | null | undefined {
  if (value === null) return null;
  if (
    !isObject(value) ||
    typeof value.text !== "string" ||
    typeof value.before !== "string" ||
    typeof value.button !== "string" ||
    typeof value.after !== "string"
  ) {
    return undefined;
  }
  return { text: value.text, before: value.before, button: value.button, after: value.after };
}

function asTotal(value: unknown): Total | null | undefined {
  if (value === null) return null;
  if (!isObject(value)) return undefined;
  const fixtures = asFixtureLine(value.fixtures);
  const runs = asRunsLine(value.runs);
  if ((typeof value.time !== "string" && value.time !== null) || runs === undefined || fixtures === undefined || typeof value.lengths !== "string") {
    return undefined;
  }
  return { time: value.time as string | null, runs, fixtures, lengths: value.lengths };
}

function asExport(value: unknown): ExportButton | null {
  if (!isObject(value)) return null;
  const tooltip = asOptionalText(value.tooltip);
  if (
    typeof value.enabled !== "boolean" ||
    tooltip === undefined ||
    typeof value.file_type !== "string" ||
    typeof value.extension !== "string"
  ) {
    return null;
  }
  return { enabled: value.enabled, tooltip, fileType: value.file_type, extension: value.extension };
}

function asSend(value: unknown): SendButton | null | undefined {
  if (value === undefined || value === null) return null;
  if (!isObject(value)) return undefined;
  const text = value.text === undefined ? null : asOptionalText(value.text);
  const cause = value.cause === undefined ? null : SEND_CAUSES.find((word) => word === value.cause);
  if (
    typeof value.enabled !== "boolean" ||
    text === undefined ||
    cause === undefined ||
    typeof value.profile_id !== "string" ||
    typeof value.address !== "string"
  ) {
    return undefined;
  }
  const starts = value.starts === undefined ? false : value.starts;
  const job = value.job === undefined ? 0 : value.job;
  if (typeof starts !== "boolean" || typeof job !== "number") return undefined;
  return { enabled: value.enabled, cause, text, profileId: value.profile_id, address: value.address, starts, job };
}

/** The Frame button of a group; `null` for a broken value. */
function asFrame(value: unknown): FrameButton | null {
  if (!isObject(value) || typeof value.enabled !== "boolean") return null;
  if (value.cause !== null && typeof value.cause !== "string") return null;
  return { enabled: value.enabled, cause: value.cause };
}

/** The output group from its JSON value; `null` for `null` (another page) or a broken value. */
export function decodeOutputGroup(value: unknown): OutputGroup | null {
  if (!isObject(value)) return null;
  // A family page has no Start pick and no Frame button: the keys are absent (ADR 0098).
  const start = value.start === undefined ? undefined : asStart(value.start);
  const checks = asList(value.checks, asCheck);
  const total = asTotal(value.total);
  const exportButton = asExport(value.export);
  const send = asSend(value.send);
  const frame = value.frame === undefined ? undefined : asFrame(value.frame);
  if (
    typeof value.machine !== "string" ||
    typeof value.toolpath_view !== "boolean" ||
    start === null ||
    checks === null ||
    total === undefined ||
    exportButton === null ||
    send === undefined ||
    frame === null
  ) {
    return null;
  }
  return {
    machine: value.machine,
    start: start ?? null,
    checks,
    total,
    export: exportButton,
    send,
    toolpathView: value.toolpath_view,
    frame: frame ?? null,
  };
}

/** The ring labels of the Toolpath view; `null` for a broken value. */
export function decodeRingLabels(value: unknown): RingLabel[] | null {
  return asList(value, (item) => {
    if (!isObject(item) || !Number.isFinite(item.x) || !Number.isFinite(item.y)) return null;
    const digit = item.digit === null ? null : asCount(item.digit);
    if (digit === undefined || (item.digit !== null && digit === null)) return null;
    return { digit, x: item.x as number, y: item.y as number };
  });
}

/** The play bar from its JSON value; `null` while the Toolpath view is not pressed or broken. */
export function decodePlayView(value: unknown): PlayView | null {
  if (!isObject(value)) return null;
  const { fraction } = value;
  if (
    typeof fraction !== "number" ||
    !(fraction >= 0 && fraction <= 1) ||
    typeof value.playing !== "boolean" ||
    typeof value.readout !== "string" ||
    typeof value.enabled !== "boolean"
  ) {
    return null;
  }
  const step = typeof value.step === "number" && value.step > 0 ? value.step : null;
  const colour = typeof value.colour === "string" ? value.colour : null;
  return { fraction, playing: value.playing, readout: value.readout, enabled: value.enabled, step, colour };
}

/** [`decodeOutputGroup`] of a JSON text. */
export function decodeOutputGroupText(text: string): OutputGroup | null {
  try {
    return decodeOutputGroup(JSON.parse(text));
  } catch {
    return null;
  }
}

/** [`decodeRingLabels`] of a JSON text. */
export function decodeRingLabelsText(text: string): RingLabel[] {
  try {
    return decodeRingLabels(JSON.parse(text)) ?? [];
  } catch {
    return [];
  }
}

/** [`decodePlayView`] of a JSON text. */
export function decodePlayViewText(text: string): PlayView | null {
  try {
    return decodePlayView(JSON.parse(text));
  } catch {
    return null;
  }
}

/** The user origin's marks in canvas CSS px (`specs/0118` W46, W47): the cross and the dashed
 * burn box `[x, y, width, height]`, each `null` while not shown. */
export interface UserOriginMarks {
  cross: [number, number] | null;
  box: [number, number, number, number] | null;
}

const NO_MARKS: UserOriginMarks = { cross: null, box: null };

function numbers(value: unknown, length: number): number[] | null {
  return Array.isArray(value) && value.length === length && value.every((n) => typeof n === "number") ? value : null;
}

/** `user_origin_marks_json()` decoded; unreadable text is no marks. */
export function decodeUserOriginMarks(text: string): UserOriginMarks {
  let value: unknown;
  try {
    value = JSON.parse(text);
  } catch {
    return NO_MARKS;
  }
  if (!isObject(value)) return NO_MARKS;
  const cross = numbers(value.cross, 2);
  const box = numbers(value.box, 4);
  return {
    cross: cross === null ? null : [cross[0], cross[1]],
    box: box === null ? null : [box[0], box[1], box[2], box[3]],
  };
}

/** The screen reader's words for a stored point (`specs/0118` W41). */
export function userOriginWords(x: number, y: number): string {
  return `User origin X ${x.toFixed(2)} mm, Y ${y.toFixed(2)} mm`;
}
