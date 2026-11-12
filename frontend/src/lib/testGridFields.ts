/**
 * The fields of the Grid block (`specs/0028-material-test-library` `ux-notes.md` 6.2, 14): which
 * field is typed under which name, what it shows, and the Up and Down step. Pure functions over the
 * decoded view (`testGridView.ts`); the session holds the ranges and answers with the chip.
 */
import type { SetKind } from "./operationSettingsView.ts";
import type { TestGridView } from "./testGridView.ts";

/** A typed field of the block; `key` is the name the session's `set_test_grid_field` takes. */
export interface GridFieldSpec {
  key: string;
  /** The text inside the field's left edge, or the heading's "From", "To". */
  short: string;
  /** The accessible name (ux-notes 14). */
  name: string;
  /** The unit inside the field, empty for none. */
  unit: string;
  width: number;
}

const power = (key: string, short: string, name: string): GridFieldSpec => ({ key, short, name, unit: "", width: 78 });

/** Power group: From, To, Columns. */
export const POWER_FIELDS: readonly GridFieldSpec[] = [
  power("power_from", "From", "Power from, percent"),
  power("power_to", "To", "Power to, percent"),
  power("columns", "Columns", "Columns"),
];
/** Speed group: From, To, Rows. */
export const SPEED_FIELDS: readonly GridFieldSpec[] = [
  power("speed_from", "From", "Speed from, millimetres per minute"),
  power("speed_to", "To", "Speed to, millimetres per minute"),
  power("rows", "Rows", "Rows"),
];
/** Cell group: Size, Gap. */
export const CELL_FIELDS: readonly GridFieldSpec[] = [
  { key: "cell", short: "Size", name: "Cell size, millimetres", unit: "", width: 118 },
  { key: "gap", short: "Gap", name: "Gap, millimetres", unit: "", width: 118 },
];
/** Label fields, shown while Labels is on. */
export const LABEL_FIELDS: readonly GridFieldSpec[] = [
  { key: "label_power", short: "", name: "Label power, percent", unit: "%", width: 118 },
  { key: "label_speed", short: "", name: "Label speed, millimetres per minute", unit: "mm/min", width: 118 },
  { key: "label_passes", short: "", name: "Label passes", unit: "×", width: 118 },
];

/** The kinds of the Kind radiogroup, in order (ux-notes 6.2). */
export const GRID_KINDS: readonly { kind: SetKind; label: string }[] = [
  { kind: "cut", label: "Cut" },
  { kind: "score", label: "Score" },
  { kind: "engrave", label: "Engrave" },
];

/** A number as a field shows it: no trailing zeros. */
function plain(value: number): string {
  return String(Math.round(value * 10) / 10);
}

/** The committed text of field `key`. A value the grid did not take stays in the field's own text with its chip until Escape. */
export function fieldShown(view: TestGridView, key: string): string {
  const f = view.fields;
  const l = view.labels;
  switch (key) {
    case "columns":
      return String(f.columns);
    case "rows":
      return String(f.rows);
    case "power_from":
      return plain(f.powerFromPct);
    case "power_to":
      return plain(f.powerToPct);
    case "speed_from":
      return String(f.speedFromMmMin);
    case "speed_to":
      return String(f.speedToMmMin);
    case "cell":
      return plain(f.cellMm);
    case "gap":
      return plain(f.gapMm);
    case "label_power":
      return plain(l.powerPct);
    case "label_speed":
      return String(l.speedMmMin);
    case "label_passes":
      return String(l.passes);
    default:
      return "";
  }
}

/** The size line under the head: "Generic laser · 75 × 62 mm". */
export function sizeLine(view: TestGridView): string {
  return `${view.machine} · ${plain(view.widthMm)} × ${plain(view.heightMm)} mm`;
}

/** Line 2 of a Test grid row: "Cut test grid · 5 × 5 cells" (ux-notes 5). */
export function gridRowLine(kindWord: string, columns: number, rows: number, outputOn: boolean): string {
  const size = columns === 0 ? "no grid" : `${columns} × ${rows} cells`;
  return `${kindWord} test grid · ${size}${outputOn ? "" : " · Output off"}`;
}
