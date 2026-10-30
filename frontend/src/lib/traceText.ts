/**
 * The words and the plain rules of the Trace tool (`specs/0025-raster-trace` criteria 1, 3, 4, 5,
 * 9, 11, 13, 15, 16, 46 and 51 to 55; UX notes sections 2, 3, 5, 7, 13 and 14): the tooltips, the empty states,
 * the status line, the notices, the field steps and the remembered values. No React and no wasm,
 * so node tests can check them. Rust sends codes and counts; the sentences are written here.
 */

/** The Select bar button's label. */
export const TRACE_BUTTON = "Trace";

/** The Select bar button's tooltip: its name and key only (criterion 1). */
export const TRACE_TOOLTIP = "Trace tool (I)";

/** The state label that opens the Trace bar (UX notes section 1). */
export const TRACE_STATE_LABEL = "Trace";

/** The bar with nothing selected (criterion 3). */
export const TRACE_EMPTY = "Click an image to trace it.";
export const TRACE_IMPORT = "Import image...";

/** The bar with something else selected (criterion 3). */
export const TRACE_NOT_IMAGE = "The selection is not an image. Click an image to trace it.";

/** The name of the progress pill (criterion 10). */
export const TRACING = "Tracing";

/** The tooltip of the Stopped pill (criterion 53). */
export const PREVIEW_OUT_OF_DATE = "Preview is out of date";

/** The pill's x: name and tooltip (criterion 10). */
export const STOP = "Stop (Esc)";

/** The pill after Stop or the first Escape (criterion 53, UX notes 21.2). */
export const STOPPED = "Stopped";

/** The pill of a failed trace (criterion 54). */
export const FAILED = "Failed";

/** What the pill at the end of the bar shows (criteria 10, 53, 54; `specs/0089` Frage 84): at
 * rest the finish button. */
export type TraceSlot =
  | { kind: "none" }
  /** A job past 400 ms; `percent` is `null` before its first report or while the picture loads. */
  | { kind: "running"; percent: number | null }
  | { kind: "stopped" }
  /** `text` is the sentence of `failureText`. */
  | { kind: "failed"; text: string };

/** The running pill's text: "42 %", or "..." without a fraction (UX notes 21.2). */
export function percentText(percent: number | null): string {
  return percent === null ? "..." : `${Math.round(percent)} %`;
}

/** The muted line of a preview over 4 Mpx (criterion 52). */
export const REDUCED_COPY = "Preview from a reduced copy";

/** The finish button's label: always blue, the same as Enter (`specs/0089` Frage 84; customer
 * 2026-10-30: "Trace", not "Create paths"). */
export const TRACE_FINISH = "Trace";

/** A job's progress shows in the finish button only after it has run this long (UX notes 21.2). */
export const PROGRESS_AFTER_MS = 400;

/** Once shown, the progress stays at least this long, so a job that ends just after
 * `PROGRESS_AFTER_MS` does not flash percent and x (customer 2026-10-30). */
export const PROGRESS_HOLD_MS = 300;

/** How much longer a progress shown at `shownAt` must stay at `now` (0: it may go). */
export function progressHoldLeft(shownAt: number | null, now: number): number {
  return shownAt === null ? 0 : Math.max(0, shownAt + PROGRESS_HOLD_MS - now);
}

/** The slot's button after Stop or a failure (criteria 53, 54). */
export const RETRY = "Retry";

/** Over this many pixels the preview is always the reduced copy (criterion 52). */
export const REDUCED_PREVIEW_OVER_PX = 4_000_000;

/** Over this many nodes the status warns that the result is slow to edit (criterion 55). */
export const SLOW_TO_EDIT_NODES = 100_000;

/** A view of the Trace tool (criterion 51; `specs/0089` criterion 21 adds Mask). */
export type TraceViewMode = "overlay" | "mask" | "trace" | "image";

/** The View pill's segments in order, with their tooltips (UX notes section 13, `specs/0089` UX
 * notes §4: Mask after Overlay). */
export const TRACE_VIEWS: readonly { mode: TraceViewMode; label: string; tooltip: string }[] = [
  { mode: "overlay", label: "Overlay", tooltip: "Overlay" },
  { mode: "mask", label: "Mask", tooltip: "Mask" },
  { mode: "trace", label: "Trace", tooltip: "Trace" },
  { mode: "image", label: "Image", tooltip: "Image (\\)" },
];

