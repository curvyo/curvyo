// The words of the Offset tool (`specs/0038-path-offset` criteria 5, 12, 15, 24, 29, 30, 32 and
// `ux-rework.md`): the bar's status, the two readout lines, the notice after a release or Apply
// and the refusal sentences. Pure, so they are tested without a session.

import { NOTHING_CHANGED, plural, sharedRefusalText } from "./booleanText.ts";

export type OffsetDirectionName = "out" | "in" | "both";
export type OffsetJoinName = "round" | "miter" | "bevel";
export type OffsetCapName = "round" | "flat" | "square";
export type OffsetResultName = "new" | "change";

/** What `WasmSession.offset_view` reports. */
export interface OffsetView {
  operands: number;
  hasOpen: boolean;
  hasGroup: boolean;
  handles: boolean;
  dragging: boolean;
  /** Signed: positive outward. */
  distanceMm: number;
  both: boolean;
  change: boolean;
  /** `""`, `"nothing_left"` or `"inset_open"`. */
  stop: string;
  vanished: number;
  of: number;
  large: boolean;
  /** `""`, `"becomes_path"` or `"stays_<kind>"`. */
  kind: string;
  /** `""` or a refusal code. */
  refusal: string;
  barDistanceMm: number;
  direction: OffsetDirectionName;
  join: OffsetJoinName;
  cap: OffsetCapName;
  result: OffsetResultName;
  minMm: number;
  maxMm: number;
}

/** What `WasmSession.apply_offset` or `take_offset_outcome` reports. */
export interface OffsetOutcome {
  /** `"ignored"`, `"applied"`, `"groups"`, `"inset_open"`, `"out_of_range"`, `"no_area"` or
   * `"empty"`. */
  kind: string;
  added: number;
  changed: number;
  count: number;
  of: number;
  vanished: number;
  openLeftOut: number;
  distanceMm: number;
  direction: string;
}

export const OFFSET_IDLE: OffsetView = {
  operands: 0,
  hasOpen: false,
  hasGroup: false,
  handles: false,
  dragging: false,
  distanceMm: 1,
  both: false,
  change: false,
  stop: "",
  vanished: 0,
  of: 0,
  large: false,
  kind: "",
  refusal: "",
  barDistanceMm: 1,
  direction: "out",
  join: "round",
  cap: "round",
  result: "new",
  minMm: 0.01,
  maxMm: 1000,
};

/** A distance in millimetres with up to three decimals and no trailing zeros. */
export function formatDistance(mm: number): string {
  return String(Math.round(Math.abs(mm) * 1000) / 1000);
}

const KIND_NAMES: Record<string, string> = {
  stays_rectangle: "Stays a rectangle.",
  stays_circle: "Stays a circle.",
  stays_ellipse: "Stays an ellipse.",
  stays_polygon: "Stays a polygon.",
  stays_star: "Stays a star.",
  becomes_path: "Becomes a path.",
};

/** The bar's status text (criteria 5, 15, 30): why controls are greyed, or what the result is. */
export function offsetStatus(view: OffsetView): string {
  if (view.hasGroup && view.operands === 0) {
    return "Groups cannot be offset. Ungroup first.";
  }
  if (view.operands === 0) {
    return "Select one or more shapes.";
  }
  if (view.refusal === "inset_open" || view.stop === "inset_open") {
    return "Inset needs closed paths.";
  }
  return KIND_NAMES[view.kind] ?? "";
}

/** The two lines of the readout chip during a drag (criteria 12, 15, 18, 30). */
export function offsetReadout(view: OffsetView): [string, string] {
  if (view.stop === "inset_open") {
    return ["Inset needs closed paths", ""];
  }
  const d = formatDistance(view.distanceMm);
  let first = view.both
    ? `Both ±${d} mm`
    : view.distanceMm < 0
      ? `Inset ${d} mm`
      : `Outset ${d} mm`;
  if (view.stop === "nothing_left") {
    first = "Nothing is left";
  } else if (view.vanished > 0) {
    first = `${first}, ${view.vanished} of ${view.of} ${view.of === 1 ? "shape vanishes" : "shapes vanish"}`;
  }
  let second: string;
  if (view.large) {
    second = "Preview off: large selection";
  } else if (!view.change) {
    second = "New object";
  } else if (view.both) {
    second =
      view.distanceMm < 0
        ? "Changes the original, adds the outer copy"
        : "Changes the original, adds the inner copy";
  } else {
    second = "Changes the original";
  }
  return [first, second];
}

/** Whether the readout's first line is in the invalid colour. */
export function offsetReadoutInvalid(view: OffsetView): boolean {
  return view.stop !== "";
}

/** The notice after a release or Apply (criterion 24), or `null` if nothing was written. */
export function offsetSuccessText(outcome: OffsetOutcome): string | null {
  if (outcome.kind !== "applied") {
    return null;
  }
  const side =
    outcome.direction === "both"
      ? "both sides"
      : outcome.direction === "in"
        ? "inward"
        : "outward";
  const d = `${formatDistance(outcome.distanceMm)} mm ${side}`;
  let what: string;
  if (outcome.changed > 0 && outcome.added > 0) {
    what = `changed ${plural(outcome.changed, "shape", "shapes")} and added ${outcome.added}`;
  } else if (outcome.changed > 0) {
    what = `changed ${plural(outcome.changed, "shape", "shapes")}`;
  } else {
    what = `added ${plural(outcome.added, "shape", "shapes")}`;
  }
  let text = `Offset: ${what}, ${d}.`;
  if (outcome.vanished > 0) {
    text += ` ${outcome.vanished} of ${outcome.of} shapes vanished.`;
  }
  if (outcome.openLeftOut > 0) {
    const verb = outcome.openLeftOut === 1 ? "is" : "are";
    text += ` Inset needs closed paths: ${outcome.openLeftOut} of ${outcome.of} ${verb} open.`;
  }
  return text;
}

/** The refusal after a release or Apply (criteria 29, 30, 32), or `null` for no refusal. */
export function offsetRefusalText(outcome: OffsetOutcome): string | null {
  switch (outcome.kind) {
    case "empty":
      return `Inset is empty: nothing is left at ${formatDistance(outcome.distanceMm)} mm. ${NOTHING_CHANGED}`;
    case "inset_open": {
      const verb = outcome.count === 1 ? "is" : "are";
      return `Inset needs closed paths: ${outcome.count} of ${outcome.of} ${verb} open. ${NOTHING_CHANGED}`;
    }
    case "groups":
      return `Offset does not work on groups. Ungroup first. ${NOTHING_CHANGED}`;
    case "no_area":
    case "out_of_range":
      return sharedRefusalText("Offset", outcome);
    default:
      return null;
  }
}
