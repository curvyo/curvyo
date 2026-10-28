/**
 * The words and the plain rules of the Trace tool (`specs/0025-raster-trace` criteria 1, 3, 4, 5,
 * 9, 11, 13, 15, 16 and 46; UX notes sections 2, 3, 5 and 7): the tooltips, the empty states,
 * the status line, the notices, the field steps and the remembered values. No React and no wasm,
 * so node tests can check them. Rust sends codes and counts; the sentences are written here.
 */

/** The Select bar button's label. */
export const TRACE_BUTTON = "Trace";

/** The Select bar button's tooltip: its name and key only (criterion 1). */
export const TRACE_TOOLTIP = "Trace tool (I)";

/** The state label that opens the Trace bar (UX notes section 1). */
export const TRACE_STATE_LABEL = "Trace";

/** The Create paths button and its tooltip (criterion 12). */
export const CREATE_PATHS = "Create paths";
export const CREATE_PATHS_TOOLTIP = "Create paths (Enter)";

/** The bar with nothing selected (criterion 3). */
export const TRACE_EMPTY = "Click an image to trace it.";
export const TRACE_IMPORT = "Import image...";

/** The bar with something else selected (criterion 3). */
export const TRACE_NOT_IMAGE = "The selection is not an image. Click an image to trace it.";

/** The status while a job runs past 400 ms (criterion 10). */
export const TRACING = "Tracing...";

/** The status while the picture is not read yet (UX notes section 7). */
export const LOADING_IMAGE = "Loading image...";

/** The muted line under a stopped preview (criterion 53, Stop of criterion 10). */
export const PREVIEW_OUT_OF_DATE = "Preview is out of date";

/** The Stop button (criterion 10). */
export const STOP = "Stop";

/** A region with 65 000 holes or more (criterion 16). */
export const TOO_DETAILED = "Too much detail: raise Specks.";

/** The Preset button's text after an edit (criterion 40). */
export const CUSTOM = "Custom";

/** The Advanced button (criterion 41). */
export const ADVANCED = "Advanced";

/** The Full-size preview switch's state text over 4 Mpx (UX notes section 12). */
export const OVER_4_MPX = "Over 4 Mpx";

/** Full-size preview is honoured up to this many pixels (UX notes section 12). */
export const FULL_SIZE_PREVIEW_MAX_PX = 4_000_000;

/** The values of a trace. `threshold` is `null` until the image's automatic value is known;
 * `upscale` and `denoise` are choice indices (Auto | Off, Off | Low | High); `nodeLimit` is `null`
 * for none. */
export interface TraceValues {
  threshold: number | null;
  smooth: number;
  specks: number;
  corners: number;
  upscale: number;
  denoise: number;
  nodeLimit: number | null;
  invert: boolean;
  ignoreBackground: boolean;
  fullSizePreview: boolean;
}

/** The values a preset sets (criterion 60). */
export type PresetValues = Pick<TraceValues, "smooth" | "specks" | "corners" | "upscale" | "denoise">;

/** The names of the preset values: an edit of one of them makes the choice "Custom". */
export const PRESET_FIELDS: readonly (keyof PresetValues)[] = ["smooth", "specks", "corners", "upscale", "denoise"];

/** One preset (`trace_presets()` of the wasm module, criteria 59 and 60). */
export interface TracePreset {
  key: string;
  label: string;
  detail: string;
  values: PresetValues;
}

/** The presets as the crate states them (the wasm module's `trace_presets()` is the source; this
 * copy is what the bar uses before the module is loaded and what the tests compare). */
export const TRACE_PRESETS_FALLBACK: readonly TracePreset[] = [
  { key: "logo", label: "Logo / Line art", detail: "Sharp corners, clean curves.", values: { smooth: 80, specks: 0.5, corners: 70, upscale: 1, denoise: 0 } },
  { key: "photo", label: "Photo", detail: "Many shapes. Photos trace roughly.", values: { smooth: 50, specks: 0.1, corners: 30, upscale: 1, denoise: 0 } },
  { key: "silhouette", label: "Silhouette for cutting/carving", detail: "Closed, simple, few nodes.", values: { smooth: 100, specks: 2, corners: 70, upscale: 1, denoise: 1 } },
  { key: "engraving", label: "Engraving", detail: "Keeps fine lines.", values: { smooth: 80, specks: 0.1, corners: 70, upscale: 0, denoise: 0 } },
];

