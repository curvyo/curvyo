/**
 * The lower part of an unfolded operation row and its Interval field (`specs/0111-laser-operation-
 * tools` `ux-notes.md` 6.2 to 6.4, 11): which toggles and fields a row shows, their names and
 * the Up/Down steps. The session holds every range and answers a typed value with its chip; these
 * are pure functions over the decoded settings (`operationSettingsView.ts`).
 */
import type { OperationSettings } from "./operationSettingsView.ts";
import type { OperationKind } from "./operationsView.ts";

/** A typed field of the unfolded row: the session's option name and how the field is drawn. */
export interface OptionField {
  key: "interval" | "scan_angle" | "overscan" | "z_offset";
  /** The word inside the field's left edge. */
  label: string;
  /** The accessible name (UX 11). */
  name: string;
  /** The unit inside the right edge. */
  unit: string;
  /** Up and Down step, and the Shift step. */
  step: number;
  shiftStep: number;
  /** Decimals kept when stepping. */
  decimals: number;
  /** The field's width, px: the inside label, the widest value and the unit fit without clipping;
   * two fields wider than the 228 px column together wrap onto two lines. */
  width: number;
}

/** Interval, in the Hatch group of the upper part (6.2). */
export const INTERVAL_FIELD: OptionField = {
  key: "interval",
  label: "Interval",
  name: "Interval, millimetres",
  unit: "mm",
  step: 0.01,
  shiftStep: 0.1,
  decimals: 2,
  width: 124,
};

/** The three fields of the lower part, in the order they are drawn (6.3). */
export const OPTION_FIELDS: readonly OptionField[] = [
  { key: "scan_angle", label: "Scan angle", name: "Scan angle, degrees", unit: "°", step: 1, shiftStep: 10, decimals: 1, width: 164 },
  { key: "overscan", label: "Overscanning", name: "Overscanning, percent", unit: "%", step: 1, shiftStep: 10, decimals: 0, width: 128 },
  { key: "z_offset", label: "Z offset", name: "Z offset, millimetres", unit: "mm", step: 0.1, shiftStep: 1, decimals: 1, width: 128 },
];

/** A toggle of the lower part. */
export interface OptionToggle {
  option: "crosshatch" | "bidirectional";
  pressed: boolean;
}

/** One field of a row with its committed text. */
export interface ShownField extends OptionField {
  shown: string;
}

/** What the lower part draws for a row. */
export interface LowerPart {
  toggles: OptionToggle[];
  fields: ShownField[];
}

/** A number as a field shows it: no trailing zeros, "" for none. */
export function optionText(value: number | null): string {
  return value === null ? "" : String(Math.round(value * 100) / 100);
}

/** The name and tooltip of a toggle (no caption, UX 6.3). */
export function toggleNames(option: OptionToggle["option"]): string {
  return option === "crosshatch" ? "Crosshatch" : "Bidirectional fill";
}

/**
 * The lower part of a row of `kind`, `null` when it has nothing to show: damaged settings, an
 * unknown kind, or a Cut, Score or Cut-out without a motorised Z axis (the hairline goes too).
 * Z offset is absent, not disabled, without the capability (criterion 21).
 */
export function lowerPart(kind: OperationKind, settings: OperationSettings): LowerPart | null {
  const options = settings.options;
  if (options === null || kind === "unknown") return null;
  const toggles: OptionToggle[] = [];
  const fields: ShownField[] = [];
  const hatch = options.hatch;
  if ((kind === "engrave" || kind === "engrave_outline") && hatch !== null) {
    toggles.push(
      { option: "crosshatch", pressed: hatch.crosshatch },
      { option: "bidirectional", pressed: hatch.bidirectional },
    );
    fields.push(
      { ...OPTION_FIELDS[0], shown: optionText(hatch.scanAngleDeg) },
      { ...OPTION_FIELDS[1], shown: optionText(hatch.overscanPct) },
    );
  }
  if (settings.zOffsetShown) fields.push({ ...OPTION_FIELDS[2], shown: optionText(options.zOffsetMm) });
  return toggles.length === 0 && fields.length === 0 ? null : { toggles, fields };
}

/** The Up/Down step of a field on typed text (Shift: the big step); `null` for another key or
 * text that is no number. */
export function optionStep(field: OptionField): (text: string, key: string, shift: boolean) => string | null {
  return (text, key, shift) => {
    if (key !== "ArrowUp" && key !== "ArrowDown") return null;
    const value = Number(text.trim().replace(",", "."));
    if (text.trim() === "" || !Number.isFinite(value)) return null;
    const amount = shift ? field.shiftStep : field.step;
    const next = value + (key === "ArrowUp" ? 1 : -1) * amount;
    const scale = 10 ** Math.max(field.decimals, 2);
    return String(Math.round(next * scale) / scale);
  };
}