/** The selection methods in menu order with their second lines (`specs/0089` criterion 14, UX
 * notes §5); the index is the settings JSON's `method`. */
export const TRACE_METHODS: readonly { key: string; label: string; detail: string }[] = [
  { key: "brightness", label: "Brightness", detail: "Dark and light pixels by a cut-off." },
  { key: "remove-background", label: "Remove background", detail: "Everything unlike the background." },
  { key: "pick-colours", label: "Pick colours", detail: "Pixels like the colours you picked." },
];

/** The method's index: Brightness, Remove background, Pick colours. */
export const BRIGHTNESS = 0;
export const REMOVE_BACKGROUND = 1;
export const PICK_COLOURS = 2;

/** The most colours Pick colours takes (`specs/0089` criterion 17, **Proposal**). */
export const MAX_SWATCHES = 8;

/** The Colour tolerance a first use of each colour method starts at (the crate's
 * `DEFAULT_BACKGROUND_TOLERANCE` and `DEFAULT_PICK_TOLERANCE`). */
export const DEFAULT_BACKGROUND_TOLERANCE = 40;
export const DEFAULT_PICK_TOLERANCE = 19;

/** The pipette's name and tooltip (UX notes §6, the Style eyedropper's words). */
export const PICK_COLOUR = "Pick colour from the image";
export const PICK_BACKGROUND = "Pick background colour from the image";
export const PICK_TOOLTIP = "Pick a colour from the image";

/** The Auto toggle of Remove background and the hex of a transparent background (UX notes §5). */
export const AUTO = "Auto";
export const NO_BACKGROUND = "none";

/** The status with Pick colours and no colour (UX notes §5, §10). */
export const NO_COLOUR_PICKED = "No colour picked.";

/** The status of a ninth pick (UX notes §5). */
export const SWATCHES_FULL = `${MAX_SWATCHES} colours picked`;

/** The name of a swatch's Remove button. */
export function removeSwatchLabel(hex: string): string {
  return `Remove ${hex}`;
}

/** The swatches after a pick of `hex` (Frage 89 A): a click replaces them with this one, Shift
 * adds it (a colour already picked or a ninth is not added). */
export function pickSwatch(swatches: readonly string[], hex: string, add: boolean): string[] {
  if (!add) {
    return [hex];
  }
  if (swatches.includes(hex) || swatches.length >= MAX_SWATCHES) {
    return [...swatches];
  }
  return [...swatches, hex];
}

/** `status` with "8 colours picked" in its muted line while Pick colours holds the most swatches
 * (UX notes 5). */
export function withSwatchNote(status: TraceStatusText, values: Pick<TraceValues, "method" | "swatches">): TraceStatusText {
  return values.method === PICK_COLOURS && values.swatches.length >= MAX_SWATCHES && status.muted === ""
    ? { ...status, muted: SWATCHES_FULL }
    : status;
}

/** The parts of a key event the backslash keys read. */
export interface PeekKey {
  key: string;
  shiftKey: boolean;
  repeat: boolean;
}

/** Holding `\` peeks at the image (criterion 51); a repeat is ignored. Ctrl and Alt are allowed:
 * AltGr types the backslash on some layouts. */
export function isPeekKey(event: PeekKey): boolean {
  return event.key === "\\" && !event.shiftKey && !event.repeat;
}

/** Shift+`\` cycles the views (criterion 51); on US layouts it types "|". */
export function isCycleKey(event: PeekKey): boolean {
  return event.shiftKey && !event.repeat && (event.key === "|" || event.key === "\\");
}

/** Whether a picture of `width` x `height` px is previewed from its reduced copy only (criterion
 * 52). */
export function previewIsReduced(width: number, height: number): boolean {
  return width * height > REDUCED_PREVIEW_OVER_PX;
}

/** The failure's tooltip and hidden alert (criterion 54). */
export function failureText(reason: string): string {
  return `Trace failed: ${reason}. Your drawing is unchanged.`;
}

/** A plain reason for a failed job: out of memory with the picture's size, else an internal
 * error; no stack, no code (UX notes section 14). */
export function failureReason(message: string, pixels: number): string {
  return /memory|allocat/i.test(message) ? `out of memory at ${Math.round(pixels / 1_000_000)} Mpx` : "internal error";
}

