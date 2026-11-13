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

/** Part `index`'s pixels of `id` for the worker, cut to its clip (`specs/0094` criterion 31);
 * `specks` is the part's Ignore below in mm and `upscaleAuto` whether its Upscale is Auto. */
export function partPixelsOf(session: WasmSession, id: string, index: number, reduced: boolean, specks: number, upscaleAuto: boolean): TracePixels | null {
  const raw = session.trace_part_source(id, index, reduced, specks, upscaleAuto);
  if (!raw) {
    return null;
  }
  const pixels = { width: raw.width, height: raw.height, rgba: raw.rgba, map: Array.from(raw.map) };
  raw.free();
  return pixels;
}

/** The parts' wires of `id` joined by the session into the preview (with `keep` also the result
 * Create paths writes); the joined header numbers, empty when a wire is no result. */
export function joinWires(session: WasmSession, id: string, wires: (Float64Array | null)[], keep: boolean): Float64Array {
  const lengths = new Uint32Array(wires.map((w) => w?.length ?? 0));
  const all = new Float64Array(lengths.reduce((n, l) => n + l, 0));
  let at = 0;
  for (const wire of wires) {
    if (wire) {
      all.set(wire, at);
      at += wire.length;
    }
  }
  return session.set_trace_joined(id, all, lengths, keep);
}

export interface ViewState {
  selection: "empty" | "other" | "images";
  images: string[];
  shown: string;
  /** The active part's stored regions (`specs/0114`). */
  maskRegions: number;
  /** The regions cover none of the image (criterion 7). */
  maskOutsideImage: boolean;
  /** The regions could not be combined (the kernel refused). */
  maskFailed: boolean;
  /** The region for the worker's settings (`{"outside", "region"}`), empty without one. */
  mask: string;
  /** The parts' chips JSON (`specs/0094` slice 2, `traceParts.readParts`). */
  parts: string;
  /** The active part's stored settings JSON (`specs/0114` slice 3), empty for an untouched image:
   * an undo or redo that changes them reshows them. */
  partSettings: string;
}

export const NO_VIEW: ViewState = { selection: "empty", images: [], shown: "", maskRegions: 0, maskOutsideImage: false, maskFailed: false, mask: "", parts: "", partSettings: "" };

/** The index of the active chip of the session's parts JSON, -1 when none can be read. */
function readActiveIndex(json: string): number {
  try {
    const parts = JSON.parse(json) as { active?: boolean }[];
    return Array.isArray(parts) ? parts.findIndex((p) => p.active === true) : -1;
  } catch {
    return -1;
  }
}

export function readView(session: WasmSession | null): ViewState {
  const raw = session?.trace_view();
  if (!raw) {
    return NO_VIEW;
  }
  const parts = session ? session.trace_parts() : "";
  const active = Math.max(0, readActiveIndex(parts));
  const view = {
    selection: raw.selection as ViewState["selection"],
    images: raw.images,
    shown: raw.shown,
    maskRegions: raw.mask_regions,
    maskOutsideImage: raw.mask_outside_image,
    maskFailed: raw.mask_failed,
    mask: raw.mask_regions > 0 && session ? session.trace_mask_json() : "",
    parts,
    partSettings: session && raw.images.length === 1 ? session.trace_part_settings(active) : "",
  };
  raw.free();
  return view;
}
