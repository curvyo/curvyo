/**
 * The decoders of the gradient views the session gives (`specs/0124-gradient-fill` criteria 3,
 * 6 to 8): the Style tab's Gradient block (`gradient_view_json`) and the canvas handles with
 * their readout and entry box (`gradient_handles_json`). Every field is required and typed;
 * anything else decodes to `null`. The Rust side pins the same fixtures
 * (`curvyo-editor-wasm/tests/gradient_view_contract.rs`).
 */

/** One stop row of the block. */
export interface GradientStopRow {
  /** `#RRGGBBAA`, empty when the shapes differ. */
  hex: string;
  /** The position in percent ("50", "33.3"), empty when the shapes differ. */
  position: string;
  /** The first or last row: fixed position text, no Remove. */
  fixed: boolean;
}

export type GradientKindView = "" | "linear" | "radial" | "mixed";

/** The Gradient block. */
export interface GradientView {
  offered: boolean;
  shown: boolean;
  kind: GradientKindView;
  angle: string;
  angleShown: boolean;
  radius: string;
  radiusShown: boolean;
  stops: GradientStopRow[];
  mixedGradients: boolean;
  activeStop: number;
  skip: string;
}

export type GradientHandleKind = "start" | "end" | "centre" | "radius" | "stop";

/** One handle on the canvas, screen pixels. */
export interface GradientHandleView {
  kind: GradientHandleKind;
  stop: number;
  x: number;
  y: number;
  hex: string;
  active: boolean;
  hovered: boolean;
}

/** The value chip at the pointer during a drag. */
export interface GradientReadout {
  text: string;
  x: number;
  y: number;
}

/** The entry box open at a handle. */
export interface GradientEntry {
  kind: "end" | "stop";
  stop: number;
  x: number;
  y: number;
  field: "" | "angle" | "radius" | "position";
  fieldText: string;
  hex: string;
}

/** The handles, their readout and entry box. */
export interface GradientHandles {
  shown: boolean;
  radial: boolean;
  line: [number, number, number, number];
  ellipse: [number, number, number, number, number, number] | null;
  handles: GradientHandleView[];
  readout: GradientReadout | null;
  entry: GradientEntry | null;
}

type Fields = Record<string, unknown>;

function record(value: unknown): Fields | null {
  return typeof value === "object" && value !== null && !Array.isArray(value) ? (value as Fields) : null;
}

const isString = (v: unknown): v is string => typeof v === "string";
const isBool = (v: unknown): v is boolean => typeof v === "boolean";
const isNumber = (v: unknown): v is number => typeof v === "number" && Number.isFinite(v);
const isIndex = (v: unknown): v is number => isNumber(v) && Number.isInteger(v) && v >= 0;

function oneOf<T extends string>(value: unknown, options: readonly T[]): value is T {
  return typeof value === "string" && (options as readonly string[]).includes(value);
}

function numbers(value: unknown, length: number): number[] | null {
  return Array.isArray(value) && value.length === length && value.every(isNumber) ? value : null;
}

function stopRow(value: unknown): GradientStopRow | null {
  const v = record(value);
  if (!v || !isString(v.hex) || !isString(v.position) || !isBool(v.fixed)) return null;
  return { hex: v.hex, position: v.position, fixed: v.fixed };
}

/** The block in `value` (parsed JSON), or `null` when it is not one. */
export function decodeGradientView(value: unknown): GradientView | null {
  const v = record(value);
  if (!v) return null;
  if (!isBool(v.offered) || !isBool(v.shown) || !oneOf(v.kind, ["", "linear", "radial", "mixed"] as const)) return null;
  if (!isString(v.angle) || !isBool(v.angleShown) || !isString(v.radius) || !isBool(v.radiusShown)) return null;
  if (!isBool(v.mixedGradients) || !isIndex(v.activeStop) || !isString(v.skip) || !Array.isArray(v.stops)) return null;
  const stops = v.stops.map(stopRow);
  if (stops.some((row) => row === null)) return null;
  return {
    offered: v.offered,
    shown: v.shown,
    kind: v.kind,
    angle: v.angle,
    angleShown: v.angleShown,
    radius: v.radius,
    radiusShown: v.radiusShown,
    stops: stops as GradientStopRow[],
    mixedGradients: v.mixedGradients,
    activeStop: v.activeStop,
    skip: v.skip,
  };
}

function handle(value: unknown): GradientHandleView | null {
  const v = record(value);
  if (!v || !oneOf(v.kind, ["start", "end", "centre", "radius", "stop"] as const)) return null;
  if (!isIndex(v.stop) || !isNumber(v.x) || !isNumber(v.y) || !isString(v.hex)) return null;
  if (!isBool(v.active) || !isBool(v.hovered)) return null;
  return { kind: v.kind, stop: v.stop, x: v.x, y: v.y, hex: v.hex, active: v.active, hovered: v.hovered };
}

/** `undefined` when `value` is neither `null` nor a valid readout. */
function readout(value: unknown): GradientReadout | null | undefined {
  if (value === null) return null;
  const v = record(value);
  if (!v || !isString(v.text) || !isNumber(v.x) || !isNumber(v.y)) return undefined;
  return { text: v.text, x: v.x, y: v.y };
}

/** `undefined` when `value` is neither `null` nor a valid entry box. */
function entry(value: unknown): GradientEntry | null | undefined {
  if (value === null) return null;
  const v = record(value);
  if (!v || !oneOf(v.kind, ["end", "stop"] as const) || !isIndex(v.stop)) return undefined;
  if (!isNumber(v.x) || !isNumber(v.y) || !oneOf(v.field, ["", "angle", "radius", "position"] as const)) return undefined;
  if (!isString(v.fieldText) || !isString(v.hex)) return undefined;
  return { kind: v.kind, stop: v.stop, x: v.x, y: v.y, field: v.field, fieldText: v.fieldText, hex: v.hex };
}

/** The handles in `value` (parsed JSON), or `null` when they are not valid. */
export function decodeGradientHandles(value: unknown): GradientHandles | null {
  const v = record(value);
  if (!v || !isBool(v.shown) || !isBool(v.radial) || !Array.isArray(v.handles)) return null;
  const line = numbers(v.line, 4);
  const ellipse = v.ellipse === null ? null : numbers(v.ellipse, 6);
  if (!line || (v.ellipse !== null && !ellipse)) return null;
  const handles = v.handles.map(handle);
  const chip = readout(v.readout);
  const box = entry(v.entry);
  if (handles.some((h) => h === null) || chip === undefined || box === undefined) return null;
  return {
    shown: v.shown,
    radial: v.radial,
    line: line as GradientHandles["line"],
    ellipse: ellipse as GradientHandles["ellipse"],
    handles: handles as GradientHandleView[],
    readout: chip,
    entry: box,
  };
}

/** Decodes one of the two JSON texts with `decode`; `null` for empty or bad JSON. */
export function decodeText<T>(text: string, decode: (value: unknown) => T | null): T | null {
  if (!text) return null;
  try {
    return decode(JSON.parse(text));
  } catch {
    return null;
  }
}
