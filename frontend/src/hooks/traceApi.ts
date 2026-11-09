import type { TraceAnalysisState } from "@/lib/traceAnalysis";
import type { TracePartInfo } from "@/lib/traceParts";

import type { HsvTriple } from "./useStylePanel";
import type { TracePaletteApi } from "./useTracePalette";
import type {
  TraceChoice,
  TraceParam,
  TracePreset,
  TraceSlot,
  TraceStatusText,
  TraceValues,
} from "@/lib/traceText";

/** The Trace tool in the frontend (`specs/0025-raster-trace` criteria 1 to 5, 9 to 13, 15, 16,
 * 46, 52 to 54): the bar's state, the worker jobs with their timers, the progress slot, Create
 * paths. The canvas always shows Overlay (`specs/0089` UX notes §24.1).
 * Every document decision is the session's; the trace runs in the worker. */
export interface TraceApi {
  /** The Trace tool is the active tool. */
  active: boolean;
  /** The Select bar shows the Trace button (only images selected). */
  available: boolean;
  /** What is selected, to the tool. */
  selection: "empty" | "other" | "images" | "mixed";
  /** The Mask group (`specs/0089` UX notes §24.4, always drawn): the mask paths selected with the
   * image (0 for none) and the chosen mode, Inside on every entry. */
  mask: { paths: number; outside: boolean };
  /** Inside (`false`) or Outside, not remembered (criterion 17). */
  setMaskOutside: (outside: boolean) => void;
  /** Pick mask paths is armed (`specs/0094` slice 1b, UX notes §11). */
  maskPick: boolean;
  /** Arms or disarms Pick mask paths. */
  setMaskPick: (armed: boolean) => void;
  /** The last pick hit a text, a sketch or a second image: the notice row says so (criterion 38). */
  maskRefused: boolean;
  /** Clear mask: only the image stays selected (criterion 40). */
  clearMask: () => void;
  /** The parts' chips (`specs/0094` slice 2, UX notes §3); one whole-image part at least. */
  parts: TracePartInfo[];
  /** A chip pick: the part becomes active with its settings and mask paths. */
  selectPart: (index: number) => void;
  /** Add part: a copy of the active part's settings, no mask paths. */
  addPart: () => void;
  /** Remove part `index` (after the inline confirm). */
  removePart: (index: number) => void;
  /** "Tracing 1 of 3 images", or empty for one image. */
  imagesLabel: string;
  params: readonly TraceParam[];
  /** The presets in menu order (criterion 59). */
  presets: readonly TracePreset[];
  values: TraceValues;
  /** The remembered choice: the preset the values came from and whether they were edited. */
  choice: TraceChoice;
  /** The Preset button's text: the preset's name or "Custom" (criterion 40). */
  presetLabel: string;
  /** The Advanced block is open (criterion 41). */
  advancedOpen: boolean;
  /** The notice row's line, muted line and message, the counts for the hidden status region, and
   * whether Enter may create (criteria 11, 71). */
  status: TraceStatusText;
  /** The result shown belongs to an older trace: a trace runs or was stopped (UX notes 20.4). */
  countsStale: boolean;
  /** What the progress slot of row 1 shows (criteria 10, 53, 54; UX notes 21.2). */
  slot: TraceSlot;
  /** Trace again after a stop, or Retry after a failure. */
  retrace: () => void;
  /** Create paths waits for full-size traces. */
  creating: boolean;
  /** A value from a field; `dragging` traces the reduced copy only (criterion 10). */
  setValue: <K extends keyof TraceValues>(name: K, value: TraceValues[K], dragging?: boolean) => void;
  /** A pick in the Preset menu (criteria 40, 60, 66): every preset value at once. */
  pick: (preset: TracePreset) => void;
  /** Opens or closes the Advanced block (remembered). */
  setAdvancedOpen: (open: boolean) => void;
  /** The end of a drag: the full-size trace follows. */
  endDrag: () => void;
  /** Stop (criterion 10): ends the job; the preview is out of date. */
  stop: () => void;
  /** Create paths (criteria 12, 13, 46). */
  create: () => void;
  /** Enter and Escape on the canvas in the tool: `true` when the key was taken. */
  keyDown: (event: KeyboardEvent) => boolean;
  /** A pick in the Method menu (`specs/0089` criterion 14): only the method's own fields change. */
  setMethod: (method: number) => void;
  /** The Colour tolerance of the current method. */
  setColourTolerance: (value: number, dragging?: boolean) => void;
  /** Remove background's Auto: the background from the image border again. */
  setBackgroundAuto: () => void;
  /** What the pipette is armed for: "background", "colours" or "" (UX notes §6). */
  pickMode: string;
  /** Arms the pipette for the current method, or disarms it. */
  setPicking: (armed: boolean) => void;
  /** Disarms an armed pipette as Escape does; `false` when it was not armed (`ux-notes.md` §25.4). */
  disarmPick: () => boolean;
  /** Takes a colour out of Pick colours' picked colours (`ux-notes.md` §25.4): not a history step. */
  removePicked: (hex: string) => void;
  /** The automatic background of the shown image (`#RRGGBB`, empty: transparent; `null` before
   * it is read). */
  suggestedBackground: string | null;
  /** The shown image's analysis line state (`specs/0089` slice F, UX notes §3). */
  analysis: TraceAnalysisState;
  /** Analyse: the analysed values back to the shown image's analysis. */
  analyse: () => void;
  /** What the bar's hidden status region says (`specs/0089` UX notes §24.5): the analysis, the
   * mask, the counts, the recognised text and shapes, the palette state, an empty part, the
   * picked colour removed and the empty set (§25.3, §25.4). */
  announcement: string;
  /** Why the Trace button is `aria-disabled`, or empty (`ux-notes.md` §22.2). */
  traceBlocked: string;
  /** The palette of a multicolour trace (`specs/0089` slice M). */
  palette: TracePaletteApi;
  /** The Recolour picker's conversions (Rust's rounding, as the Style section's). */
  hsvOf: (rgb: number) => HsvTriple;
  rgbOf: (hue: number, saturation: number, value: number) => number;
}