/** A region with 65 000 holes or more (criterion 16). */
export const TOO_DETAILED = "Too much detail: raise Specks.";

/** The Preset button's text after an edit (criterion 40). */
export const CUSTOM = "Custom";

/** The expander's name and tooltip (UX notes 19, the pair of the Text bar's `expandLabel`). */
export const TRACE_EXPAND_LABELS = { closed: "More trace options", open: "Fewer trace options" } as const;

/** The bar's hidden description (UX notes 19). */
export const TRACE_BAR_DESCRIPTION = "Enter creates the paths";

/** The info button's name and tooltip, and the heading of its page (criterion 70). */
export const TRACE_INFO_TITLE = "About Trace";

/** The id of the info page, which the button's `aria-controls` names. */
export const TRACE_INFO_PAGE_ID = "trace-info-page";

/** The info page's intro (criterion 70, exact text). */
export const TRACE_INFO_INTRO =
  "Trace turns the selected image into paths. The method decides which parts become shapes. Press Enter or the Trace button to create the paths.";

/** The info page's entries in bar order, each "Term: text" (criterion 70, exact text). */
export const TRACE_INFO_ENTRIES: readonly string[] = [
  "Preset: a named set of values for what you do with the paths.",
  "Logo / Line art: sharp corners, clean curves.",
  "Photo: many shapes, photos trace roughly.",
  "Silhouette for cutting/carving: closed, simple, few nodes; drops shapes and holes below 2 mm.",
  "Engraving: keeps fine lines, drops shapes below 0.1 mm.",
  "View: the image with the outline, the mask, the result alone, or the image alone. Hold \\ to peek at the image; Shift+\\ steps through the views.",
  "Mask: what the tracer turns into shapes, black on white, after Ignore below and Drop border shapes.",
  "Method: how pixels become shapes.",
  "Brightness: pixels darker than the Threshold.",
  "Remove background: everything unlike the background colour.",
  "Background: the colour Remove background leaves out; Auto takes it from the image border.",
  "Pick colours: pixels like the colours picked with the pipette; Shift+click adds a colour, up to 8.",
  "Pipette: takes a colour from the image under a click; Esc ends it.",
  "Colour tolerance: how far a colour may be from the background or a picked colour and still count as it.",
  "Threshold: pixels darker than this brightness (0 to 100 %) become shapes.",
  "Smooth: calms the pixel steps of the outline before the fit; higher is calmer.",
  "Tolerance: how far in pixels the paths may stray from the calmed outline; higher gives fewer nodes.",
  "Ignore below: drops shapes and holes smaller than this size in mm.",
  "Denoise: smooths out noise in the picture before tracing.",
  "Upscale: doubles small images before tracing; Auto does it below 1 Mpx.",
  "Corners: higher turns more bends into sharp corners.",
  "Invert: traces what the method leaves out; transparent pixels never become shapes.",
  "Drop border shapes: drops shapes that touch the image border.",
];

/** An info entry split into its term and its text at the first ": ". */
export function infoEntryParts(entry: string): { term: string; text: string } {
  const at = entry.indexOf(": ");
  return at < 0 ? { term: entry, text: "" } : { term: entry.slice(0, at), text: entry.slice(at + 2) };
}

/** The Threshold's luma (0 to 255) of a whole percent of brightness, rounded half up (Terms,
 * Frage 75): 1 % is 3, 50 % is 128. */
export function percentToLuma(percent: number): number {
  const p = Math.min(100, Math.max(0, Math.round(percent)));
  return Math.floor((p * 255) / 100 + 0.5);
}

/** The whole percent of brightness the field shows for a luma, rounded half up (Terms, Frage
 * 75): 127 and 128 are 50 %, 126 is 49 %. */
export function lumaToPercent(luma: number): number {
  const t = Math.min(255, Math.max(0, Math.round(luma)));
  return Math.floor((t * 100) / 255 + 0.5);
}

/** The Threshold as the bar shows it: brightness 0 to 100 % in whole steps of 1 and 10 (the
 * schema keeps the luma 0 to 255 the tracer uses). */
export function thresholdPercentParam(param: TraceParam): TraceParam {
  return { ...param, unit: "%", min: 0, max: 100, default: lumaToPercent(param.default), step: 1, big_step: 10, fine_step: 1 };
}

