// The Style panel's ready-made width profiles (`specs/0110-pen-modes-variable-stroke` criterion 5):
// one icon button each, the name in tooltip and accessible name; the session writes the points.

/** A preset code as the session reads it. */
export type WidthPresetCode = "uniform" | "taper-start" | "taper-end" | "taper-both";

/** The presets in panel order. */
export const WIDTH_PRESETS: readonly { code: WidthPresetCode; label: string; tooltip: string }[] = [
  { code: "uniform", label: "Uniform", tooltip: "Uniform: the stroke width everywhere" },
  { code: "taper-start", label: "Taper start", tooltip: "Taper start: thin at the start" },
  { code: "taper-end", label: "Taper end", tooltip: "Taper end: thin at the end" },
  { code: "taper-both", label: "Taper both", tooltip: "Taper both: thin at both ends" },
];

/** The outline of each preset's glyph in a 24 × 24 box: a band from left to right. */
export const WIDTH_PRESET_GLYPHS: Record<WidthPresetCode, string> = {
  uniform: "M3 10h18v4H3z",
  "taper-start": "M3 12 21 9v6z",
  "taper-end": "M3 9l18 3-18 3z",
  "taper-both": "M3 12q9-7 18 0-9 7-18 0z",
};
