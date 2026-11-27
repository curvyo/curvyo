import type { OperationId } from "@/lib/operationsView";

/** The active CNC page as the session's `cnc_view_json` gives it (`specs/0161`). */
export interface CncView {
  id: string;
  page: string;
  profile: {
    id: string | null;
    name: string;
    builtin: boolean;
    /** "GRBL". */
    output: string;
    /** "300 × 300 mm". */
    workArea: string;
    zTravel: number;
    spindleMax: number;
    safeHeight: number;
    /** The connector address of a user profile; `null` on a built-in. */
    connector: string | null;
  };
  profiles: { id: string; name: string; builtin: boolean }[];
  stock: { width: number; height: number; thickness: number; material: string };
  tools: CncTool[];
  operations: CncRow[];
  kinds: { side: CncSide; label: string; description: string }[];
  checks: { severity: "block" | "warn"; text: string; items: OperationId[]; operation: string | null }[];
  total: { time: string | null; runs: string; lengths: string };
  machineLine: string;
  exportAllowed: boolean;
  fileType: string;
  toolpath: boolean;
  /** The play position and the planner's total in seconds, with the readout. */
  play: { position: number; length: number; text: string };
  /** Send to machine: present while the profile names a connector. */
  send: { profileId: string; address: string; enabled: boolean } | null;
  notice: string | null;
}

/** The two sides of a Profile (criterion 5). */
export type CncSide = "outside" | "inside";

/** One end mill of the page's tool list. */
export interface CncTool {
  id: string;
  /** "Flat 3.175 mm". */
  name: string;
  /** "2 flutes · 800 mm/min · 12,000 rpm · 1 mm down". */
  line2: string;
  numbers: {
    diameter_mm: number;
    flutes: number;
    feed_mm_min: number;
    plunge_mm_min: number;
    rpm: number;
    step_down_mm: number;
  };
}

/** One operation row. */
export interface CncRow {
  id: OperationId;
  name: string;
  number: number;
  side: CncSide | null;
  line2: string;
  line3: string;
  time: string | null;
  passes: number;
  numbers: {
    side: CncSide;
    direction: "conventional" | "climb";
    depth_mm: number;
    tool: string;
    feed_mm_min: number;
    plunge_mm_min: number;
    feedTyped: boolean;
    plungeTyped: boolean;
    toolFeed: number;
    toolPlunge: number;
  } | null;
  output: boolean;
  unfolds: boolean;
}

/** The play position `seconds` after play started at `start` at `speed` ×, clamped to `length`. */
export function cncPositionAt(start: number, seconds: number, speed: number, length: number): number {
  return Math.min(length, start + Math.max(0, seconds) * speed);
}
