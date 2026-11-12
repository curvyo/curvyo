/**
 * The texts and rules of the Settings block (`specs/0028-material-test-library` `ux-notes.md`
 * section 8): the head, the Material pick's value, the state lines, the number fields' names and
 * the "Material 60" lines beside typed numbers. Pure functions over the decoded view
 * (`operationSettingsView.ts`); the component draws what these return.
 */
import type {
  FieldNumbers,
  OperationSettings,
  PickMaterial,
  SetKind,
  SettingsField,
  SettingsRole,
  SetView,
} from "./operationSettingsView.ts";

/** The set kind word of a material kind. */
export const SET_WORDS: Record<SetKind, string> = { cut: "Cut", score: "Score", engrave: "Engrave" };

/** With more than this many materials the pick shows a Filter field (Proposal). */
export const PICK_FILTER_ABOVE = 8;

/** The three number fields of a set, in the order they are drawn. */
export const FIELDS: readonly { field: SettingsField; label: string; unit: string; unitWords: string }[] = [
  { field: "power", label: "Power", unit: "%", unitWords: "percent" },
  { field: "speed", label: "Speed", unit: "mm/min", unitWords: "millimetres per minute" },
  { field: "passes", label: "Passes", unit: "×", unitWords: "passes" },
];

/** A number as a field shows it: no trailing zeros, "" for a number that is not there. */
export function numberText(value: number | null): string {
  return value === null ? "" : String(value);
}

/** The value of `field` in `numbers`. */
export function fieldOf(numbers: FieldNumbers, field: SettingsField): number | null {
  switch (field) {
    case "power":
      return numbers.powerPct;
    case "speed":
      return numbers.speedMmMin;
    case "passes":
      return numbers.passes;
  }
}

/** The material's number beside a typed field: "Material 60" (the unit is in the field). */
export function materialLine(set: SetView, field: SettingsField): string | null {
  if (!set.typed.includes(field) || set.material === null) return null;
  const value = fieldOf(
    { powerPct: set.material.powerPct, speedMmMin: set.material.speedMmMin, passes: set.material.passes },
    field,
  );
  return value === null ? null : `Material ${value}`;
}

/** The field's description for assistive technology: "Material 60 %". */
export function materialDescription(set: SetView, field: SettingsField): string | null {
  const line = materialLine(set, field);
  if (line === null) return null;
  const unit = FIELDS.find((entry) => entry.field === field)?.unit ?? "";
  return `${line} ${unit}`.trim();
}

/** The accessible name of a field: "Power, percent"; the Outline set of Engrave + Outline says so. */
export function fieldName(field: SettingsField, role: SettingsRole, twoSets: boolean): string {
  const entry = FIELDS.find((candidate) => candidate.field === field);
  const base = `${entry?.label ?? field}, ${entry?.unitWords ?? ""}`;
  return twoSets ? `${base}, ${role === "outline" ? "outline" : "hatch"}` : base;
}

/** The symbol of a state line. */
export type StateGlyph = "not-found" | "differs" | "warning";

/** One state line of the block: a glyph and a text (at most two, `ux-notes.md` 8.1). */
export interface StateLine {
  glyph: StateGlyph;
  text: string;
}

/** The lines of states (b), (c) and "Differs", then the speed warning; "No settings" has its own plain line. */
export function stateLines(settings: OperationSettings): StateLine[] {
  const lines: StateLine[] = [];
  if (settings.reason !== null) lines.push({ glyph: "not-found", text: settings.reason });
  if (settings.differs !== null) lines.push({ glyph: "differs", text: settings.differs });
  if (settings.overFeed !== null) lines.push({ glyph: "warning", text: settings.overFeed });
  return lines;
}

/** The block's accessible summary: "Power 65 %, speed 3000 mm/min, 1 pass, from Birch 3 mm, differs". */
export function blockSummary(settings: OperationSettings): string {
  const main = settings.sets.find((set) => set.role === "main");
  if (settings.state === "damaged") return "Damaged settings";
  if (!main || settings.state === "no_settings") return "No settings";
  const parts: string[] = [];
  const { powerPct, speedMmMin, passes } = main.run;
  if (powerPct !== null) parts.push(`Power ${powerPct} %`);
  if (speedMmMin !== null) parts.push(`speed ${speedMmMin} mm/min`);
  if (passes !== null) parts.push(`${passes} ${passes === 1 ? "pass" : "passes"}`);
  if (settings.material !== null) parts.push(`from ${settings.material.name}`);
  if (settings.differs !== null) parts.push("differs");
  return parts.join(", ");
}

/** The polite status after Reset to material. */
export function resetStatus(materialName: string): string {
  return `Numbers reset to ${materialName}.`;
}

/** The pick value of the source row: the material's name, "Own numbers", or "Mixed" when the rows differ. */
export function pickValue(rows: readonly { settings: OperationSettings | null }[]): { text: string; mixed: boolean } {
  const values = rows.map((row) => row.settings?.material?.name ?? null);
  const first = values[0] ?? null;
  const ids = rows.map((row) => row.settings?.material?.id ?? null);
  const same = ids.every((id) => id === (ids[0] ?? null));
  if (!same) return { text: "Mixed", mixed: true };
  return { text: first ?? "Own numbers", mixed: false };
}

/** The head: the operation's name, or "3 operations". */
export function blockHead(names: readonly string[]): string {
  return names.length === 1 ? (names[0] ?? "") : `${names.length} operations`;
}

/** The pick entries that stay after the Filter text (by name, case-insensitive). */
export function filterPick(pick: readonly PickMaterial[], query: string): PickMaterial[] {
  const needle = query.trim().toLowerCase();
  return needle === "" ? [...pick] : pick.filter((entry) => entry.name.toLowerCase().includes(needle));
}

/** "3.0 mm": the thickness as the rows and the pick show it. */
export function thicknessText(mm: number): string {
  return `${Number.isInteger(mm) ? mm.toFixed(1) : String(mm)} mm`;
}

/** The kind symbols' words in a material's accessible name: "Cut and Engrave sets". */
export function kindsText(kinds: readonly SetKind[]): string {
  const words = kinds.map((kind) => SET_WORDS[kind]);
  if (words.length === 0) return "no sets";
  if (words.length === 1) return `${words[0]} set`;
  return `${words.slice(0, -1).join(", ")} and ${words[words.length - 1]} sets`;
}

/** Whether "Edit material" shows: the material is readable (states a, c and "Differs" with them). */
export function canEditMaterial(settings: OperationSettings, pick: readonly PickMaterial[]): boolean {
  return settings.material !== null && pick.some((entry) => entry.id === settings.material?.id);
}

/** Whether a pick entry is disabled for the single selected operation, with its words. */
export function disabledReason(settings: OperationSettings | null, id: string): string | null {
  return settings?.disabled.find((entry) => entry.id === id)?.reason ?? null;
}
