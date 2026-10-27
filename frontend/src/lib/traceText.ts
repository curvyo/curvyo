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

/** The values of the bar. `threshold` is `null` until the image's automatic value is known. */
export interface TraceValues {
  threshold: number | null;
  smooth: number;
  specks: number;
}

/** One field of the settings schema (`trace_params()` of the wasm module, criterion 6). */
export interface TraceParam {
  name: "threshold" | "smooth" | "specks";
  label: string;
  kind: "integer" | "length";
  unit: string;
  min: number;
  max: number;
  default: number;
  step: number;
  big_step: number;
  fine_step: number;
  group: string;
  advanced: boolean;
}

/** The schema of slice 1 as the crate states it (the wasm module's `trace_params()` is the source;
 * this copy is what the bar uses before the module is loaded and what the tests compare). */
export const TRACE_PARAMS_FALLBACK: readonly TraceParam[] = [
  { name: "threshold", label: "Threshold", kind: "integer", unit: "", min: 0, max: 255, default: 128, step: 1, big_step: 10, fine_step: 1, group: "threshold", advanced: false },
  { name: "smooth", label: "Smooth", kind: "integer", unit: "", min: 0, max: 100, default: 80, step: 1, big_step: 10, fine_step: 1, group: "smooth", advanced: false },
  { name: "specks", label: "Ignore below", kind: "length", unit: "mm", min: 0, max: 10, default: 0.5, step: 0.1, big_step: 1, fine_step: 0.01, group: "specks", advanced: false },
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
  return `${mm.toFixed(2)} mm  ~ ${px} px`;
}

/** Thousands with a comma ("1,204"). */
function grouped(n: number): string {
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
}

/** Reads the head of the worker's numbers (`curvyo_ui_core::trace_job`'s layout). */
export function readSummary(wire: ArrayLike<number>): TraceSummary {
  const status = wire[0] === 1 ? "too_detailed" : wire[0] === 0 ? "traced" : "invalid";
  return {
    status,
    paths: wire[1] ?? 0,
    nodes: wire[2] ?? 0,
    specks: wire[3] ?? 0,
    foregroundPixels: wire[4] ?? 0,
    coversWholeImage: wire[5] === 1,
  };
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
  if (summary.paths === 1 && summary.coversWholeImage) {
    return { line: "1 path covering the whole image.", muted, canCreate: true };
  }
  const line = `${plural(summary.paths, "path", "paths")}, ${plural(summary.nodes, "node", "nodes")}, ${seconds.toFixed(1)} s`;
  return { line, muted, canCreate: true };
}

/** "Tracing 1 of 3 images" with two or more images, else nothing (criterion 46). */
export function imagesLabel(shownIndex: number, count: number): string {
  return count >= 2 ? `Tracing ${shownIndex + 1} of ${count} images` : "";
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

/** The remembered Smooth and Specks; the defaults for a new user or anything unreadable. The
 * Threshold always starts at the image's automatic value (criterion 9). */
export function readValues(
  params: readonly TraceParam[] = TRACE_PARAMS_FALLBACK,
  storage: ValueStorage | undefined = defaultStorage(),
): TraceValues {
  const byName = (name: TraceParam["name"]) => params.find((p) => p.name === name) ?? TRACE_PARAMS_FALLBACK.find((p) => p.name === name)!;
  const fallback = { threshold: null, smooth: byName("smooth").default, specks: byName("specks").default };
  try {
    const raw: unknown = JSON.parse(storage?.getItem(TRACE_VALUES_KEY) ?? "null");
    if (raw && typeof raw === "object") {
      const { smooth, specks } = raw as { smooth?: unknown; specks?: unknown };
      return {
        threshold: null,
        smooth: typeof smooth === "number" ? clampValue(byName("smooth"), smooth) : fallback.smooth,
        specks: typeof specks === "number" ? clampValue(byName("specks"), specks) : fallback.specks,
      };
    }
  } catch {
    /* A broken value reads as the defaults. */
  }
  return fallback;
}

/** Remembers Smooth and Specks. A failing write is ignored. */
export function writeValues(values: TraceValues, storage: ValueStorage | undefined = defaultStorage()): void {
  try {
    storage?.setItem(TRACE_VALUES_KEY, JSON.stringify({ smooth: values.smooth, specks: values.specks }));
  } catch {
    /* Not kept; the next start uses the defaults. */
  }
}
