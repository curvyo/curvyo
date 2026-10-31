/**
 * The automatic analysis in the Trace bar (`specs/0089-trace-quality-and-colour` slice F,
 * criteria 31 to 33, UX notes §3): the worker's report read from JSON, the values it sets on a
 * new image, and the analysis line's words per state. The analysis sets only image-dependent
 * values (method, Threshold, background, Colour tolerance; Frage 86 A): never the preset, Smooth,
 * Tolerance, Corners, Ignore below, Denoise, Upscale, Invert, Drop border shapes, Output or picked
 * colours. Applied to a new image automatically, never after a manual change (Frage 83 B).
 */

import { BRIGHTNESS, PICK_COLOURS, REMOVE_BACKGROUND, type TraceValues } from "./traceText.ts";

/** The image types of the analysis (criterion 31). */
export type TraceImageType = "graphic" | "logo" | "line_drawing" | "sketch";

/** The capture conditions (criterion 31: not types). */
export type TraceCapture = "digital" | "scan" | "paper_photo";

/** Why the proposal differs from Brightness at the automatic Threshold. */
export type TraceReason = "pale_colours" | "transparent_background" | "shadow_left_out" | "low_confidence";

/** The worker's analysis of one image (`trace_analyse` of the wasm module). */
export interface TraceAnalysis {
  type: TraceImageType;
  capture: TraceCapture;
  confidence: number;
  /** The type is unclear: the proposal is Brightness at the automatic Threshold. */
  low: boolean;
  /** `BRIGHTNESS` or `REMOVE_BACKGROUND`. */
  method: number;
  /** The Threshold as luma 0 to 255. */
  threshold: number;
  /** With Remove background: `#RRGGBB`, empty for a transparent background. */
  background: string;
  /** With Remove background: the Colour tolerance 0 to 100. */
  tolerance: number | null;
  reasons: readonly TraceReason[];
  /** Every measured property by name (reported, not shown in the line). */
  properties: Readonly<Record<string, number>>;
}

/** The analysis line's state (UX notes §3; Frage 83 B: no Suggested state). */
export type TraceAnalysisState =
  | { kind: "none" }
  | { kind: "analysing" }
  | { kind: "auto"; analysis: TraceAnalysis }
  | { kind: "custom"; analysis: TraceAnalysis }
  | { kind: "failed" }
  /** Stopped before it answered (Stop or Escape while "Analysing..." shows). */
  | { kind: "stopped" };

/** The button of the analysis row (always enabled, `ux-notes.md` §22.3). */
export const ANALYSE = "Analyse";

/** The line while the analysis runs (shown after 400 ms, like the trace). */
export const ANALYSING = "Analysing...";

/** The line of a failed analysis: the image keeps the automatic Threshold. */
export const ANALYSIS_FAILED = "Analysis failed · automatic Threshold used";

/** The line of an analysis stopped before it answered (Proposal, customer 2026-10-31): Analyse
 * or Retry runs it again. */
export const ANALYSIS_STOPPED = "Analysis stopped";

/** The line under low confidence (criterion 32; Proposal: names the method, not a preset, since
 * the analysis never sets the preset). */
export const TYPE_UNCLEAR = "Type unclear · Brightness with automatic Threshold";

/** The words of each image type. */
export const IMAGE_TYPE_WORDS: Readonly<Record<TraceImageType, string>> = {
  graphic: "Graphic",
  logo: "Logo",
  line_drawing: "Line drawing",
  sketch: "Sketch",
};

/** The words of each capture condition that is not digital. */
const CAPTURE_WORDS: Readonly<Partial<Record<TraceCapture, string>>> = {
  scan: "scan",
  paper_photo: "photo of paper",
};

/** The words of each reason that changed a value. */
const REASON_WORDS: Readonly<Partial<Record<TraceReason, string>>> = {
  pale_colours: "pale colours kept",
  transparent_background: "transparent background",
  shadow_left_out: "shadow left out",
};

/** The finding when nothing but the automatic Threshold was set. */
const AUTOMATIC_THRESHOLD = "automatic Threshold";

