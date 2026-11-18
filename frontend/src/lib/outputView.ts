/**
 * The one decoder of the output group's, the play bar's and the ring labels' JSON
 * (`specs/0029-laser-job-preview-and-output`; UX notes sections 5 and 6): `curvyo-editor-wasm/src/session/output/view_json.rs` and
 * `play.rs` build it, and every field is checked here at runtime. No cast: a value of the wrong
 * shape decodes to `null`, and the tab shows nothing rather than a guess. The Rust contract test
 * reads the same fixtures (`curvyo-editor-wasm/tests/fixtures/output_view/`).
 */
import { asId, type OperationId } from "./operationsView.ts";

/** The three Start values, as the session stores them (`set_start`). */
export type StartValue = "machine_origin" | "head_position" | "home_then_origin";
const START_VALUES: readonly StartValue[] = ["machine_origin", "head_position", "home_then_origin"];

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
  | { kind: "items"; ids: OperationId[] }
  | { kind: "operations"; ids: OperationId[] }
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
  time: string;
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
  | "sending";
const SEND_CAUSES: readonly SendCause[] = [
  "blocked",
  "browser",
  "format_not_spoken",
  "not_paired",
  "no_longer_paired",
  "key_changed",
  "sending",
];

/** The Send to machine button, present only for a profile that names a connector (S1). */
export interface SendButton {
  enabled: boolean;
  cause: SendCause | null;
  /** The words of `blocked` (Export's text) and `format_not_spoken`. */
  text: string | null;
  profileId: string;
  address: string;
}

/** The output group at the end of the Operations tab of a laser page (UX notes 6.3). */
export interface OutputGroup {
  /** "Diode 10 W · GRBL[ · air assist]". */
  machine: string;
  start: StartView;
  checks: Check[];
  total: Total | null;
  export: ExportButton;
  /** `null` without a connector address on the profile. */
  send: SendButton | null;
  toolpathView: boolean;
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
  return { value: current, damaged: value.damaged, options };
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

function asTarget(value: unknown): CheckTarget | null {
  if (!isObject(value)) return null;
  switch (value.kind) {
    case "none":
      return { kind: "none" };
    case "fixture_pick":
      return { kind: "fixture_pick" };
    case "start_pick":
      return { kind: "start_pick" };
    case "items":
    case "operations": {
      const ids = asList(value.ids, asId);
      return ids === null ? null : { kind: value.kind, ids };
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
  if (typeof value.time !== "string" || runs === undefined || fixtures === undefined || typeof value.lengths !== "string") {
    return undefined;
  }
  return { time: value.time, runs, fixtures, lengths: value.lengths };
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
  return { enabled: value.enabled, cause, text, profileId: value.profile_id, address: value.address };
}

/** The output group from its JSON value; `null` for `null` (another page) or a broken value. */
export function decodeOutputGroup(value: unknown): OutputGroup | null {
  if (!isObject(value)) return null;
  const start = asStart(value.start);
  const checks = asList(value.checks, asCheck);
  const total = asTotal(value.total);
  const exportButton = asExport(value.export);
  const send = asSend(value.send);
  if (
    typeof value.machine !== "string" ||
    typeof value.toolpath_view !== "boolean" ||
    start === null ||
    checks === null ||
    total === undefined ||
    exportButton === null ||
    send === undefined
  ) {
    return null;
  }
  return { machine: value.machine, start, checks, total, export: exportButton, send, toolpathView: value.toolpath_view };
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
  return { fraction, playing: value.playing, readout: value.readout, enabled: value.enabled };
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
