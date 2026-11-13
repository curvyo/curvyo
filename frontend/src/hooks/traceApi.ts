import type { TracePartInfo } from "@/lib/traceParts";

import type { TracePaletteApi } from "./useTracePalette";
import type {
  TraceChoice,
  TraceParam,
  TracePreset,
  TraceSlot,
  TraceValues,
} from "@/lib/traceText";

/** A region tool of the Mask group (`specs/0114`): Rectangle, Lasso or none. */
export type MaskTool = "none" | "rect" | "lasso";

/** The Trace tool in the frontend (`specs/0025-raster-trace` criteria 1 to 5, 9 to 13, 15, 16,
 * 46, 52 to 54): the bar's state, the worker jobs with their timers, the progress slot, Create
 * paths. The canvas always shows Overlay (`specs/0089` UX notes §24.1).
 * Every document decision is the session's; the trace runs in the worker. */
export interface TraceApi {
  /** The Trace tool is the active tool. */
  active: boolean;
  /** The Select bar shows the Trace button (an image or image-root clone in the selection). */
  available: boolean;
  /** What is selected, to the tool. */
  selection: "empty" | "other" | "images";
  /** The Mask group (`specs/0114` UX notes §1): the active part's regions and its mode. */
  mask: { regions: number; outside: boolean };
  /** Inside (`false`) or Outside: a step on a stored part (`specs/0114`), tool state before. */
  setMaskOutside: (outside: boolean) => void;
  /** The pressed region tool of the Mask group (`specs/0114` Behaviour 1). */
  maskTool: MaskTool;
  /** Presses a region tool; `"none"` releases it. */
  setMaskTool: (tool: MaskTool) => void;
  /** Subtract is pressed: a region drawn now is taken out of the mask. */
  subtract: boolean;
  setSubtract: (on: boolean) => void;
  /** The image or the clone's root is locked: the Mask group's controls are unavailable
   * (`specs/0114` UX notes §8). */
  maskLocked: boolean;
  /** Two or more images are selected: the Parts and Mask unit is not drawn (Frage 11 A). */
  multiImage: boolean;
  /** Clear mask: the active part's regions go in one step (`specs/0114` criterion 20). */
  clearMask: () => void;
  /** The parts' chips (`specs/0094` slice 2, UX notes §3); one whole-image part at least. */
  parts: TracePartInfo[];
  /** A chip pick: the part becomes active with its settings and regions. */
  selectPart: (index: number) => void;
  /** Add part: a copy of the active part's settings, no regions. */
  addPart: () => void;
  /** Remove part `index` (after the inline confirm). */
  removePart: (index: number) => void;
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
  /** Arms the pipette for the current method (`palette`: the Multicolour palette's, `ux-notes.md`
   * §27), or disarms it. */
  setPicking: (armed: boolean, palette?: boolean) => void;
  /** Disarms an armed pipette as Escape does; `false` when it was not armed (`ux-notes.md` §25.4). */
  disarmPick: () => boolean;
  /** Takes a colour out of Pick colours' picked colours (`ux-notes.md` §25.4): not a history step. */
  removePicked: (hex: string) => void;
  /** The automatic background of the shown image (`#RRGGBB`, empty: transparent; `null` before
   * it is read). */
  suggestedBackground: string | null;
  /** Analyse: the analysed values back to the shown image's analysis. */
  analyse: () => void;
  /** What the bar's hidden status region says (`specs/0089` UX notes §24.5): the analysis, the
   * mask, the counts, the notices the bar does not draw (§27), the recognised text and shapes, the
   * palette state, an empty part, the picked colour removed and the empty set (§25.3, §25.4). */
  announcement: string;
  /** Why the Trace button is `aria-disabled`, or empty (`ux-notes.md` §22.2). */
  traceBlocked: string;
  /** The palette of a multicolour trace (`specs/0089` slice M). */
  palette: TracePaletteApi;
}
