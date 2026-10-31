/** Reads what the Trace tool needs from the wasm module: its schema, the pixels and the view. */
import type { WasmSession } from "@/lib/editorSession";
import {
  MULTICOLOUR,
  PICK_COLOURS,
  REMOVE_BACKGROUND,
  SINGLE_COLOUR,
  TRACE_PARAMS_FALLBACK,
  TRACE_PRESETS_FALLBACK,
  type TraceParam,
  type TracePreset,
} from "@/lib/traceText";
import type { TracePixels } from "@/lib/traceJobs";
import { trace_params, trace_presets } from "@/wasm-bindings/curvyo_editor_wasm.js";

/** The settings schema from the wasm module, or the copy in `traceText` before it loads. */
export function loadParams(): readonly TraceParam[] {
  try {
    const parsed = JSON.parse(trace_params()) as TraceParam[];
    return parsed.length > 0 ? parsed : TRACE_PARAMS_FALLBACK;
  } catch {
    return TRACE_PARAMS_FALLBACK;
  }
}

/** The presets from the wasm module, or the copy in `traceText` before it loads. */
export function loadPresets(): readonly TracePreset[] {
  try {
    const parsed = JSON.parse(trace_presets()) as TracePreset[];
    return parsed.length > 0 ? parsed : TRACE_PRESETS_FALLBACK;
  } catch {
    return TRACE_PRESETS_FALLBACK;
  }
}

/** A method's pipette (Frage 89 A): Remove background takes one pick, Pick colours stays armed,
 * Brightness has one only in Multicolour, for the palette (stays armed, `specs/0089` UX notes §7). */
export function pickModeOf(method: number, output = SINGLE_COLOUR): string {
  if (method === REMOVE_BACKGROUND) {
    return "background";
  }
  return method === PICK_COLOURS || output === MULTICOLOUR ? "colours" : "";
}

/** The pixels of `id` for the worker, or `null` while the picture is not readable. */
export function pixelsOf(session: WasmSession, id: string, reduced: boolean): TracePixels | null {
  const raw = session.trace_source(id, reduced);
  if (!raw) {
    return null;
  }
  const pixels = { width: raw.width, height: raw.height, rgba: raw.rgba, map: Array.from(raw.map) };
  raw.free();
  return pixels;
}

export interface ViewState {
  selection: "empty" | "other" | "images";
  images: string[];
  shown: string;
}

export const NO_VIEW: ViewState = { selection: "empty", images: [], shown: "" };

export function readView(session: WasmSession | null): ViewState {
  const raw = session?.trace_view();
  if (!raw) {
    return NO_VIEW;
  }
  const view = { selection: raw.selection as ViewState["selection"], images: raw.images, shown: raw.shown };
  raw.free();
  return view;
}
