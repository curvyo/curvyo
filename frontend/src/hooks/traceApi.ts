import type {
  TraceChoice,
  TraceParam,
  TracePreset,
  TraceSlot,
  TraceStatusText,
  TraceValues,
  TraceViewMode,
} from "@/lib/traceText";

export type ShowNotice = (text: string, ms?: number, source?: "canvas" | "panel", kind?: "status" | "alert") => void;

/** The Trace tool in the frontend (`specs/0025-raster-trace` criteria 1 to 5, 9 to 13, 15, 16,
 * 46, 51 to 54): the bar's state, the worker jobs with their timers, the progress slot, the
 * view and the peek, Create paths and its notice.
 * Every document decision is the session's; the trace runs in the worker. */
export interface TraceApi {
  /** The Trace tool is the active tool. */
  active: boolean;
  /** The Select bar shows the Trace button (only images selected). */
  available: boolean;
  /** What is selected, to the tool. */
  selection: "empty" | "other" | "images";
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
  /** The state row's line and muted line, the counts of the Advanced block, and whether Enter
   * may create (criteria 11, 71). */
  status: TraceStatusText;
  /** The counts belong to an older result: a trace runs or was stopped (UX notes 20.4). */
  countsStale: boolean;
  /** What the progress slot of row 1 shows (criteria 10, 53, 54; UX notes 21.2). */
  slot: TraceSlot;
  /** The view on the canvas as the View pill shows it (criterion 51). */
  viewMode: TraceViewMode;
  /** A segment of the View pill. */
  setViewMode: (mode: TraceViewMode) => void;
  /** Trace again after a stop, or Retry after a failure. */
  retrace: () => void;
  /** Create paths waits for full-size traces. */
  creating: boolean;
  /** Working-image pixels per millimetre of the shown image, for the Specks readout. */
  pxPerMm: number;
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
  /** Takes a picked colour out of Pick colours. */
  removeSwatch: (hex: string) => void;
  /** What the pipette is armed for: "background", "colours" or "" (UX notes §6). */
  pickMode: string;
  /** Arms the pipette for the current method, or disarms it. */
  setPicking: (armed: boolean) => void;
  /** The automatic background of the shown image (`#RRGGBB`, empty: transparent; `null` before
   * it is read). */
  suggestedBackground: string | null;
}
