// The measured width of a field's inside label (`docs/ui-elements.md` §10).

/** The width of `label` in the 12 px label font, px; 0 where nothing can measure it (tests). */
export function fieldLabelWidth(label: string): number {
  const context =
    typeof document === "undefined" ? null : document.createElement("canvas").getContext?.("2d");
  if (!context) {
    return 0;
  }
  context.font = `12px ${getComputedStyle(document.body).fontFamily}`;
  return context.measureText(label).width;
}
