/** One printed text of a plotter page's mat (canvas CSS px). */
export interface MatLabel {
  text: string;
  x: number;
  y: number;
  /** The digit height in CSS px: the mat's 3 mm at the current zoom. */
  size: number;
  /** Turned 180 degrees, as the inch numbers are printed. */
  turned: boolean;
}

function isLabel(value: unknown): value is MatLabel {
  if (typeof value !== "object" || value === null) return false;
  const v = value as Record<string, unknown>;
  return (
    typeof v.text === "string" &&
    typeof v.x === "number" &&
    typeof v.y === "number" &&
    typeof v.size === "number" &&
    typeof v.turned === "boolean"
  );
}

/** The labels of the session's `plotter_mat_labels_json`; none for anything else. */
export function readMatLabels(text: string): MatLabel[] {
  try {
    const parsed: unknown = JSON.parse(text);
    return Array.isArray(parsed) ? parsed.filter(isLabel) : [];
  } catch {
    return [];
  }
}