const TYPES = Object.keys(IMAGE_TYPE_WORDS);
const CAPTURES = ["digital", "scan", "paper_photo"];
const REASONS = ["pale_colours", "transparent_background", "shadow_left_out", "low_confidence"];

/** The analysis of the worker's JSON, or `null` when it is empty or unreadable (a failed
 * analysis). */
export function readAnalysis(json: string): TraceAnalysis | null {
  try {
    const raw = JSON.parse(json) as Record<string, unknown>;
    const number = (v: unknown) => (typeof v === "number" && Number.isFinite(v) ? v : null);
    const threshold = number(raw.threshold);
    if (!TYPES.includes(raw.type as string) || !CAPTURES.includes(raw.capture as string) || threshold === null) {
      return null;
    }
    const reasons = Array.isArray(raw.reasons) ? raw.reasons.filter((r): r is TraceReason => REASONS.includes(r as string)) : [];
    const properties: Record<string, number> = {};
    if (raw.properties && typeof raw.properties === "object") {
      for (const [name, value] of Object.entries(raw.properties as Record<string, unknown>)) {
        const v = number(value);
        if (v !== null) {
          properties[name] = v;
        }
      }
    }
    return {
      type: raw.type as TraceImageType,
      capture: raw.capture as TraceCapture,
      confidence: number(raw.confidence) ?? 0,
      low: raw.low === true,
      method: raw.method === REMOVE_BACKGROUND ? REMOVE_BACKGROUND : BRIGHTNESS,
      threshold: Math.min(255, Math.max(0, Math.round(threshold))),
      background: typeof raw.background === "string" ? raw.background : "",
      tolerance: number(raw.tolerance),
      reasons,
      properties,
    };
  } catch {
    return null;
  }
}

/** `values` with the analysed values of `analysis` (criterion 33): the method and its
 * background and Colour tolerance, the Threshold. With Pick colours the method stays (picked
 * colours are the maker's; Proposal) and only the Threshold is set. */
export function applyAnalysis(values: TraceValues, analysis: TraceAnalysis): TraceValues {
  const withThreshold = { ...values, threshold: analysis.threshold };
  if (values.method === PICK_COLOURS) {
    return withThreshold;
  }
  if (analysis.method === REMOVE_BACKGROUND) {
    return {
      ...withThreshold,
      method: REMOVE_BACKGROUND,
      backgroundAuto: true,
      background: analysis.background,
      backgroundTolerance: analysis.tolerance ?? values.backgroundTolerance,
    };
  }
  return { ...withThreshold, method: BRIGHTNESS };
}

/** The values whose edit makes the analysis line "Custom" (UX notes §3): the analysed ones. */
export const ANALYSED_FIELDS: readonly (keyof TraceValues)[] = [
  "method",
  "threshold",
  "backgroundTolerance",
  "backgroundAuto",
  "background",
];

/** What the analysis found and set, after the type: the capture, then each reason. */
function findings(analysis: TraceAnalysis): string[] {
  const words: string[] = [];
  const capture = CAPTURE_WORDS[analysis.capture];
  if (capture) {
    words.push(capture);
  }
  const set = analysis.reasons.map((r) => REASON_WORDS[r]).filter((w): w is string => w !== undefined);
  return [...words, ...(set.length > 0 ? set : [AUTOMATIC_THRESHOLD])];
}

/** The analysis line of `state` (UX notes §3, `role="status"`): "Line drawing · scan · automatic
 * Threshold", "Custom · Logo", "Type unclear · ...", "Analysing...", "Analysis failed · ...";
 * empty before the first image. */
export function analysisLine(state: TraceAnalysisState): string {
  switch (state.kind) {
    case "none":
      return "";
    case "analysing":
      return ANALYSING;
    case "failed":
      return ANALYSIS_FAILED;
    case "stopped":
      return ANALYSIS_STOPPED;
    case "custom":
      return `Custom · ${IMAGE_TYPE_WORDS[state.analysis.type]}`;
    case "auto":
      return state.analysis.low
        ? TYPE_UNCLEAR
        : [IMAGE_TYPE_WORDS[state.analysis.type], ...findings(state.analysis)].join(" · ");
  }
}
