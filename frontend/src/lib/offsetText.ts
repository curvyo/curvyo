// The words of the Offset command (`specs/0038-path-offset` criteria 7, 8, 13, 14 and the UX notes,
// `docs/design-system.md` rows "Offset section" and "Action notice"): the caption under Distance,
// the success notice and the refusal sentences. Pure, so they are tested without a session.

import { NOTHING_CHANGED, plural, sharedRefusalText } from "./booleanText.ts";

/** What `WasmSession.offset_state` reports. */
export interface OffsetState {
  /** `"closed"`, `"result"`, `"zero"`, `"no_operands"`, `"inset_open"`, `"out_of_range"`,
   * `"no_area"`, `"groups"` or `"empty"`. */
  kind: string;
  count: number;
  of: number;
  vanished: number;
  offered: boolean;
  hasOpen: boolean;
  distanceMm: number;
  join: OffsetJoinName;
  cap: OffsetCapName;
  typedLimitMm: number;
  dragLimitMm: number;
}

export type OffsetJoinName = "round" | "miter" | "bevel";
export type OffsetCapName = "round" | "flat" | "square";

/** What `WasmSession.apply_offset` did. */
export interface OffsetResult {
  /** `"ignored"`, `"closed_zero"`, `"applied"`, `"groups"` or a refusal code of `OffsetState`. */
  kind: string;
  count: number;
  of: number;
  vanished: number;
  distanceMm: number;
}

export const OFFSET_CLOSED: OffsetState = {
  kind: "closed",
  count: 0,
  of: 0,
  vanished: 0,
  offered: false,
  hasOpen: false,
  distanceMm: 1,
  join: "round",
  cap: "round",
  typedLimitMm: 1000,
  dragLimitMm: 50,
};

/** A distance in millimetres with up to three decimals and no trailing zeros; a negative one with
 * a real minus sign when `signed` ("−10"). */
export function formatDistance(mm: number, signed: boolean): string {
  const rounded = Math.round(Math.abs(mm) * 1000) / 1000;
  const text = String(rounded);
  return signed && mm < 0 && rounded !== 0 ? `−${text}` : text;
}

/** The caption under Distance: the side ("Outset", "Inset", nothing at 0), or the reason the
 * previewed result is empty or refused. `refusal` says whether it is shown as a refusal. */
export function offsetCaption(state: OffsetState): { text: string; refusal: boolean } {
  switch (state.kind) {
    case "empty":
      return { text: `Nothing is left at ${formatDistance(state.distanceMm, true)} mm.`, refusal: true };
    case "inset_open": {
      const verb = state.count === 1 ? "is" : "are";
      return {
        text: `Needs closed paths: ${state.count} of ${state.of} selected ${verb} open.`,
        refusal: true,
      };
    }
    case "no_area":
      return { text: "Needs shapes that enclose an area.", refusal: true };
    case "groups":
      return { text: "Does not work on groups. Ungroup first.", refusal: true };
    case "out_of_range":
      return { text: "Reaches beyond 10 km from the point 0, 0.", refusal: true };
    default:
      if (state.distanceMm > 0) {
        return { text: "Outset", refusal: false };
      }
      if (state.distanceMm < 0) {
        return { text: "Inset", refusal: false };
      }
      return { text: "", refusal: false };
  }
}

/** The notice after Apply (criteria 7 and 13), or `null` if nothing was added. */
export function offsetSuccessText(result: OffsetResult): string | null {
  if (result.kind !== "applied") {
    return null;
  }
  const side = result.distanceMm < 0 ? "inward" : "outward";
  const added = `Offset: added ${plural(result.count, "shape", "shapes")}, ${formatDistance(result.distanceMm, false)} mm ${side}.`;
  if (result.vanished === 0) {
    return added;
  }
  return `${added} ${result.vanished} of ${result.of} shapes vanished.`;
}

/** The refusal after Apply (criteria 7, 8, 14), or `null` for a result that is no refusal. */
export function offsetRefusalText(result: OffsetResult, distanceMm: number): string | null {
  switch (result.kind) {
    case "empty":
      return `Inset is empty: nothing is left at ${formatDistance(distanceMm, false)} mm. ${NOTHING_CHANGED}`;
    case "inset_open": {
      const verb = result.count === 1 ? "is" : "are";
      const objects = plural(result.of, "selected object", "selected objects");
      return `Inset needs closed paths. ${result.count} of ${objects} ${verb} open. ${NOTHING_CHANGED}`;
    }
    case "groups":
    case "no_area":
    case "out_of_range":
      return sharedRefusalText("Offset", result);
    default:
      return null;
  }
}
