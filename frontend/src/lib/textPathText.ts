// The words and states of the On path button, Release from path in the Text bar's second row and
// the entry box of the position handle (`specs/0080-text-on-path` criteria 1, 2, 17 to 19, 23,
// 48; UX notes §1, §5; customer 2026-10-24). No React, so node tests can check them; every value
// comes from the session.

/** The two sides and the two lines on path, in the groups' order (the session's codes). */
export const SIDE_CODES = ["above", "below"] as const;
export type SideCode = (typeof SIDE_CODES)[number];
export const LINE_CODES = ["baseline", "top"] as const;
export type LineCode = (typeof LINE_CODES)[number];

/** Each button's accessible name and tooltip (UX notes §5: the control only). */
export const SIDE_NAMES: Record<SideCode, string> = { above: "Above", below: "Below" };
export const LINE_NAMES: Record<LineCode, string> = { baseline: "Baseline", top: "Top edge" };

/** What the session says about the On path button and the row-2 dot (`text_path_view_json`). */
export interface TextPathBar {
  /** The number of subject texts: the button acts with exactly one. */
  texts: number;
  /** Pick mode runs: the button is pressed. */
  picking: boolean;
  /** How many subject texts are on a path: Release from path shows when it is not 0. */
  onPath: number;
  /** A subject text on path is vertical: Writing mode and Glyph orientation are greyed. */
  vertical: boolean;
}

export const EMPTY_TEXT_PATH: TextPathBar = {
  texts: 0,
  picking: false,
  onPath: 0,
  vertical: false,
};

const finite = (value: unknown) => (typeof value === "number" && Number.isFinite(value) ? value : null);

/** The session's view JSON, or the empty view for anything unreadable. */
export function parseTextPath(json: string | undefined): TextPathBar {
  try {
    const raw = JSON.parse(json ?? "") as Partial<Record<keyof TextPathBar, unknown>>;
    return {
      texts: finite(raw.texts) ?? 0,
      picking: raw.picking === true,
      onPath: finite(raw.onPath) ?? 0,
      vertical: raw.vertical === true,
    };
  } catch {
    return EMPTY_TEXT_PATH;
  }
}

/** The entry box of the position handle (`text_path_entry_json`, customer 2026-10-24): where
 * the handle is (canvas px) and what the box shows. */
export interface TextPathEntry {
  handle: { x: number; y: number };
  side: SideCode;
  line: LineCode;
  /** mm, or degrees on an ellipse carrier. */
  position: number;
  degrees: boolean;
  /** mm. */
  distance: number;
}

/** The open entry box, or `null` when it is closed or the JSON is damaged. */
export function parseTextPathEntry(json: string | undefined): TextPathEntry | null {
  try {
    const raw = JSON.parse(json ?? "") as Record<string, unknown>;
    const handle = raw.handle as { x?: unknown; y?: unknown } | undefined;
    const x = finite(handle?.x);
    const y = finite(handle?.y);
    const position = finite(raw.position);
    const distance = finite(raw.distance);
    const side = (SIDE_CODES as readonly unknown[]).includes(raw.side) ? (raw.side as SideCode) : null;
    const line = (LINE_CODES as readonly unknown[]).includes(raw.line) ? (raw.line as LineCode) : null;
    if (x === null || y === null || position === null || distance === null || !side || !line) {
      return null;
    }
    return { handle: { x, y }, side, line, position, degrees: raw.degrees === true, distance };
  } catch {
    return null;
  }
}

/** The On path button acts with exactly one subject text (UX notes §0.6). */
export function onPathEnabled(bar: TextPathBar): boolean {
  return bar.texts === 1;
}

/** The tooltip's second line: the state that disables the button ("3 texts selected"), else
 * none. */
export function onPathTooltipLine(bar: TextPathBar): string | null {
  return bar.texts > 1 ? `${bar.texts} texts selected` : null;
}

/** What a screen reader hears after a press: the put, the locked text, nothing else. */
export function onPathPressNotice(outcome: string): string | null {
  if (outcome === "put") {
    return "Put on path";
  }
  return null;
}

/** A field's value as the entry box opens with it: one decimal. */
export function pathFieldText(value: number): string {
  return (Math.round(value * 10) / 10).toFixed(1);
}

/** The unit of the Position field: "°" on an ellipse carrier, else "mm". */
export function positionUnit(entry: TextPathEntry): string {
  return entry.degrees ? "°" : "mm";
}

/** The second tooltip line of the greyed Writing mode and Glyph orientation buttons while a
 * vertical text is on a path (criterion 48). */
export const ON_PATH_WRITING_LINE = "Text on a path runs along the path";

/** Whether the Writing mode and Glyph orientation groups are greyed (criterion 48). */
export function writingGreyedOnPath(bar: TextPathBar): boolean {
  return bar.onPath > 0 && bar.vertical;
}
