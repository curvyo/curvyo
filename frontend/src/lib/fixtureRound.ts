/**
 * The unfolded body of a pin or a screw (`specs/0113-fixture-sets-and-pins` `ux-notes.md` section
 * 6.1): which fields and toggles show, with their keys, labels and limits. Pure; the session clamps
 * a typed value and sends the notice, so the limits here only say what the field shows.
 */
import type { FixtureKind, RoleToggle, RoundValues } from "./fixturesView.ts";

/** One number field of the body. */
export interface RoundField {
  /** The key `set_fixture_grid_value` takes. */
  key: "diameter_mm" | "columns" | "rows" | "column_spacing_mm" | "row_spacing_mm" | "mark_mm";
  label: string;
  unit: string;
  value: number;
  /** A whole number (a count): typed decimals are rounded. */
  whole: boolean;
  /** Shown greyed out and not editable: a Spacing while its count is one (customer, 2026-11-16). */
  disabled?: boolean;
}

/** One toggle of the Jobs part, with its Size field while it is on. */
export interface JobToggle {
  /** The role job whose Output the toggle is. */
  toggle: RoleToggle;
  label: "Centre mark" | "Diameters";
  tooltip: string;
  /** The Centre mark's Size; `null` for Diameters and while off. */
  size: RoundField | null;
}

/** What the body shows for a pin or a screw. */
export interface RoundBody {
  /** The Diameter field; `null` for a screw. */
  diameter: RoundField | null;
  /** Columns and its Spacing, the Spacing greyed out at one. */
  columns: RoundField[];
  rows: RoundField[];
  /** The Jobs toggles; empty for an old screw without role jobs (criterion 39). */
  jobs: JobToggle[];
}

function unitFields(count: RoundField, spacing: RoundField): RoundField[] {
  return [count, { ...spacing, disabled: count.value <= 1 }];
}

/** The body of `round`. */
export function roundBody(round: RoundValues): RoundBody {
  const columns = unitFields(
    { key: "columns", label: "Columns", unit: "", value: round.columns, whole: true },
    { key: "column_spacing_mm", label: "Spacing", unit: "mm", value: round.columnSpacingMm, whole: false },
  );
  const rows = unitFields(
    { key: "rows", label: "Rows", unit: "", value: round.rows, whole: true },
    { key: "row_spacing_mm", label: "Spacing", unit: "mm", value: round.rowSpacingMm, whole: false },
  );
  const jobs: JobToggle[] = [];
  if (round.centreMark) {
    jobs.push({
      toggle: round.centreMark,
      label: "Centre mark",
      tooltip: "Centre mark",
      size: round.centreMark.on
        ? { key: "mark_mm", label: "Size", unit: "mm", value: round.markMm, whole: false }
        : null,
    });
  }
  if (round.diameters) {
    jobs.push({ toggle: round.diameters, label: "Diameters", tooltip: "Diameters", size: null });
  }
  return {
    diameter:
      round.diameterMm === null
        ? null
        : { key: "diameter_mm", label: "Diameter", unit: "mm", value: round.diameterMm, whole: false },
    columns,
    rows,
    jobs,
  };
}

/** Whether a kind has the pin and screw body. */
export function hasRoundBody(kind: FixtureKind): boolean {
  return kind === "pin" || kind === "screw";
}

/** The value a field sends for typed `text`: whole counts rounded; `null` for an empty or non-number text. */
export function fieldValue(field: Pick<RoundField, "whole">, text: string): number | null {
  const number = Number(text.trim().replace(",", "."));
  if (text.trim() === "" || !Number.isFinite(number)) return null;
  return field.whole ? Math.round(number) : number;
}

/** The text a field shows for its stored value: up to three decimals, no trailing zeros. */
export function fieldText(value: number): string {
  return String(Number(value.toFixed(3)));
}