/** The remembered choice (criteria 40 and 66, `adrs.md` 2026-10-28 second note point 6): the
 * preset the values came from, whether they were edited since, and the values. No value
 * comparison: a pick clears `edited`, any edit of a preset value sets it. */
export interface TraceChoice {
  preset: string;
  edited: boolean;
  values: TraceValues;
}

/** The name of a setting in the schema. */
export type TraceParamName =
  | "threshold"
  | "smooth"
  | "specks"
  | "denoise"
  | "upscale"
  | "corners"
  | "node_limit"
  | "invert"
  | "ignore_background"
  | "full_size_preview";

/** One field of the settings schema (`trace_params()` of the wasm module, criteria 6 and 45). */
export interface TraceParam {
  name: TraceParamName;
  label: string;
  kind: "integer" | "length" | "choice" | "toggle";
  unit: string;
  min: number;
  max: number;
  default: number;
  step: number;
  big_step: number;
  fine_step: number;
  group: string;
  advanced: boolean;
  /** A choice's labels in index order; empty otherwise. */
  choices: readonly string[];
}

/** The schema of slice 1 as the crate states it (the wasm module's `trace_params()` is the source;
 * this copy is what the bar uses before the module is loaded and what the tests compare). */
export const TRACE_PARAMS_FALLBACK: readonly TraceParam[] = [
  { name: "threshold", label: "Threshold", kind: "integer", unit: "", min: 0, max: 255, default: 128, step: 1, big_step: 10, fine_step: 1, group: "threshold", advanced: false, choices: [] },
  { name: "smooth", label: "Smooth", kind: "integer", unit: "", min: 0, max: 100, default: 80, step: 1, big_step: 10, fine_step: 1, group: "smooth", advanced: false, choices: [] },
  { name: "specks", label: "Ignore below", kind: "length", unit: "mm", min: 0, max: 10, default: 0.5, step: 0.1, big_step: 1, fine_step: 0.01, group: "specks", advanced: false, choices: [] },
  { name: "denoise", label: "Denoise", kind: "choice", unit: "", min: 0, max: 2, default: 0, step: 1, big_step: 1, fine_step: 1, group: "advanced", advanced: true, choices: ["Off", "Low", "High"] },
  { name: "upscale", label: "Upscale", kind: "choice", unit: "", min: 0, max: 1, default: 1, step: 1, big_step: 1, fine_step: 1, group: "advanced", advanced: true, choices: ["Auto", "Off"] },
  { name: "corners", label: "Corners", kind: "integer", unit: "", min: 0, max: 100, default: 70, step: 1, big_step: 10, fine_step: 1, group: "advanced", advanced: true, choices: [] },
  { name: "node_limit", label: "Node limit", kind: "integer", unit: "", min: 0, max: 10_000_000, default: 0, step: 100, big_step: 1000, fine_step: 1, group: "advanced", advanced: true, choices: [] },
  { name: "invert", label: "Invert", kind: "toggle", unit: "", min: 0, max: 1, default: 0, step: 1, big_step: 1, fine_step: 1, group: "advanced", advanced: true, choices: [] },
  { name: "ignore_background", label: "Ignore background", kind: "toggle", unit: "", min: 0, max: 1, default: 0, step: 1, big_step: 1, fine_step: 1, group: "advanced", advanced: true, choices: [] },
  { name: "full_size_preview", label: "Full-size preview", kind: "toggle", unit: "", min: 0, max: 1, default: 0, step: 1, big_step: 1, fine_step: 1, group: "advanced", advanced: true, choices: [] },
];

/** `value` clamped to the field's range and rounded to its finest step (criterion 5). */
export function clampValue(param: TraceParam, value: number): number {
  if (!Number.isFinite(value)) {
    return param.default;
  }
  const clamped = Math.min(param.max, Math.max(param.min, value));
  return param.kind === "integer" ? Math.round(clamped) : Math.round(clamped * 100) / 100;
}

