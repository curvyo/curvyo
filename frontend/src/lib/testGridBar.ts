/**
 * The field logic of the Test grid bar (`specs/0112-test-grid-and-materials` `ux-notes.md` 4):
 * names, units, steps, the text a field shows and the dropdown's filter. Pure functions over the
 * decoded view (`testGridView.ts`); the session holds the ranges and answers with the cause.
 */
import type { AxisFields, BarMaterial, Param, TestGridBarView } from "./testGridView.ts";

/** The word a parameter has on a segment and in a field. */
export const PARAM_WORD: Record<Param, string> = {
  power: "Power",
  speed: "Speed",
  passes: "Passes",
  interval: "Interval",
};

/** The unit inside a parameter's field. */
export const PARAM_UNIT: Record<Param, string> = {
  power: "%",
  speed: "mm/min",
  passes: "×",
  interval: "mm",
};

/** The width of a shared field in px (4.1). */
export const SHARED_WIDTH: Record<Param, number> = { power: 104, speed: 160, passes: 88, interval: 120 };

/** The Up and Down step of a parameter (4.3); Shift takes ten times. */
export const PARAM_STEP: Record<Param, number> = { power: 1, speed: 100, passes: 1, interval: 0.01 };

/** The step of the Cell size field. */
export const CELL_STEP = 1;

/** The dropdown shows a Filter field above this many materials (4.4). */
export const MATERIAL_FILTER_ABOVE = 8;

/** The toolbar's accessible name: "Test grid K7Q2M", "Test grid" before a grid exists. */
export function barName(view: TestGridBarView): string {
  return view.id === null ? "Test grid" : `Test grid ${view.id}`;
}

/** A number as a field shows it: at most two decimals, no trailing zeros. */
export function plainNumber(value: number): string {
  return String(Math.round(value * 100) / 100);
}

/** The text of field `key`: the typed text while the grid did not take it, else the number. */
export function fieldText(view: TestGridBarView, key: string, value: number): string {
  return view.invalid !== null && view.invalid.field === key ? view.invalid.typed : plainNumber(value);
}

/** The cause shown under field `key`, `null` when the field is valid. */
export function causeOf(view: TestGridBarView, key: string): string | null {
  return view.invalid !== null && view.invalid.field === key ? view.invalid.cause : null;
}

/** The bed notice under the bar (a grid wider or higher than the bed), when no field owns it. */
export function bedNotice(view: TestGridBarView): string | null {
  return view.fit;
}

/** The text `typed` moved one step: `direction` is +1 or -1, `step` the parameter's step. */
export function steppedText(typed: string, step: number, direction: 1 | -1, shift: boolean): string | null {
  const current = Number(typed.trim().replace(",", "."));
  if (typed.trim() === "" || !Number.isFinite(current)) return null;
  const size = shift ? step * 10 : step;
  const decimals = Math.max(2, String(step).split(".")[1]?.length ?? 0);
  return plainNumber(Number((current + direction * size).toFixed(decimals)));
}

/** The names of the fields of one axis unit, as the session's `set_test_grid_field` takes them. */
export function axisKeys(side: "columns" | "rows"): { min: string; max: string; count: string } {
  return { min: `${side}_min`, max: `${side}_max`, count: `${side}_count` };
}

/** The accessible name of an axis field: "Columns Min, %". */
export function axisFieldName(side: "columns" | "rows", part: "Min" | "Max" | "Count", axis: AxisFields): string {
  const head = side === "columns" ? "Columns" : "Rows";
  return part === "Count" ? `${head} count` : `${head} ${part}, ${PARAM_UNIT[axis.param]}`;
}

/** The unit inside an axis field (Count has none). */
export function axisUnit(part: "Min" | "Max" | "Count", axis: AxisFields): string {
  return part === "Count" ? "" : PARAM_UNIT[axis.param];
}

/** The step of an axis field. */
export function axisStep(part: "Min" | "Max" | "Count", axis: AxisFields): number {
  return part === "Count" ? 1 : PARAM_STEP[axis.param];
}

/** The pill's width the bar takes: a unit (Columns, Rows) is one wrap item. A unit wider than
 * `available` breaks once, after its parameter (G5). */
export function unitBreaks(unitWidth: number, available: number): boolean {
  return unitWidth > available;
}

/** The dropdown's rows: the materials whose name holds `query` (case-insensitive). */
export function filterMaterials(materials: readonly BarMaterial[], query: string): BarMaterial[] {
  const needle = query.trim().toLowerCase();
  return needle === "" ? [...materials] : materials.filter((one) => one.name.toLowerCase().includes(needle));
}

/** The thickness at the right of a dropdown row: "3.0 mm", empty for none. */
export function thicknessLabel(material: BarMaterial): string {
  return material.thicknessMm === null ? "" : `${material.thicknessMm.toFixed(1)} mm`;
}

/** The text of the material button: the picked name, or "Material". */
export function materialButtonText(view: TestGridBarView): { text: string; picked: boolean } {
  return view.material === null ? { text: "Material", picked: false } : { text: view.material.name, picked: true };
}

/** The shared values of row 1 by parameter. */
export function sharedValue(view: TestGridBarView, param: Param): number | null {
  return view.shared.find((one) => one.param === param)?.value ?? null;
}