/** The values of a trace. `threshold` is the luma 0 to 255, `null` until the image's automatic
 * value is known; `upscale` and `denoise` are choice indices (Auto | Off, Off | Low | High). */
export interface TraceValues {
  threshold: number | null;
  smooth: number;
  /** The deviation tolerance in source pixels (`specs/0089` Frage 85 B). */
  tolerance: number;
  specks: number;
  corners: number;
  upscale: number;
  denoise: number;
  invert: boolean;
  ignoreBackground: boolean;
  /** The selection method: `BRIGHTNESS`, `REMOVE_BACKGROUND` or `PICK_COLOURS`. */
  method: number;
  /** The Colour tolerance of Remove background and of Pick colours, 0 to 100, each its own. */
  backgroundTolerance: number;
  pickTolerance: number;
  /** Remove background takes the background from the image border. */
  backgroundAuto: boolean;
  /** The background colour `#RRGGBB`, empty for a transparent one, `null` while Auto has not
   * read the image yet. */
  background: string | null;
  /** The picked colours `#RRGGBB` (the tool's session, not remembered). */
  swatches: readonly string[];
}

/** The values a preset sets (criterion 60). */
export type PresetValues = Pick<TraceValues, "smooth" | "tolerance" | "specks" | "corners" | "upscale" | "denoise">;

/** The names of the preset values: an edit of one of them makes the choice "Custom". */
export const PRESET_FIELDS: readonly (keyof PresetValues)[] = ["smooth", "tolerance", "specks", "corners", "upscale", "denoise"];

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
  { key: "logo", label: "Logo / Line art", detail: "Sharp corners, clean curves.", values: { smooth: 80, tolerance: 0.5, specks: 0.5, corners: 70, upscale: 1, denoise: 0 } },
  { key: "photo", label: "Photo", detail: "Many shapes. Photos trace roughly.", values: { smooth: 50, tolerance: 0.5, specks: 0.1, corners: 30, upscale: 1, denoise: 0 } },
  { key: "silhouette", label: "Silhouette for cutting/carving", detail: "Closed, simple, few nodes.", values: { smooth: 100, tolerance: 1.5, specks: 2, corners: 70, upscale: 1, denoise: 1 } },
  { key: "engraving", label: "Engraving", detail: "Keeps fine lines.", values: { smooth: 80, tolerance: 0.35, specks: 0.1, corners: 70, upscale: 0, denoise: 0 } },
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
  | "tolerance"
  | "specks"
  | "invert"
  | "ignore_background"
  | "denoise"
  | "upscale"
  | "corners"
  | "method"
  | "colour_tolerance";

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
  { name: "tolerance", label: "Tolerance", kind: "length", unit: "px", min: 0.1, max: 3, default: 0.5, step: 0.05, big_step: 0.5, fine_step: 0.01, group: "tolerance", advanced: false, choices: [] },
  { name: "specks", label: "Ignore below", kind: "length", unit: "mm", min: 0, max: 10, default: 0.5, step: 0.1, big_step: 1, fine_step: 0.01, group: "specks", advanced: false, choices: [] },
  { name: "invert", label: "Invert", kind: "toggle", unit: "", min: 0, max: 1, default: 0, step: 1, big_step: 1, fine_step: 1, group: "advanced", advanced: true, choices: [] },
  { name: "ignore_background", label: "Drop border shapes", kind: "toggle", unit: "", min: 0, max: 1, default: 0, step: 1, big_step: 1, fine_step: 1, group: "advanced", advanced: true, choices: [] },
  { name: "denoise", label: "Denoise", kind: "choice", unit: "", min: 0, max: 2, default: 0, step: 1, big_step: 1, fine_step: 1, group: "advanced", advanced: true, choices: ["Off", "Low", "High"] },
  { name: "upscale", label: "Upscale", kind: "choice", unit: "", min: 0, max: 1, default: 1, step: 1, big_step: 1, fine_step: 1, group: "advanced", advanced: true, choices: ["Auto", "Off"] },
  { name: "corners", label: "Corners", kind: "integer", unit: "", min: 0, max: 100, default: 70, step: 1, big_step: 10, fine_step: 1, group: "advanced", advanced: true, choices: [] },
  { name: "method", label: "Method", kind: "choice", unit: "", min: 0, max: 2, default: 0, step: 1, big_step: 1, fine_step: 1, group: "method", advanced: false, choices: ["Brightness", "Remove background", "Pick colours"] },
  { name: "colour_tolerance", label: "Colour tolerance", kind: "integer", unit: "", min: 0, max: 100, default: 19, step: 1, big_step: 10, fine_step: 1, group: "method", advanced: false, choices: [] },
];