/** A key of the field (criterion 5): Up and Down step, Shift steps big, Ctrl steps fine, Home and
 * End go to the limits. `null` for any other key. */
export function stepValue(
  param: TraceParam,
  value: number,
  key: { key: string; shiftKey: boolean; ctrlKey: boolean; metaKey: boolean },
): number | null {
  if (key.key === "Home") {
    return param.min;
  }
  if (key.key === "End") {
    return param.max;
  }
  const direction = key.key === "ArrowUp" ? 1 : key.key === "ArrowDown" ? -1 : 0;
  if (direction === 0) {
    return null;
  }
  const step = key.shiftKey ? param.big_step : key.ctrlKey || key.metaKey ? param.fine_step : param.step;
  return clampValue(param, value + direction * step);
}

/** A typed value (criterion 5): a number clamped silently to the range, or `null` for no number. */
export function parseTyped(param: TraceParam, text: string): number | null {
  const value = Number.parseFloat(text.replace(",", ".").replace(/mm$/i, "").trim());
  return Number.isFinite(value) ? clampValue(param, value) : null;
}

/** How a field shows its value: whole numbers, millimetres with two decimals. */
export function valueText(param: TraceParam, value: number): string {
  return param.kind === "integer" ? String(Math.round(value)) : value.toFixed(2);
}

/** The muted pixel readout of Specks ("0.50 mm  ~ 2 px", criterion 4): the size on the canvas. */
export function specksHint(mm: number, pxPerMm: number): string {
  const px = Math.round(mm * pxPerMm);
  return `${mm.toFixed(2)} mm  ${px < 1 ? "< 1 px" : `~ ${px} px`}`;
}

/** Thousands with a comma ("1,204"). */
export function grouped(n: number): string {
  return Math.round(n).toLocaleString("en-US");
}

function plural(n: number, one: string, many: string): string {
  return `${grouped(n)} ${n === 1 ? one : many}`;
}

/** What a settled trace says, read from the head of the worker's numbers. */
export interface TraceSummary {
  status: "traced" | "too_detailed" | "invalid";
  paths: number;
  nodes: number;
  specks: number;
  foregroundPixels: number;
  coversWholeImage: boolean;
  /** What the Node limit did (criterion 43). */
  nodeLimit: "untouched" | "reached" | "not_met";
}

/** Reads the head of the worker's numbers (`curvyo_ui_core::trace_job_with`'s layout: status,
 * paths, nodes, specks, foreground pixels, whole picture, Node limit, outline count). */
export function readSummary(wire: ArrayLike<number>): TraceSummary {
  const status = wire[0] === 1 ? "too_detailed" : wire[0] === 0 ? "traced" : "invalid";
  return {
    status,
    paths: wire[1] ?? 0,
    nodes: wire[2] ?? 0,
    specks: wire[3] ?? 0,
    foregroundPixels: wire[4] ?? 0,
    coversWholeImage: wire[5] === 1,
    nodeLimit: wire[6] === 1 ? "reached" : wire[6] === 2 ? "not_met" : "untouched",
  };
}

/** The status line when the Node limit acted (criterion 43). */
export function nodeLimitText(outcome: "reached" | "not_met", nodes: number): string {
  return outcome === "reached"
    ? `Node limit reached: ${grouped(nodes)} nodes, tolerance raised.`
    : `Node limit could not be met: ${grouped(nodes)} nodes.`;
}

