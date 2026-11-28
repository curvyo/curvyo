/**
 * The settings part of the Operations tab's JSON (`specs/0028-material-test-library` `adrs.md`
 * section 6): each row's Settings block and the Material pick. `curvyo-editor-wasm/src/session/
 * operations/settings_view.rs` builds it, `operationsView.ts` calls these checked decoders, and the
 * Rust fixtures `operations_view/settings_*.json` pin both sides. No cast: a missing field, a wrong
 * type or an enum text this build does not know decodes to `null`.
 */
import { asPickKind, type PickKind } from "./picks.ts";

/** The kinds of a material's set. */
export const SET_KINDS = ["cut", "score", "engrave", "image"] as const;

/** One kind of set. */
export type SetKind = (typeof SET_KINDS)[number];

/** The state of an operation's settings (criterion 26). */
export type SettingsState = "damaged" | "own" | "live" | "stored";

/** The numbers field of a set. */
export type SettingsField = "power" | "speed" | "passes";

/** The two sets of an operation: the kind's own, and the outline of Engrave + Outline. */
export type SettingsRole = "main" | "outline";

/** The numbers of a set: never empty (`specs/0150` criterion 27). */
export interface FieldNumbers {
  powerPct: number;
  speedMmMin: number;
  passes: number;
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

/** One of the four picks an operation can follow, with whether its material has that set. */
export interface PickChoice {
  pick: PickKind;
  available: boolean;
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
  /** The pick the link follows; Best while none is stored (`specs/0146` criterion 15). */
  pick: PickKind;
  /** The four picks with their availability; empty while the operation has no link. */
  picks: PickChoice[];
  /** Whether "Reset to material" shows. */
  reset: boolean;
  needs: SetKind;
  disabled: DisabledPick[];
  /** The Interval in mm for Engrave and Engrave + Outline with numbers (`specs/0111`); else `null`. */
  intervalMm: number | null;
  /** The operation-only settings of the lower part; `null` for Damaged settings. */
  options: OperationOptions | null;
  /** Whether Z offset shows: the machine has a motorised Z axis (`specs/0111` criterion 21). */
  zOffsetShown: boolean;
}

/** The lower part of an unfolded row (`specs/0111` criterion 20). */
export interface OperationOptions {
  /** The hatch options, for Engrave and Engrave + Outline only. */
  hatch: HatchOptions | null;
  /** The stored Z offset in mm (kept even while hidden). */
  zOffsetMm: number;
  /** The Air assist toggle (ADR 0098 decision 6, Frage 435); `null` while the machine has no air
   * assist (the stored value is kept). */
  air: boolean | null;
  /** The Image settings group of an Image operation, always (`specs/0123` criterion 11);
   * `null` for every other kind. */
  image: ImageSettings | null;
  /** The Tabs group of a Cut or Cut-out (`specs/0139` criteria 1, 2); `null` for every other kind. */
  tabs: TabsOptions | null;
  /** The Path options group of a Cut or Cut-out (`specs/0139` criterion 13); `null` for every
   * other kind. */
  path: PathOptionValues | null;
}

/** A lead's shape. */
export type LeadKind = "none" | "line" | "arc";

/** Lead-in, lead-out and overcut of an operation or of an item's own block (`specs/0139`). */
export interface PathOptionValues {
  leadIn: LeadKind;
  leadInMm: number;
  leadOut: LeadKind;
  leadOutMm: number;
  overcutMm: number;
}

function asLead(value: unknown): LeadKind | undefined {
  return value === "none" || value === "line" || value === "arc" ? value : undefined;
}

/** The path option values of the session's JSON, `null` when absent, `undefined` when malformed. */
export function asPathOptions(value: unknown): PathOptionValues | null | undefined {
  if (value === null || value === undefined) return null;
  if (!isObject(value)) return undefined;
  const leadIn = asLead(value.lead_in);
  const leadOut = asLead(value.lead_out);
  if (leadIn === undefined || leadOut === undefined) return undefined;
  if (!finite(value.lead_in_mm) || !finite(value.lead_out_mm) || !finite(value.overcut_mm)) return undefined;
  return { leadIn, leadInMm: value.lead_in_mm, leadOut, leadOutMm: value.lead_out_mm, overcutMm: value.overcut_mm };
}

/** The operation's tabs (`specs/0139` criteria 1 to 3). */
export interface TabsOptions {
  count: number;
  widthMm: number;
  skipInner: boolean;
  /** "Placed: 3 per path", only when some path gets fewer than Count. */
  placed: string | null;
}

function asTabs(value: unknown): TabsOptions | null | undefined {
  if (value === null || value === undefined) return null;
  if (!isObject(value)) return undefined;
  if (!Number.isInteger(value.count) || !finite(value.width_mm) || typeof value.skip_inner !== "boolean") return undefined;
  if (value.placed !== null && typeof value.placed !== "string") return undefined;
  return {
    count: value.count as number,
    widthMm: value.width_mm,
    skipInner: value.skip_inner,
    placed: value.placed as string | null,
  };
}

/** The Dither modes in the strip's order (`specs/0123` criterion 11; the two screens of `specs/0135`
 * criteria 42 to 44 last). */
export const DITHER_MODES = [
  "threshold",
  "ordered",
  "floyd_steinberg",
  "jarvis",
  "atkinson",
  "stucki",
  "grey",
  "halftone",
  "line_screen",
] as const;

/** One Dither mode. */
export type DitherMode = (typeof DITHER_MODES)[number];

/** One mode of the Dither strip. */
export interface DitherOption {
  value: DitherMode;
  label: string;
}

/** The names `edit_operation_option` takes for the Image settings numbers (`screen_size` is the
 * Cell size of Halftone dots and the Line pitch of Line screen, `specs/0135` criteria 42, 43). */
export type ImageFieldName =
  | "threshold"
  | "lowest_power"
  | "brightness"
  | "contrast"
  | "gamma"
  | "screen_size"
  | "screen_angle";

/** One number field of the Image settings group. */
export interface ImageField {
  field: ImageFieldName;
  label: string;
  value: number;
  /** "%", empty for Gamma. */
  unit: string;
  /** The chip text of a value out of range. */
  range: string;
}

/** A Dither strip with its mode's own fields: a picture's (`specs/0135` UX notes 3.10) or an
 * Image operation's for its gradient fills (UX notes 10). */
export interface DitherSettings {
  /** `null` for a damaged dither: no cell is pressed (UX P46). */
  dither: DitherMode | null;
  modes: DitherOption[];
  /** Threshold's field with Threshold, `null` for the other modes. */
  modeField: ImageField | null;
  /** Cell size or Line pitch, then Angle, for the two screens; empty for the other modes. */
  screenFields: ImageField[];
}

/** The Image settings group of an Image row (`specs/0135` UX notes 10): Brightness, Contrast,
 * Gamma, Invert, Lowest power while an item burns Grey, and the operation's own Dither only while
 * it holds a gradient fill. */
export interface ImageSettings {
  fields: ImageField[];
  invert: boolean;
  lowestPower: ImageField | null;
  dither: DitherSettings | null;
}

/** The four hatch options. */
export interface HatchOptions {
  /** `null` on an Image operation: crosshatch would do nothing there. */
  crosshatch: boolean | null;
  bidirectional: boolean;
  scanAngleDeg: number;
  overscanPct: number;
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
  return asMaterialNumbers(value);
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

function asHatch(value: unknown): HatchOptions | null | undefined {
  if (value === null) return null;
  if (!isObject(value)) return undefined;
  if (
    (value.crosshatch !== null && typeof value.crosshatch !== "boolean") ||
    typeof value.bidirectional !== "boolean" ||
    !finite(value.scan_angle_deg) ||
    !whole(value.overscan_pct)
  ) {
    return undefined;
  }
  return {
    crosshatch: value.crosshatch,
    bidirectional: value.bidirectional,
    scanAngleDeg: value.scan_angle_deg,
    overscanPct: value.overscan_pct,
  };
}

function asDither(value: unknown): DitherMode | null {
  return (DITHER_MODES as readonly unknown[]).includes(value) ? (value as DitherMode) : null;
}

/** One mode of a Dither strip; `null` when wrong. */
export function decodeDitherOption(value: unknown): DitherOption | null {
  if (!isObject(value) || !text(value.label)) return null;
  const mode = asDither(value.value);
  return mode === null ? null : { value: mode, label: value.label };
}

const IMAGE_FIELDS: readonly unknown[] = [
  "threshold",
  "lowest_power",
  "brightness",
  "contrast",
  "gamma",
  "screen_size",
  "screen_angle",
];

function asImageField(value: unknown): ImageField | null {
  if (
    !isObject(value) ||
    !IMAGE_FIELDS.includes(value.field) ||
    !text(value.label) ||
    !finite(value.value) ||
    typeof value.unit !== "string" ||
    !text(value.range)
  ) {
    return null;
  }
  return {
    field: value.field as ImageFieldName,
    label: value.label,
    value: value.value,
    unit: value.unit,
    range: value.range,
  };
}

/** A Dither strip and its fields; `undefined` when wrong. */
export function decodeDitherSettings(value: unknown): DitherSettings | undefined {
  if (!isObject(value) || !Array.isArray(value.modes) || !Array.isArray(value.screen_fields)) {
    return undefined;
  }
  const dither = value.dither === null ? null : asDither(value.dither);
  const modeField = value.mode_field === null ? null : asImageField(value.mode_field);
  const modes = value.modes.map(decodeDitherOption);
  const screenFields = value.screen_fields.map(asImageField);
  if (
    (value.dither !== null && dither === null) ||
    (value.mode_field !== null && modeField === null) ||
    modes.some((m) => m === null) ||
    screenFields.some((f) => f === null)
  ) {
    return undefined;
  }
  return {
    dither,
    modes: modes as DitherOption[],
    modeField,
    screenFields: screenFields as ImageField[],
  };
}

function asImage(value: unknown): ImageSettings | null | undefined {
  if (value === null) return null;
  if (!isObject(value) || !Array.isArray(value.fields)) return undefined;
  const fields = value.fields.map(asImageField);
  const lowestPower = value.lowest_power === null ? null : asImageField(value.lowest_power);
  const dither = value.dither === null ? null : decodeDitherSettings(value.dither);
  if (
    typeof value.invert !== "boolean" ||
    fields.some((f) => f === null) ||
    (value.lowest_power !== null && lowestPower === null) ||
    dither === undefined
  ) {
    return undefined;
  }
  return { fields: fields as ImageField[], invert: value.invert, lowestPower, dither };
}

function asOptions(value: unknown): OperationOptions | null | undefined {
  if (value === null) return null;
  if (!isObject(value)) return undefined;
  const hatch = asHatch(value.hatch);
  const image = asImage(value.image);
  const tabs = asTabs(value.tabs);
  const path = asPathOptions(value.path);
  if (hatch === undefined || image === undefined || tabs === undefined || path === undefined || !finite(value.z_offset_mm)) {
    return undefined;
  }
  const air = typeof value.air === "boolean" ? value.air : null;
  return { hatch, zOffsetMm: value.z_offset_mm, air, image, tabs, path };
}

function asPickChoices(value: unknown): PickChoice[] | null {
  if (!Array.isArray(value)) return null;
  const out: PickChoice[] = [];
  for (const raw of value) {
    const pick = isObject(raw) ? asPickKind(raw.pick) : null;
    if (!isObject(raw) || pick === null || typeof raw.available !== "boolean") return null;
    out.push({ pick, available: raw.available });
  }
  return out;
}

const STATES: readonly unknown[] = ["damaged", "own", "live", "stored"];

/** One row's Settings block; `undefined` when wrong, `null` for a row that has none. */
export function decodeSettings(value: unknown): OperationSettings | null | undefined {
  if (value === null) return null;
  if (!isObject(value)) return undefined;
  const needs = asSetKind(value.needs);
  const reason = nullable(value.reason, text);
  const differs = nullable(value.differs, text);
  const overFeed = nullable(value.over_feed, text);
  const link = value.material === null ? null : asLink(value.material);
  const intervalMm = nullable(value.interval_mm, finite);
  const options = asOptions(value.options);
  const pick = asPickKind(value.pick);
  const picks = asPickChoices(value.picks);
  if (
    pick === null ||
    picks === null ||
    intervalMm === undefined ||
    options === undefined ||
    typeof value.z_offset_shown !== "boolean" ||
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
    pick,
    picks,
    reset: value.reset,
    needs,
    disabled,
    intervalMm,
    options,
    zOffsetShown: value.z_offset_shown,
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