/** `value` clamped to the field's range and rounded to its finest step (criterion 5). */
export function clampValue(param: TraceParam, value: number): number {
  if (!Number.isFinite(value)) {
    return param.default;
  }
  const clamped = Math.min(param.max, Math.max(param.min, value));
  return param.kind === "integer" ? Math.round(clamped) : Math.round(clamped * 100) / 100;
}

/** The grid of a step or a drag: Ctrl fine, Shift coarse (the app's value fields). */
export type TraceGrid = "normal" | "coarse" | "fine";

/** The step of a grid (criterion 5): Up and Down `step`, Shift `big_step`, Ctrl `fine_step`. */
export function gridStep(param: TraceParam, grid: TraceGrid): number {
  return grid === "fine" ? param.fine_step : grid === "coarse" ? param.big_step : param.step;
}

/** `steps` steps of `grid` from `value`, clamped (criterion 5). */
export function stepOnGrid(param: TraceParam, value: number, steps: number, grid: TraceGrid): number {
  return clampValue(param, value + steps * gridStep(param, grid));
}

/** The value at position `p` (0 to 1) along the field, rounded to the grid's step and clamped:
 * a click, a drag, Home (0) and End (1). */
export function valueAt(param: TraceParam, p: number, grid: TraceGrid): number {
  const step = gridStep(param, grid);
  return clampValue(param, Math.round((param.min + p * (param.max - param.min)) / step) * step);
}

/** A typed value (criterion 5): a number clamped silently to the range, or `null` for no number. */
export function parseTyped(param: TraceParam, text: string): number | null {
  const value = Number.parseFloat(text.replace(",", ".").trim().replace(/(mm|%)$/i, "").trim());
  return Number.isFinite(value) ? clampValue(param, value) : null;
}

/** How a field shows its value: whole numbers, millimetres with two decimals. */
export function valueText(param: TraceParam, value: number): string {
  return param.kind === "integer" ? String(Math.round(value)) : value.toFixed(2);
}

/** The muted pixel readout of Specks ("~ 2 px", criterion 4; the field shows the mm): the size on the canvas. */
export function specksHint(mm: number, pxPerMm: number): string {
  const px = Math.round(mm * pxPerMm);
  return px < 1 ? "< 1 px" : `~ ${px} px`;
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
}

/** Reads the head of the worker's numbers (`curvyo_ui_core::trace_job_with`'s layout: status,
 * paths, nodes, specks, foreground pixels, whole picture, outline count). */
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

/** What the bar says about the trace (criteria 11 and 71, UX notes 20.4): the state row's line
 * and its muted line (empty when no state is active), the counts line of the Advanced block and
 * its muted line, and whether Enter may create. */
export interface TraceStatusText {
  line: string;
  muted: string;
  counts: string;
  countsMuted: string;
  canCreate: boolean;
}

/** No state, no counts. */
export const NO_STATUS: TraceStatusText = { line: "", muted: "", counts: "", countsMuted: "", canCreate: false };

/** The line of a trace without paths in a colour method (UX notes §10), or `null` for
 * Brightness, whose line follows the Threshold's direction. */
export function zeroPathsLine(values: Pick<TraceValues, "method" | "swatches">): string | null {
  if (values.method === PICK_COLOURS) {
    return values.swatches.length === 0 ? "0 paths: no colour picked." : "0 paths at this Colour tolerance: too low.";
  }
  return values.method === REMOVE_BACKGROUND ? "0 paths at this Colour tolerance: too high." : null;
}

/** The state and the counts of a settled trace (criteria 11, 15, 16, 52, 55); `zeroLine`
 * replaces the Threshold's line of a trace without paths (a colour method). */
