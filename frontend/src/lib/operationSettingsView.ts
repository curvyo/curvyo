/**
 * The settings part of the Operations tab's JSON (`specs/0028-material-test-library` `adrs.md`
 * section 6): each row's Settings block and the Material pick. `curvyo-editor-wasm/src/session/
 * operations/settings_view.rs` builds it, `operationsView.ts` calls these checked decoders, and the
 * Rust fixtures `operations_view/settings_*.json` pin both sides. No cast: a missing field, a wrong
 * type or an enum text this build does not know decodes to `null`.
 */

/** The kinds of a material's set. */
export const SET_KINDS = ["cut", "score", "engrave"] as const;

/** One kind of set. */
export type SetKind = (typeof SET_KINDS)[number];

/** The state of an operation's settings (criterion 26). */
export type SettingsState = "no_settings" | "damaged" | "own" | "live" | "stored";

/** The numbers field of a set. */
export type SettingsField = "power" | "speed" | "passes";

/** The two sets of an operation: the kind's own, and the outline of Engrave + Outline. */
export type SettingsRole = "main" | "outline";

/** The numbers of a set; a field is `null` only in the No settings draft. */
export interface FieldNumbers {
  powerPct: number | null;
  speedMmMin: number | null;
  passes: number | null;
}

/** A material's three numbers (never empty). */
export interface MaterialNumbers {
  powerPct: number;
  speedMmMin: number;
  passes: number;
}

/** One set of the block: what runs, the material's numbers and the fields typed over them. */
export interface SetView {
  role: SettingsRole;
  run: FieldNumbers;
  material: MaterialNumbers | null;
  typed: SettingsField[];
}

/** The material an operation points to. */
export interface MaterialLink {
  id: string;
  name: string;
}

/** A pick entry this operation cannot take, with the words that replace the symbols. */
export interface DisabledPick {
  id: string;
  reason: string;
}

/** One row's Settings block. */
export interface OperationSettings {
  state: SettingsState;
  material: MaterialLink | null;
  /** "Material not found, using stored numbers" and its sibling (state stored). */
  reason: string | null;
  /** "Differs from Birch 3 mm" while a number is typed. */
  differs: string | null;
  /** "Speed 7000 is above this machine's 6000 mm/min." */
  overFeed: string | null;
  sets: SetView[];
  /** Whether "Reset to material" shows. */
  reset: boolean;
  needs: SetKind;
  disabled: DisabledPick[];
}

/** A material of the pick. */
export interface PickMaterial {
  id: string;
  name: string;
  thicknessMm: number;
  kinds: SetKind[];
}

type Json = Record<string, unknown>;

export function isObject(value: unknown): value is Json {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

/** A set kind this build knows, else `null`. */
export function asSetKind(value: unknown): SetKind | null {
  return (SET_KINDS as readonly unknown[]).includes(value) ? (value as SetKind) : null;
}

/** A list of set kinds, `null` when one is wrong. */
export function asSetKinds(value: unknown): SetKind[] | null {
  if (!Array.isArray(value)) return null;
  const kinds: SetKind[] = [];
  for (const raw of value) {
    const kind = asSetKind(raw);
    if (kind === null) return null;
    kinds.push(kind);
  }
  return kinds;
}

function finite(value: unknown): value is number {
  return typeof value === "number" && Number.isFinite(value);
}

function whole(value: unknown): value is number {
  return Number.isSafeInteger(value) && (value as number) >= 0;
}

function nullable<T>(value: unknown, check: (v: unknown) => v is T): T | null | undefined {
  if (value === null) return null;
  return check(value) ? value : undefined;
}

function text(value: unknown): value is string {
  return typeof value === "string";
}

/** The three numbers of a material's set; `null` when wrong. */
export function asMaterialNumbers(value: unknown): MaterialNumbers | null {
  if (!isObject(value)) return null;
  const { power_pct: power, speed_mm_min: speed, passes } = value;
  if (!finite(power) || !whole(speed) || !whole(passes)) return null;
  return { powerPct: power, speedMmMin: speed, passes };
}

function asFieldNumbers(value: unknown): FieldNumbers | null {
  if (!isObject(value)) return null;
  const power = nullable(value.power_pct, finite);
  const speed = nullable(value.speed_mm_min, whole);
  const passes = nullable(value.passes, whole);
  if (power === undefined || speed === undefined || passes === undefined) return null;
  return { powerPct: power, speedMmMin: speed, passes };
}

function asField(value: unknown): SettingsField | null {
  return value === "power" || value === "speed" || value === "passes" ? value : null;
}

function asSet(value: unknown): SetView | null {
  if (!isObject(value)) return null;
  const role = value.role === "main" || value.role === "outline" ? value.role : null;
  const run = asFieldNumbers(value.run);
  const material = value.material === null ? null : asMaterialNumbers(value.material);
  if (role === null || run === null || (value.material !== null && material === null)) return null;
  if (!Array.isArray(value.typed)) return null;
  const typed: SettingsField[] = [];
  for (const raw of value.typed) {
    const field = asField(raw);
    if (field === null) return null;
    typed.push(field);
  }
  return { role, run, material, typed };
}

function asLink(value: unknown): MaterialLink | null {
  if (!isObject(value) || !text(value.id) || !text(value.name)) return null;
  return { id: value.id, name: value.name };
}

function asDisabled(value: unknown): DisabledPick | null {
  if (!isObject(value) || !text(value.id) || !text(value.reason)) return null;
  return { id: value.id, reason: value.reason };
}

const STATES: readonly unknown[] = ["no_settings", "damaged", "own", "live", "stored"];

/** One row's Settings block; `undefined` when wrong, `null` for a row that has none. */
export function decodeSettings(value: unknown): OperationSettings | null | undefined {
  if (value === null) return null;
  if (!isObject(value)) return undefined;
  const needs = asSetKind(value.needs);
  const reason = nullable(value.reason, text);
  const differs = nullable(value.differs, text);
  const overFeed = nullable(value.over_feed, text);
  const link = value.material === null ? null : asLink(value.material);
  if (
    !STATES.includes(value.state) ||
    needs === null ||
    reason === undefined ||
    differs === undefined ||
    overFeed === undefined ||
    (value.material !== null && link === null) ||
    typeof value.reset !== "boolean" ||
    !Array.isArray(value.sets) ||
    !Array.isArray(value.disabled)
  ) {
    return undefined;
  }
  const sets: SetView[] = [];
  for (const raw of value.sets) {
    const set = asSet(raw);
    if (set === null) return undefined;
    sets.push(set);
  }
  const disabled: DisabledPick[] = [];
  for (const raw of value.disabled) {
    const entry = asDisabled(raw);
    if (entry === null) return undefined;
    disabled.push(entry);
  }
  return {
    state: value.state as SettingsState,
    material: link,
    reason,
    differs,
    overFeed,
    sets,
    reset: value.reset,
    needs,
    disabled,
  };
}

/** The pick of the active machine's materials; `null` when wrong. */
export function decodePick(value: unknown): PickMaterial[] | null {
  if (!Array.isArray(value)) return null;
  const pick: PickMaterial[] = [];
  for (const raw of value) {
    if (!isObject(raw) || !text(raw.id) || !text(raw.name) || !finite(raw.thickness_mm)) return null;
    const kinds = asSetKinds(raw.kinds);
    if (kinds === null) return null;
    pick.push({ id: raw.id, name: raw.name, thicknessMm: raw.thickness_mm, kinds });
  }
  return pick;
}
