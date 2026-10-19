/** The words of the Select bar's ellipse group and Curve field
 * (`specs/0021-ellipse-arcs-and-shaping/` UX notes, "Select bar"). */

export type ArcKindName = "pie" | "arc" | "chord";

export const ARC_KIND_TEXT: Record<ArcKindName, { name: string; line: string }> = {
  pie: { name: "Pie", line: "The arc and straight lines to the centre." },
  arc: { name: "Arc", line: "The arc only. Open." },
  chord: { name: "Chord", line: "The arc closed by a straight line." },
};

export const ARC_TEXT = {
  whole: { name: "Whole ellipse", line: "Remove the arc." },
  angle: { line: "Angle from the right-hand axis, clockwise." },
  curve: { line: "100 % circle, 0 % diamond, -100 % star." },
} as const;

/** The second muted line of a greyed control, by the session's reason code. */
export const ARC_GREY_LINE: Record<string, string> = {
  curve: "Not available while the curve is not 100%",
  "no-arc": "Not available on a whole ellipse",
  arc: "Not available while the ellipse has an arc",
};
