import type { OperationId } from "@/lib/operationsView";

/** The active embroidery page as the session's `embroidery_view_json` gives it (`specs/0160`). */
export interface EmbroideryView {
  id: string;
  page: string;
  profile: {
    id: string | null;
    name: string;
    /** "DST". */
    output: string;
    /** Stitches per minute: the simulation's 1×. */
    speed: number;
    /** "12.1 mm". */
    maxStitch: string;
    /** The active hoop, "150 × 250 mm". */
    hoop: string;
  };
  profiles: { id: string; name: string; builtin: boolean }[];
  hoop: { key: string; sizes: { key: string; label: string }[] };
  operations: EmbroideryRow[];
  kinds: { kind: EmbroideryKind; label: string; description: string }[];
  checks: { severity: "block" | "warn"; text: string; items: OperationId[] }[];
  total: { time: string | null; runs: string; stitches: string };
  machineLine: string;
  exportAllowed: boolean;
  fileType: string;
  toolpath: boolean;
  play: EmbroideryPlay;
  /** The profile's speed in stitches per second: 1× of the play bar. */
  stitchesPerSecond: number;
  notice: string | null;
}

/** The three kinds (Frage 428). */
export type EmbroideryKind = "running" | "tatami" | "satin";

/** The play position and its readout (UX 6.2). */
export interface EmbroideryPlay {
  /** Entries of the stitch list shown sewn. */
  position: number;
  /** Entries of the stitch list. */
  length: number;
  /** "1,204 of 8,960". */
  text: string;
  /** The thread at the position, "#RRGGBB". */
  colour: string | null;
  /** "2 of 3". */
  colourText: string;
}

/** One operation row. */
export interface EmbroideryRow {
  id: OperationId;
  name: string;
  kind: EmbroideryKind | null;
  number: number;
  /** "#RRGGBB". */
  thread: string | null;
  /** The operation starts a new thread colour. */
  startsColour: boolean;
  line2: string;
  /** The row's sewing time ("1 min 4 s"), `null` with nothing sewn. */
  time: string | null;
  line3: string;
  numbers: {
    stitch_length_mm: number;
    repeats: number;
    angle_deg: number;
    row_spacing_mm: number;
    max_stitch_mm: number;
    zigzag_spacing_mm: number;
  } | null;
  output: boolean;
  unfolds: boolean;
}

/** The packed 0xRRGGBB of "#RRGGBB", black when it does not read. */
export function packedOf(hex: string | null): number {
  const n = hex && /^#[0-9a-fA-F]{6}$/.test(hex) ? Number.parseInt(hex.slice(1), 16) : 0;
  return Number.isFinite(n) ? n : 0;
}

/** "#RRGGBB" of a packed colour. */
export function hexOf(rgb: number): string {
  return `#${(rgb & 0xffffff).toString(16).toUpperCase().padStart(6, "0")}`;
}

/** The play position `seconds` after play started at `start`, at `speed` × `stitchesPerSecond`,
 * clamped to `length`: counted from the start, so no frame rounds a step away. */
export function positionAt(start: number, seconds: number, speed: number, stitchesPerSecond: number, length: number): number {
  return Math.min(length, start + Math.floor(Math.max(0, seconds) * speed * stitchesPerSecond));
}
