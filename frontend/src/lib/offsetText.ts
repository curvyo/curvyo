// The words of the Offset tool (`specs/0038-path-offset` criteria 12, 15, 24, 29, 32 and
// `ux-rework.md`): the readout lines, the notice after a release or the chip's Enter and the
// refusal sentences. Pure, so they are tested without a session.

import { NOTHING_CHANGED, plural, sharedRefusalText } from "./booleanText.ts";
import { textRefusalText } from "./textText.ts";

export type OffsetDirectionName = "out" | "in" | "both";
export type OffsetJoinName = "round" | "miter" | "bevel";
export type OffsetResultName = "new" | "change";

/** What `WasmSession.offset_view` reports. */
export interface OffsetView {
  operands: number;
  hasGroup: boolean;
  handles: boolean;
  dragging: boolean;
  /** Signed: positive outward. */
  distanceMm: number;
  both: boolean;
  change: boolean;
  /** `""` or `"nothing_left"`. */
  stop: string;
  vanished: number;
  of: number;
  large: boolean;
  /** `""` or a refusal code. */
  refusal: string;
  /** The objects the refusal blames, and how many it counts against. */
  refusalCount: number;
  refusalOf: number;
  valuesDistanceMm: number;
  direction: OffsetDirectionName;
  /** The remembered Corners, which the chip shows. */
  join: OffsetJoinName;
  /** The corner style in use: the drag's (stepped by Alt), else the remembered one. */
  dragJoin: OffsetJoinName;
  chipOpen: boolean;
  result: OffsetResultName;
  minMm: number;
  maxMm: number;
}

/** What `WasmSession.apply_offset` or `take_offset_outcome` reports. */
export interface OffsetOutcome {
  /** `"ignored"`, `"applied"`, `"groups"`, `"out_of_range"`, `"no_area"` or `"empty"`. */
  kind: string;
  added: number;
  changed: number;
  count: number;
  of: number;
  vanished: number;
  distanceMm: number;
  direction: string;
  /** The one missing face of a refusal of texts (`specs/0074-text-objects` criterion 29). */
  face?: string;
}

export const OFFSET_IDLE: OffsetView = {
  operands: 0,
  hasGroup: false,
  handles: false,
  dragging: false,
  distanceMm: 1,
  both: false,
  change: true,
  stop: "",
  vanished: 0,
  of: 0,
  large: false,
  refusal: "",
  refusalCount: 0,
  refusalOf: 0,
  valuesDistanceMm: 1,
  direction: "out",
  join: "round",
  dragJoin: "round",
  chipOpen: false,
  result: "change",
  minMm: 0.01,
  maxMm: 1000,
};

/** A distance in millimetres with up to three decimals and no trailing zeros. */
export function formatDistance(mm: number): string {
  return String(Math.round(Math.abs(mm) * 1000) / 1000);
}

/** The corner styles by name, as the readout and the chip's tooltips say them. */
export const JOIN_NAMES: Record<OffsetJoinName, string> = {
  round: "Round",
  miter: "Miter",
  bevel: "Bevel",
};

/** The readout chip during a drag (criteria 12, 15, 18; `ux-rework.md` section 3): line 1 the side,
 * the distance and always the corner style; line 2 the effective Mode and Direction; line 3 only when it applies,
 * in the invalid colour when nothing is left. */
export function offsetReadout(view: OffsetView): { lines: string[]; invalid: boolean } {
  const d = formatDistance(view.distanceMm);
  const side = view.both ? `Both ±${d} mm` : view.distanceMm < 0 ? `Inset ${d} mm` : `Outset ${d} mm`;
  const lines = [`${side} · ${JOIN_NAMES[view.dragJoin]}`];
  // Both with Change original: the original takes the outset, the copy is the inner one
  // (criterion 19, third round).
  if (view.change) {
    lines.push(view.both ? "Changes the original, adds the inner copy" : "Changes the original");
  } else {
    lines.push(view.both ? "Two new objects" : "New object");
  }
  if (view.stop === "nothing_left") {
    lines.push("Nothing is left");
  } else if (view.large) {
    lines.push("Preview off: large selection");
  } else if (view.vanished > 0) {
    const shapes = view.of === 1 ? "shape" : "shapes";
    lines.push(`${view.vanished} of ${view.of} ${shapes} ${view.vanished === 1 ? "vanishes" : "vanish"}`);
  }
  return { lines, invalid: view.stop === "nothing_left" };
}

/** The chip's Distance text: up to two decimals, no trailing zeros (`ux-rework.md` 2.3). */
export function chipDistanceText(mm: number): string {
  return String(Math.round(Math.abs(mm) * 100) / 100);
}

/** The typed Distance as a number (`.` or `,` as the decimal mark), or `NaN`. */
export function parseChipDistance(text: string): number {
  const trimmed = text.trim().replace(",", ".");
  return /^(\d+\.?\d*|\.\d+)$/.test(trimmed) ? Number(trimmed) : Number.NaN;
}

/** Up or Down in Distance (`ux-rework.md` 2.6): 0.1 mm, Shift 1 mm, Ctrl 0.01 mm, rounded to the
 * step and clamped to 0.01 to 1000 mm. */
export function stepChipDistance(
  mm: number,
  direction: 1 | -1,
  keys: { shift?: boolean; ctrl?: boolean },
): number {
  const step = keys.ctrl ? 0.01 : keys.shift ? 1 : 0.1;
  const stepped = Math.round((mm + direction * step) / step) * step;
  return Math.min(1000, Math.max(0.01, Math.round(stepped * 100) / 100));
}

/** The notice after a release or the chip's Enter (criterion 24), or `null` if nothing was written. */
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
  return text;
}

/** The refusal after a release or the chip's Enter (criteria 29, 32), or `null` for no refusal. */
export function offsetRefusalText(outcome: OffsetOutcome): string | null {
  const texts = textRefusalText(outcome);
  if (texts) {
    return texts;
  }
  switch (outcome.kind) {
    case "empty":
      return `Inset is empty: nothing is left at ${formatDistance(outcome.distanceMm)} mm. ${NOTHING_CHANGED}`;
    case "groups":
      return `Offset does not work on groups. Ungroup first. ${NOTHING_CHANGED}`;
    case "no_area":
    case "out_of_range":
      return sharedRefusalText("Offset", outcome);
    default:
      return null;
  }
}

/** What the canvas does with a key outcome of the session (criterion 13): `ignored` keeps the
 * page default; any other outcome is `preventDefault`-ed, so Alt during an Offset drag never
 * focuses a menu in the webview; `cornerStyle` (Alt stepped the drag's corner style) refreshes
 * the readout at the pointer. Auto-repeat is ignored by the session's key table. */
export function offsetKeyOutcome(outcome: string): { ignored: boolean; cornerStyle: boolean } {
  return { ignored: outcome === "ignored", cornerStyle: outcome === "offset-corner-style" };
}

/** A typed Distance clamped to 0.01 to 1000 mm (criterion 6): what the field shows after an
 * out-of-range Enter, selected so the next keystroke replaces it. */
export function clampChipDistance(mm: number): number {
  return Math.min(1000, Math.max(0.01, mm));
}
