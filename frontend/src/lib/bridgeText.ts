/**
 * The words and values of Bridge (`specs/0156-fused-lettering/`; customer answers of 2026-11-26:
 * no chip and no preview, the fields in the Select bar, Join all): name, rule, tooltip note,
 * refusal notices, the field limits and the remembered values. Rust sends codes and counts; the
 * sentences are written here, once.
 */

import { NOTHING_CHANGED, sharedRefusalText } from "./booleanText.ts";
import { textRefusalOutlines, textRefusalText } from "./textText.ts";

/** The button's accessible name and tooltip name line. */
export const BRIDGE_NAME = "Bridge";

/** The tooltip's rule line (UX notes §3, at most 45 characters). */
export const BRIDGE_RULE = "Bridges loose pieces so they hold together.";

/** What the Bridge buttons show (`WasmSession.fuse_availability`). */
export type BridgeAvailability = "absent" | "needs_one" | "ready";

/** Lettering joins the pieces; Stencil holds the islands (criterion 21). */
export type BridgeMode = "lettering" | "stencil";

/** The values of the Bridge fields. */
export interface BridgeSettings {
  mode: BridgeMode;
  widthMm: number;
  reachMm: number;
  joinAll: boolean;
  bridgesPerIsland: number;
}

/** The limits and default of a field (criteria 5 and 21, the same numbers as `fuse_plan.rs`). */
export interface FieldRange {
  min: number;
  max: number;
  default: number;
}

export const WIDTH_RANGE: FieldRange = { min: 0.2, max: 10, default: 1 };
export const REACH_RANGE: FieldRange = { min: 0.5, max: 50, default: 5 };
export const BRIDGES_RANGE: FieldRange = { min: 1, max: 4, default: 2 };

export const DEFAULT_BRIDGE_SETTINGS: BridgeSettings = {
  mode: "lettering",
  widthMm: WIDTH_RANGE.default,
  reachMm: REACH_RANGE.default,
  joinAll: false,
  bridgesPerIsland: BRIDGES_RANGE.default,
};

/** What `WasmSession.apply_fuse` did (`kind` is the outcome code). */
export interface BridgeResult {
  kind: string;
  /** Bridges made, or objects or texts to blame for a refusal. */
  count: number;
  /** Objects selected. */
  of: number;
  /** The one missing face of a refusal of texts. */
  face?: string;
}

/** The third tooltip line: dimmed, what is missing; else what a press does. */
export function bridgeTooltipNote(availability: BridgeAvailability, selectTool: boolean): string {
  if (availability !== "ready") {
    const note = "Select one or more objects.";
    return selectTool ? note : `${note} Use the Select tool.`;
  }
  return "Replaces the selection.";
}

/** A field value within its limits, with the limit notice when a typed value was clamped
 * (criterion 5, the `0141` pattern). `null` text: not a number. */
export function clampField(
  range: FieldRange,
  label: string,
  value: number,
  unit: string,
): { value: number; notice: string | null } | null {
  if (!Number.isFinite(value)) {
    return null;
  }
  if (value > range.max) {
    return { value: range.max, notice: `${label} set to ${range.max}${unit}, the maximum.` };
  }
  if (value < range.min) {
    return { value: range.min, notice: `${label} set to ${range.min}${unit}, the minimum.` };
  }
  return { value, notice: null };
}

/** The notice after a refusal (criteria 4, 14, 17 and 22), or `null` for a result that is no
 * refusal. */
export function bridgeRefusalText(result: BridgeResult): string | null {
  switch (result.kind) {
    case "one_page":
      return `Bridge works on one laser page. ${NOTHING_CHANGED}`;
    case "assigned_parent":
      return `Bridge: the group or layer already has an operation. ${NOTHING_CHANGED}`;
    case "nothing_to_join":
      return `Bridge: nothing to join. ${NOTHING_CHANGED}`;
    case "too_many_bridges":
      return `Bridge would make more than 2000 bridges. ${NOTHING_CHANGED}`;
    case "no_islands":
      return `Bridge: no islands. ${NOTHING_CHANGED}`;
    default:
      return textRefusalText(result) ?? sharedRefusalText(BRIDGE_NAME, result);
  }
}

/** Whether a refusal outlines objects on the canvas. */
export function bridgeRefusalOutlines(result: BridgeResult): boolean {
  return (
    result.kind === "extended_path" ||
    result.kind === "no_area" ||
    result.kind === "out_of_range" ||
    textRefusalOutlines(result.kind)
  );
}

/** The app preference keys (`adrs.md` decision 6). */
export const BRIDGE_KEYS = {
  width: "curvyo.fuse-width-mm",
  reach: "curvyo.fuse-reach-mm",
  joinAll: "curvyo.fuse-join-all",
  mode: "curvyo.fuse-mode",
  bridges: "curvyo.fuse-bridges",
} as const;

/** What the preferences are read from and written to. */
export interface BridgeStorage {
  getItem(key: string): string | null;
  setItem(key: string, value: string): void;
}

function readNumber(storage: BridgeStorage | undefined, key: string, range: FieldRange): number {
  try {
    const raw = storage?.getItem(key);
    const value = raw === null || raw === undefined ? Number.NaN : Number(raw);
    return Number.isFinite(value) ? Math.min(range.max, Math.max(range.min, value)) : range.default;
  } catch {
    return range.default;
  }
}

/** The remembered values, clamped; anything unreadable is its default. */
export function readBridgeSettings(storage: BridgeStorage | undefined): BridgeSettings {
  let mode: BridgeMode = "lettering";
  let joinAll = false;
  try {
    mode = storage?.getItem(BRIDGE_KEYS.mode) === "stencil" ? "stencil" : "lettering";
    joinAll = storage?.getItem(BRIDGE_KEYS.joinAll) === "true";
  } catch {
    /* Defaults. */
  }
  return {
    mode,
    widthMm: readNumber(storage, BRIDGE_KEYS.width, WIDTH_RANGE),
    reachMm: readNumber(storage, BRIDGE_KEYS.reach, REACH_RANGE),
    joinAll,
    bridgesPerIsland: Math.round(readNumber(storage, BRIDGE_KEYS.bridges, BRIDGES_RANGE)),
  };
}

/** Remembers `settings`. A failing write is ignored. */
export function writeBridgeSettings(storage: BridgeStorage | undefined, settings: BridgeSettings): void {
  try {
    storage?.setItem(BRIDGE_KEYS.mode, settings.mode);
    storage?.setItem(BRIDGE_KEYS.width, String(settings.widthMm));
    storage?.setItem(BRIDGE_KEYS.reach, String(settings.reachMm));
    storage?.setItem(BRIDGE_KEYS.joinAll, String(settings.joinAll));
    storage?.setItem(BRIDGE_KEYS.bridges, String(settings.bridgesPerIsland));
  } catch {
    /* Not kept; the next start shows the defaults. */
  }
}
