/**
 * What each slider and slider-like value field is called (`specs/0047-transform-polish/`
 * criterion 58): one table, so the accessible name and the first line of the tooltip are the same
 * text and a test can check that none is empty. The second line of a tooltip is the gesture help
 * of its kind.
 */

/** The five value fields with a bar (`ValueField`). */
export const VALUE_FIELD_NAMES = {
  strokeWidth: "Stroke width",
  strokeOpacity: "Stroke opacity",
  fillOpacity: "Fill opacity",
  markerCount: "Marker count",
  backgroundOpacity: "Background opacity",
} as const;

/** The two ratio sliders (`RatioSlider`): the selected stars' and the tool setting for new ones. */
export const RATIO_SLIDER_NAMES = {
  selectedStars: "Star inner ratio",
  newStars: "Star inner ratio of new stars",
} as const;

/** The two colour sliders of one paint ("Stroke", "Fill", "Background"). */
export const COLOUR_SLIDER_KINDS = {
  saturationValue: "saturation and value",
  hue: "hue",
} as const;

/** The name of a colour slider: "Stroke hue", "Fill saturation and value". */
export function colourSliderName(
  paint: string,
  kind: keyof typeof COLOUR_SLIDER_KINDS,
): string {
  return `${paint} ${COLOUR_SLIDER_KINDS[kind]}`;
}

/** The gesture help, the second line of a tooltip. */
export const HELP_VALUE_FIELD =
  "Click to set, double-click to type, drag to change. Shift: coarse. Ctrl: fine. Ctrl+Backspace: reset.";
export const HELP_RATIO = "Click to set, double-click to type";
export const HELP_COLOUR = "Click to set, double-click to type the hex value";

/** The first line of a tooltip: the name, and the current value when there is one to show
 * ("Stroke width: 0.50 mm"). A mixed value reads "Mixed". */
export function tipHeading(name: string, valueText: string | null): string {
  return valueText ? `${name}: ${valueText}` : name;
}

/** Every name in the table, for the test that none is empty. */
export function allSliderNames(): string[] {
  const paints = ["Stroke", "Fill", "Background"];
  const kinds = Object.keys(COLOUR_SLIDER_KINDS) as Array<keyof typeof COLOUR_SLIDER_KINDS>;
  return [
    ...Object.values(VALUE_FIELD_NAMES),
    ...Object.values(RATIO_SLIDER_NAMES),
    ...paints.flatMap((paint) => kinds.map((kind) => colourSliderName(paint, kind))),
  ];
}