export function statusText(
  summary: TraceSummary,
  seconds: number,
  pixels: number,
  reduced = false,
  zeroLine: string | null = null,
): TraceStatusText {
  if (summary.status === "too_detailed") {
    return { ...NO_STATUS, line: TOO_DETAILED };
  }
  if (summary.status === "invalid") {
    return { ...NO_STATUS, line: "This image can't be traced." };
  }
  const counts = `${plural(summary.paths, "path", "paths")}, ${plural(summary.nodes, "node", "nodes")}, ${seconds.toFixed(1)} s`;
  const countsMuted = summary.specks > 0 ? `${plural(summary.specks, "speck", "specks")} ignored` : "";
  // Over 4 Mpx the counts are the reduced copy's, and the state row says so (criterion 52).
  const muted = reduced ? REDUCED_COPY : "";
  const settled = { line: "", muted, counts, countsMuted, canCreate: true };
  if (summary.paths === 0) {
    // The direction from the histogram: more than half the picture below the Threshold means it
    // is too high, otherwise too low (UX notes section 7).
    const direction = summary.foregroundPixels * 2 > pixels ? "too high" : "too low";
    return { ...settled, line: zeroLine ?? `0 paths at this Threshold: ${direction}.`, canCreate: false };
  }
  if (summary.nodes > SLOW_TO_EDIT_NODES) {
    return { ...settled, line: `${grouped(summary.nodes)} nodes. Slow to edit.` };
  }
  if (summary.paths === 1 && summary.coversWholeImage) {
    return { ...settled, line: "1 path covering the whole image." };
  }
  return settled;
}

/** "Tracing 1 of 3 images" with two or more images, else nothing (criterion 46). `done` counts the
 * job's progress (0-based image it is on), not the position of the previewed image. */