/** The status line of a settled trace (criteria 11, 15, 16) and its muted second line. */
export function statusText(
  summary: TraceSummary,
  seconds: number,
  pixels: number,
): { line: string; muted: string; canCreate: boolean } {
  if (summary.status === "too_detailed") {
    return { line: TOO_DETAILED, muted: "", canCreate: false };
  }
  if (summary.status === "invalid") {
    return { line: "This image can't be traced.", muted: "", canCreate: false };
  }
  const muted = summary.specks > 0 ? `${plural(summary.specks, "speck", "specks")} ignored` : "";
  if (summary.paths === 0) {
    // The direction from the histogram: more than half the picture below the Threshold means it
    // is too high, otherwise too low (UX notes section 7).
    const direction = summary.foregroundPixels * 2 > pixels ? "too high" : "too low";
    return { line: `0 paths at this Threshold: ${direction}.`, muted, canCreate: false };
  }
  if (summary.nodeLimit !== "untouched") {
    return { line: nodeLimitText(summary.nodeLimit, summary.nodes), muted, canCreate: true };
  }
  if (summary.paths === 1 && summary.coversWholeImage) {
    return { line: "1 path covering the whole image.", muted, canCreate: true };
  }
  const line = `${plural(summary.paths, "path", "paths")}, ${plural(summary.nodes, "node", "nodes")}, ${seconds.toFixed(1)} s`;
  return { line, muted, canCreate: true };
}

/** "Tracing 1 of 3 images" with two or more images, else nothing (criterion 46). `done` counts the
 * job's progress (0-based image it is on), not the position of the previewed image. */
export function imagesLabel(done: number, count: number): string {
  return count >= 2 ? `Tracing ${done + 1} of ${count} images` : "";
}

/** "Tracing 2 of 3..." while Create paths traces several images (criterion 46). */
export function creatingLabel(index: number, count: number): string {
  return count >= 2 ? `Tracing ${index + 1} of ${count}...` : TRACING;
}

/** The success notice (criteria 13, 46). */
export function tracedNotice(images: number, paths: number, nodes: number): string {
  const counts = `${plural(paths, "path", "paths")}, ${plural(nodes, "node", "nodes")}.`;
  return images >= 2 ? `Traced ${grouped(images)} images: ${counts}` : `Traced: ${counts}`;
}

/** The storage key of the remembered values (an app setting, not in the project, criterion 5). */
export const TRACE_VALUES_KEY = "curvyo.trace-values";

/** The part of `Storage` this module uses, so a test can hand in a fake. */
export interface ValueStorage {
  getItem(key: string): string | null;
  setItem(key: string, value: string): void;
}

function defaultStorage(): ValueStorage | undefined {
  try {
    return window.localStorage;
  } catch {
    return undefined;
  }
}

/** The storage key of the remembered Advanced block state (criterion 41). */
export const TRACE_ADVANCED_KEY = "curvyo.trace-advanced-open";

/** The values of `preset` over `base` (criterion 60): never the Threshold, the Node limit or the
 * switches. */
export function withPreset(base: TraceValues, preset: TracePreset): TraceValues {
  return { ...base, ...preset.values };
}

/** The first use (criterion 59): the first preset, the Threshold not known yet, no Node limit,
 * the switches off. */
export function firstChoice(presets: readonly TracePreset[] = TRACE_PRESETS_FALLBACK): TraceChoice {
  const first = presets[0] ?? TRACE_PRESETS_FALLBACK[0];
  const base: TraceValues = {
    threshold: null,
    ...first.values,
    nodeLimit: null,
    invert: false,
    ignoreBackground: false,
    fullSizePreview: false,
  };
  return { preset: first.key, edited: false, values: base };
}

/** A pick of `preset` (criterion 40): its values, not edited. */
export function pickPreset(choice: TraceChoice, preset: TracePreset): TraceChoice {
  return { preset: preset.key, edited: false, values: withPreset(choice.values, preset) };
}

/** An edit of one value: a preset value makes the choice "Custom" (criterion 40), the others
 * (Threshold, Node limit, the switches) do not. */
export function editChoice<K extends keyof TraceValues>(choice: TraceChoice, name: K, value: TraceValues[K]): TraceChoice {
  const edited = choice.edited || (PRESET_FIELDS as readonly string[]).includes(name);
  return { ...choice, edited, values: { ...choice.values, [name]: value } };
}

/** The Preset button's text: the preset's name, or "Custom" after an edit (criterion 40). */
export function presetLabel(choice: TraceChoice, presets: readonly TracePreset[] = TRACE_PRESETS_FALLBACK): string {
  if (choice.edited) {
    return CUSTOM;
  }
  return presets.find((p) => p.key === choice.preset)?.label ?? CUSTOM;
}

