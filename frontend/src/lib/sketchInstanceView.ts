// The Parameters override of a clone of a sketch (`specs/0099-sketch-pages` criteria 24 and 26, UX
// notes 12.2): the decoder of the session's JSON and the adapter that lets a row ride on the same
// value-field maths as the Generator block's rows. Pure functions, so the tests load it without a
// bundler. The rules (clamps, notices, the solve) are Rust's.

import type { GeneratedRow } from "./generatedView.ts";

/** One exposed value as the clone block shows it. */
export interface SketchInstanceRow {
  /** `p:<id>` or `d:<id>`. */
  key: string;
  label: string;
  unit: "mm" | "deg";
  /** The shown value, `null` while the selected clones differ. */
  value: number | null;
  mixed: boolean;
  /** Where the value comes from: its own, inherited up a chain, the sketch's, or mixed. */
  origin: "own" | "inherited" | "sketch" | "mixed";
  sketchValue: number;
  min: number | null;
  max: number | null;
  trackMin: number;
  trackMax: number;
  trackLog: boolean;
  /** A selected clone follows another clone ("Back to the original's value"). */
  chained: boolean;
  /** The letter of its link group ("A"), `null` when it is in none (UX notes 12.9). */
  group: string | null;
  /** The labels of the other members of its group. */
  partners: string[];
  /** Whether the selected clones couple it (`linked`), hold it alone (`unlinked`) or differ. */
  link: "none" | "linked" | "unlinked" | "mixed";
}

/** The row of the pill and the fields. */
export interface SketchInstanceView {
  sketch: string;
  /** The sketch's name, for the fields' accessible names (UX notes 12.7). */
  sketchName: string;
  clones: number;
  /** The pressed segment: `original`, `own` or `mixed` (none pressed). */
  state: "original" | "own" | "mixed";
  noSolution: boolean;
  rows: SketchInstanceRow[];
}

/** The decoder of `sketch_instance_json`: `null` for no row. */
export function decodeSketchInstance(text: string): SketchInstanceView | null {
  let raw: unknown;
  try {
    raw = JSON.parse(text);
  } catch {
    return null;
  }
  if (raw === null || typeof raw !== "object") {
    return null;
  }
  const view = raw as Record<string, unknown> & { rows?: Record<string, unknown>[] };
  if (!Array.isArray(view.rows) || typeof view.state !== "string") {
    return null;
  }
  return {
    sketch: String(view.sketch),
    sketchName: String(view.sketch_name ?? ""),
    clones: Number(view.clones),
    state: view.state as SketchInstanceView["state"],
    noSolution: view.no_solution === true,
    rows: view.rows.map((row) => ({
      key: String(row.key),
      label: String(row.label),
      unit: row.unit === "deg" ? "deg" : "mm",
      value: typeof row.value === "number" ? row.value : null,
      mixed: row.mixed === true,
      origin: row.origin as SketchInstanceRow["origin"],
      sketchValue: Number(row.sketch_value),
      min: typeof row.min === "number" ? row.min : null,
      max: typeof row.max === "number" ? row.max : null,
      trackMin: Number(row.track_min),
      trackMax: Number(row.track_max),
      trackLog: row.track_log === true,
      chained: row.chained === true,
      group: typeof row.group === "string" ? row.group : null,
      partners: Array.isArray(row.partners) ? row.partners.map(String) : [],
      link: (["linked", "unlinked", "mixed"].includes(String(row.link)) ? row.link : "none") as SketchInstanceRow["link"],
    })),
  };
}

/** The farthest a value may be typed when its sketch sets no limit. */
const FAR = 1e7;

/** The row as the Generator block's value-field maths reads it (position on the track, steps,
 * clamp to the limits). */
export function asGeneratedRow(row: SketchInstanceRow): GeneratedRow {
  const angle = row.unit === "deg";
  const min = row.min ?? (angle ? -FAR : -FAR);
  const max = row.max ?? FAR;
  return {
    group: "",
    key: row.key,
    label: row.label,
    kind: angle
      ? { kind: "angle", min, max, step: 1, decimals: 1 }
      : { kind: "length", min, max, step: 1, decimals: 2 },
    track: { from: row.trackMin, to: row.trackMax, curve: row.trackLog ? "log" : "linear" },
    limit: null,
    state: null,
    used: null,
    problem: null,
    defaultValue: row.sketchValue,
    value: row.value ?? row.sketchValue,
  };
}

/** The reset slot's name and tooltip (UX notes 12.2): the sketch's value, or the original's up a
 * chain. */
export function resetName(row: SketchInstanceRow): string {
  return row.chained ? "Back to the original's value" : "Back to the sketch's value";
}

/** The line under the pill while a selected clone follows a value set without a solution. */
export const NO_SOLUTION_LINE = "No solution for these values";

/** The tail of a field's accessible value text: a value that follows the original says so. */
export function followsWords(row: SketchInstanceRow): string {
  return row.origin === "own" ? "" : row.origin === "inherited" ? ", original's value" : row.origin === "sketch" ? ", sketch's value" : "";
}

/** "3 clones go back to the sketch's value" (`specs/0099` UX notes 12.1): what Stop exposing does. */
export function stopTitle(holders: number, partners: string[] = []): string {
  // A linked member names the link first (UX notes 12.9.1).
  const link = partners.length > 0 ? `Leaves the link with ${partners.join(", ")}. ` : "";
  if (holders === 0) return `${link}Stop exposing: clones can no longer set this value`;
  return `${link}Stop exposing: ${holders} ${holders === 1 ? "clone goes" : "clones go"} back to the sketch's value`;
}

/** The tail of a field's accessible value text for a linked value (UX notes 12.9.5). */
export function linkWords(row: SketchInstanceRow): string {
  const partners = row.partners.join(", ");
  switch (row.link) {
    case "linked":
      return `, linked with ${partners}`;
    case "unlinked":
      return `, unlinked from ${partners}`;
    case "mixed":
      return ", linked on some clones";
    default:
      return "";
  }
}

/** The tooltip of a field's link mark (UX notes 12.9.2), empty when it shows none. */
export function linkMarkTitle(row: SketchInstanceRow): string {
  const partners = row.partners.join(", ");
  return row.link === "linked" ? `Linked with ${partners}` : row.link === "unlinked" ? `Unlinked from ${partners}` : "";
}

/** The open entry chip of a sketch clone's knob (`specs/0099` criterion 25). */
export interface SketchKnobEntry {
  key: string;
  label: string;
  unit: "mm" | "deg";
  value: number;
  /** The link mark after the label: coupled, held alone, or none. */
  glyph: "link" | "unlink" | "";
  /** The knob, in canvas CSS pixels. */
  x: number;
  y: number;
}

/** The decoder of `sketch_knob_entry_json`: `null` while no chip is open. */
export function decodeKnobEntry(text: string): SketchKnobEntry | null {
  if (text === "") {
    return null;
  }
  try {
    const raw = JSON.parse(text) as Record<string, unknown>;
    return {
      key: String(raw.key),
      label: String(raw.label),
      unit: raw.unit === "deg" ? "deg" : "mm",
      value: Number(raw.value),
      glyph: raw.glyph === "link" || raw.glyph === "unlink" ? raw.glyph : "",
      x: Number(raw.x),
      y: Number(raw.y),
    };
  } catch {
    return null;
  }
}