export function imagesLabel(done: number, count: number): string {
  return count >= 2 ? `Tracing ${done + 1} of ${count} images` : "";
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

/** The values of `preset` over `base` (criterion 60): never the Threshold or the switches. */
export function withPreset(base: TraceValues, preset: TracePreset): TraceValues {
  return { ...base, ...preset.values };
}

/** The first use (criterion 59): the first preset, the Threshold not known yet, the switches
 * off. */
export function firstChoice(presets: readonly TracePreset[] = TRACE_PRESETS_FALLBACK): TraceChoice {
  const first = presets[0] ?? TRACE_PRESETS_FALLBACK[0];
  const base: TraceValues = {
    threshold: null,
    ...first.values,
    invert: false,
    ignoreBackground: false,
    method: BRIGHTNESS,
    backgroundTolerance: DEFAULT_BACKGROUND_TOLERANCE,
    pickTolerance: DEFAULT_PICK_TOLERANCE,
    backgroundAuto: true,
    background: null,
    swatches: [],
  };
  return { preset: first.key, edited: false, values: base };
}

/** A pick of `preset` (criterion 40): its values, not edited. */
export function pickPreset(choice: TraceChoice, preset: TracePreset): TraceChoice {
  return { preset: preset.key, edited: false, values: withPreset(choice.values, preset) };
}

/** An edit of one value: a preset value makes the choice "Custom" (criterion 40), the others
 * (Threshold, the switches) do not. */
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

/** The Colour tolerance of the current method. */
export function colourTolerance(values: TraceValues): number {
  return values.method === REMOVE_BACKGROUND ? values.backgroundTolerance : values.pickTolerance;
}

/** The settings JSON `trace_job` of the wasm module reads, with `threshold`. Pick colours takes
 * the image's `suggested` background (resolved once on the full image, so the reduced preview and
 * the full-size trace agree; empty: transparent). */
export function settingsJson(values: TraceValues, threshold: number, suggested?: string): string {
  const background = values.method === PICK_COLOURS && suggested !== undefined ? suggested : values.background;
  return JSON.stringify({
    method: values.method,
    colour_tolerance: colourTolerance(values),
    background: background === "" ? null : background,
    swatches: values.swatches,
    threshold,
    smooth: values.smooth,
    tolerance: values.tolerance,
    specks: values.specks,
    corners: values.corners,
    upscale: values.upscale,
    denoise: values.denoise,
    invert: values.invert,
    ignore_background: values.ignoreBackground,
  });
}

/** The remembered choice (criterion 66): the preset, whether it was edited, and the preset
 * values; the first use for a new user or anything unreadable. The Threshold always starts at the
 * image's automatic value (criterion 9); the switches start off (Proposal). A
 * slice-1 setting (`{ smooth, specks }`) reads as an edited Logo / Line art; Smooth keeps its
 * number and meaning (calmness, `specs/0089` Frage 85 B); a missing preset value is its preset's;
 * a missing method is Brightness; keys of older builds (Node limit, Full-size preview) are
 * ignored. */
export function readChoice(
  params: readonly TraceParam[] = TRACE_PARAMS_FALLBACK,
  presets: readonly TracePreset[] = TRACE_PRESETS_FALLBACK,
  storage: ValueStorage | undefined = defaultStorage(),
): TraceChoice {
  const first = firstChoice(presets);
  const byName = (name: TraceParamName) => params.find((p) => p.name === name) ?? TRACE_PARAMS_FALLBACK.find((p) => p.name === name)!;
  try {
    const raw: unknown = JSON.parse(storage?.getItem(TRACE_VALUES_KEY) ?? "null");
    if (!raw || typeof raw !== "object") {
      return first;
    }
    const stored = raw as {
      preset?: unknown;
      edited?: unknown;
      values?: unknown;
      method?: unknown;
      backgroundTolerance?: unknown;
      pickTolerance?: unknown;
    };
    // The method and its tolerances are app settings too (`specs/0089` criterion 20); a missing
    // method reads as Brightness.
    const method = (m: unknown) => (m === REMOVE_BACKGROUND || m === PICK_COLOURS ? m : BRIGHTNESS);
    const tolerance = (t: unknown, fallback: number) =>
      typeof t === "number" && Number.isFinite(t) ? Math.min(100, Math.max(0, Math.round(t))) : fallback;
    const slice1 = stored.values === undefined;
    const values = (slice1 ? raw : stored.values) as Record<string, unknown>;
    if (!values || typeof values !== "object") {
      return first;
    }
    const preset = typeof stored.preset === "string" && presets.some((p) => p.key === stored.preset) ? stored.preset : first.preset;
    // A value the stored choice does not hold reads as its preset's (`specs/0089` Frage 85 B: a
    // 0025 choice has no Tolerance and starts at its preset's, Silhouette 1.5 px, not Logo's).
    const presetValues = presets.find((p) => p.key === preset)?.values ?? first.values;
    const number = (raw: Record<string, unknown>, field: keyof PresetValues, name: TraceParamName) =>
      typeof raw[field] === "number" ? clampValue(byName(name), raw[field]) : presetValues[field];
    return {
      preset,
      edited: slice1 || stored.edited === true,
      values: {
        ...first.values,
        smooth: number(values, "smooth", "smooth"),
        tolerance: number(values, "tolerance", "tolerance"),
        specks: number(values, "specks", "specks"),
        corners: number(values, "corners", "corners"),
        upscale: number(values, "upscale", "upscale"),
        denoise: number(values, "denoise", "denoise"),
        method: method(stored.method),
        backgroundTolerance: tolerance(stored.backgroundTolerance, first.values.backgroundTolerance),
        pickTolerance: tolerance(stored.pickTolerance, first.values.pickTolerance),
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
    storage?.setItem(
      TRACE_VALUES_KEY,
      JSON.stringify({
        preset: choice.preset,
        edited: choice.edited,
        values,
        method: choice.values.method,
        backgroundTolerance: choice.values.backgroundTolerance,
        pickTolerance: choice.values.pickTolerance,
      }),
    );
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

/** The Advanced values that differ from their defaults, in block order, as the closed expander's
 * "Changed: Invert, Corners" line; `null` when none (UX notes 19). */
export function changedAdvancedText(values: TraceValues, params: readonly TraceParam[] = TRACE_PARAMS_FALLBACK): string | null {
  const dflt = (name: TraceParam["name"]) => params.find((p) => p.name === name)?.default ?? 0;
  const changed: string[] = [];
  if (values.invert) {
    changed.push("Invert");
  }
  if (values.ignoreBackground) {
    changed.push("Drop border shapes");
  }
  if (values.denoise !== dflt("denoise")) {
    changed.push("Denoise");
  }
  if (values.upscale !== dflt("upscale")) {
    changed.push("Upscale");
  }
  if (values.corners !== dflt("corners")) {
    changed.push("Corners");
  }
  return changed.length > 0 ? `Changed: ${changed.join(", ")}` : null;
}