/** The settings JSON `trace_job` of the wasm module reads, with `threshold`. */
export function settingsJson(values: TraceValues, threshold: number): string {
  return JSON.stringify({
    threshold,
    smooth: values.smooth,
    specks: values.specks,
    corners: values.corners,
    upscale: values.upscale,
    denoise: values.denoise,
    node_limit: values.nodeLimit,
    invert: values.invert,
    ignore_background: values.ignoreBackground,
  });
}

/** The remembered choice (criterion 66): the preset, whether it was edited, and the preset
 * values; the first use for a new user or anything unreadable. The Threshold always starts at the
 * image's automatic value (criterion 9); the Node limit and the switches start off (Proposal). A
 * slice-1 setting (`{ smooth, specks }`) reads as an edited Logo / Line art. */
export function readChoice(
  params: readonly TraceParam[] = TRACE_PARAMS_FALLBACK,
  presets: readonly TracePreset[] = TRACE_PRESETS_FALLBACK,
  storage: ValueStorage | undefined = defaultStorage(),
): TraceChoice {
  const first = firstChoice(presets);
  const byName = (name: TraceParamName) => params.find((p) => p.name === name) ?? TRACE_PARAMS_FALLBACK.find((p) => p.name === name)!;
  const number = (raw: Record<string, unknown>, field: keyof PresetValues, name: TraceParamName) =>
    typeof raw[field] === "number" ? clampValue(byName(name), raw[field]) : first.values[field];
  try {
    const raw: unknown = JSON.parse(storage?.getItem(TRACE_VALUES_KEY) ?? "null");
    if (!raw || typeof raw !== "object") {
      return first;
    }
    const stored = raw as { preset?: unknown; edited?: unknown; values?: unknown };
    const slice1 = stored.values === undefined;
    const values = (slice1 ? raw : stored.values) as Record<string, unknown>;
    if (!values || typeof values !== "object") {
      return first;
    }
    const preset = typeof stored.preset === "string" && presets.some((p) => p.key === stored.preset) ? stored.preset : first.preset;
    return {
      preset,
      edited: slice1 || stored.edited === true,
      values: {
        ...first.values,
        smooth: number(values, "smooth", "smooth"),
        specks: number(values, "specks", "specks"),
        corners: number(values, "corners", "corners"),
        upscale: number(values, "upscale", "upscale"),
        denoise: number(values, "denoise", "denoise"),
      },
    };
  } catch {
    /* A broken value reads as the first use. */
  }
  return first;
}

/** Remembers the choice: the preset, `edited` and the preset values. A failing write is
 * ignored. */
export function writeChoice(choice: TraceChoice, storage: ValueStorage | undefined = defaultStorage()): void {
  const values: Record<string, number> = {};
  for (const field of PRESET_FIELDS) {
    values[field] = choice.values[field];
  }
  try {
    storage?.setItem(TRACE_VALUES_KEY, JSON.stringify({ preset: choice.preset, edited: choice.edited, values }));
  } catch {
    /* Not kept; the next start uses the first use. */
  }
}

/** Whether the Advanced block was left open (criterion 41). */
export function readAdvancedOpen(storage: ValueStorage | undefined = defaultStorage()): boolean {
  try {
    return storage?.getItem(TRACE_ADVANCED_KEY) === "1";
  } catch {
    return false;
  }
}

/** Remembers whether the Advanced block is open. */
export function writeAdvancedOpen(open: boolean, storage: ValueStorage | undefined = defaultStorage()): void {
  try {
    storage?.setItem(TRACE_ADVANCED_KEY, open ? "1" : "0");
  } catch {
    /* Not kept. */
  }
}

/** The Node limit as typed (criterion 43): empty or 0 is none, else a whole number in range. */
export function parseNodeLimit(text: string): number | null {
  const digits = text.replace(/[,\s]/g, "");
  if (digits === "") {
    return null;
  }
  const value = Number.parseInt(digits, 10);
  if (!Number.isFinite(value) || value <= 0) {
    return null;
  }
  return Math.min(10_000_000, value);
}

/** The Node limit as shown after leaving the field: thousands with a comma, empty for none. */
export function nodeLimitShown(limit: number | null): string {
  return limit === null ? "" : grouped(limit);
}
