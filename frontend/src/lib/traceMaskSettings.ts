/** The bar's settings of a trace part in the shape the image stores them (`specs/0114`, ADR 0061
 * §5): snake-case keys, the method and output by name. The session keeps them as tool state; the
 * first mask gesture on an untouched image and Add part store them. */
import { MULTICOLOUR, PICK_COLOURS, REMOVE_BACKGROUND, type TraceChoice } from "@/lib/traceText";

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
