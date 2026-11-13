/** The bar's settings of a trace part in the shape the image stores them (`specs/0114`, ADR 0061
 * §5): snake-case keys, the method and output by name, and read back. The session keeps them as
 * tool state for an untouched image; the first mask gesture and Add part store them, a settings
 * edit of a stored part writes them. */
import { BRIGHTNESS, MULTICOLOUR, PICK_COLOURS, REMOVE_BACKGROUND, SINGLE_COLOUR, type TraceChoice, type TraceValues } from "@/lib/traceText";

const METHODS: Record<number, string> = { [REMOVE_BACKGROUND]: "remove_background", [PICK_COLOURS]: "pick_colours" };

/** `choice` as the JSON of ADR 0061 §5. */
export function partSettingsJson(choice: TraceChoice): string {
  const v = choice.values;
  return JSON.stringify({
    threshold: v.threshold,
    smooth: v.smooth,
    tolerance: v.tolerance,
    specks: v.specks,
    corners: v.corners,
    upscale: v.upscale,
    denoise: v.denoise,
    invert: v.invert,
    ignore_background: v.ignoreBackground,
    recognise_text: v.recogniseText,
    recognise_shapes: v.recogniseShapes,
    method: METHODS[v.method] ?? "brightness",
    background_tolerance: v.backgroundTolerance,
    pick_tolerance: v.pickTolerance,
    background_auto: v.backgroundAuto,
    background: v.background === null ? null : v.background.toLowerCase(),
    swatches: v.swatches.map((c) => c.toLowerCase()),
    output: v.output === MULTICOLOUR ? "multi" : "single",
    colours: v.colours,
    fixed: v.fixed.map((c) => c.toLowerCase()),
    excluded: v.excluded.map((c) => c.toLowerCase()),
    kept_palette: v.keptPalette === null ? null : v.keptPalette.map((c) => c.toLowerCase()),
    preset: choice.preset,
    preset_edited: choice.edited,
  });
}

/** `choice` as a stored part keeps it (ADR 0061 §5): an automatic value is stored as such and
 * read again from the picture on return, so a Threshold the maker did not set (`makerThreshold`
 * false) is `null` (Auto), and with Remove background's Auto the background is `null`. */
export function storedPartSettingsJson(choice: TraceChoice, makerThreshold: boolean): string {
  const v = choice.values;
  return partSettingsJson({
    ...choice,
    values: { ...v, threshold: makerThreshold ? v.threshold : null, background: v.backgroundAuto ? null : v.background },
  });
}

const METHOD_OF: Record<string, number> = { brightness: BRIGHTNESS, remove_background: REMOVE_BACKGROUND, pick_colours: PICK_COLOURS };

/** The choice a part's stored settings (`trace_part_settings`, every key valid) give, over `base`
 * for anything not read; `null` for no stored settings (the implicit part of an untouched image). */
export function readPartChoice(json: string, base: TraceChoice): TraceChoice | null {
  if (json === "") {
    return null;
  }
  let raw: Record<string, unknown>;
  try {
    raw = JSON.parse(json) as Record<string, unknown>;
  } catch {
    return null;
  }
  const num = (key: string, fallback: number) => (typeof raw[key] === "number" ? (raw[key] as number) : fallback);
  const flag = (key: string, fallback: boolean) => (typeof raw[key] === "boolean" ? (raw[key] as boolean) : fallback);
  const list = (key: string): readonly string[] | null =>
    Array.isArray(raw[key]) ? (raw[key] as unknown[]).filter((c): c is string => typeof c === "string").map((c) => c.toUpperCase()) : null;
  const b = base.values;
  const background = raw.background === null ? null : typeof raw.background === "string" ? (raw.background as string).toUpperCase() : b.background;
  const values: TraceValues = {
    ...b,
    threshold: raw.threshold === null ? null : num("threshold", b.threshold ?? 0),
    smooth: num("smooth", b.smooth),
    tolerance: num("tolerance", b.tolerance),
    specks: num("specks", b.specks),
    corners: num("corners", b.corners),
    upscale: num("upscale", b.upscale),
    denoise: num("denoise", b.denoise),
    invert: flag("invert", b.invert),
    ignoreBackground: flag("ignore_background", b.ignoreBackground),
    recogniseText: flag("recognise_text", b.recogniseText),
    recogniseShapes: flag("recognise_shapes", b.recogniseShapes),
    method: METHOD_OF[String(raw.method)] ?? b.method,
    backgroundTolerance: num("background_tolerance", b.backgroundTolerance),
    pickTolerance: num("pick_tolerance", b.pickTolerance),
    backgroundAuto: flag("background_auto", b.backgroundAuto),
    background,
    swatches: list("swatches") ?? b.swatches,
    output: raw.output === "multi" ? MULTICOLOUR : raw.output === "single" ? SINGLE_COLOUR : b.output,
    colours: raw.colours === null ? null : num("colours", b.colours ?? 0),
    fixed: list("fixed") ?? b.fixed,
    excluded: list("excluded") ?? b.excluded,
    keptPalette: raw.kept_palette === null ? null : (list("kept_palette") ?? b.keptPalette),
  };
  return {
    preset: typeof raw.preset === "string" ? raw.preset : base.preset,
    edited: flag("preset_edited", base.edited),
    values,
  };
}
