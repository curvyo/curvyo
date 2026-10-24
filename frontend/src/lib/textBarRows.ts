// The words of the Text bar's Font jump button and expand button, and the stored row 2 state
// (`specs/0084-text-bar-second-row` criteria 2, 3, 11, 13, 14). No React, so node tests can check
// them. Row 2 open or closed is one per-user preference in the webview's own storage, outside the
// project (`adrs.md`, dated note): every access is guarded, and anything unreadable means closed.

import type { OrientationCode, WritingModeCode } from "@/lib/textWritingText";

/** The storage key; the value is the JSON boolean `true` while row 2 is open. */
export const TEXT_BAR_ROW2_KEY = "curvyo.text-bar-row2";

/** The part of `Storage` this module uses, so a test can hand in a fake. */
export interface BarStorage {
  getItem(key: string): string | null;
  setItem(key: string, value: string): void;
}

/** The webview's `localStorage`, or `undefined` when even reaching it throws. */
function defaultStorage(): BarStorage | undefined {
  try {
    return window.localStorage;
  } catch {
    return undefined;
  }
}

/** Whether row 2 was open when the maker last toggled it; a new user starts closed. */
export function readRow2Open(storage: BarStorage | undefined = defaultStorage()): boolean {
  try {
    return storage?.getItem(TEXT_BAR_ROW2_KEY) === "true";
  } catch {
    return false;
  }
}

/** Remembers whether row 2 is open. A failing write is ignored. */
export function writeRow2Open(open: boolean, storage: BarStorage | undefined = defaultStorage()): void {
  try {
    storage?.setItem(TEXT_BAR_ROW2_KEY, JSON.stringify(open));
  } catch {
    /* Not kept; the next start shows row 2 closed. */
  }
}

/** What the Font button names: the face, "" for "Mixed", and whether it is missing. */
export interface FontSubject {
  font: string;
  missing: boolean;
}

/** "Font: Source Sans 3 Regular", "Font: Mixed", "Font: Lobster Regular, missing". */
export function fontButtonName({ font, missing }: FontSubject): string {
  return `Font: ${font || "Mixed"}${missing ? ", missing" : ""}`;
}

/** How many different faces the subject texts use and how many of them are missing (UX notes
 * §0.4); read only for "Mixed". */
export interface FaceCounts {
  faces?: number;
  missingFaces?: number;
}

/** The tooltip's lines: the command (with its key on the desktop), the face or "Mixed", and the
 * missing line as in `0074`, for "Mixed" "1 of 3 fonts missing on this computer." */
export function fontTooltipLines({
  font,
  missing,
  substitute,
  desktop,
  faces = 0,
  missingFaces = 0,
}: FontSubject & FaceCounts & { substitute: string; desktop: boolean }): string[] {
  const lines = [desktop ? "Font (Shift+Ctrl+T)" : "Font", font || "Mixed"];
  if (missing && font) {
    lines.push(`Missing on this computer. Shown in ${substitute}.`);
  } else if (missing) {
    lines.push(`${missingFaces} of ${faces} fonts missing on this computer.`);
  }
  return lines;
}

/** The expand button's accessible name and tooltip: what a press does. */
export function expandLabel(open: boolean): string {
  return open ? "Fewer text options" : "More text options";
}

/** The row 2 values of the subject texts; `null` is Mixed. */
export interface Row2Values {
  writingMode: WritingModeCode | null;
  orientation: OrientationCode | null;
}

/** The row 2 controls whose value differs from the default, in row order. Mixed counts as
 * changed; the glyph orientation counts only while some subject text is vertical, since it acts on
 * nothing else (UX notes §3). */
export function changedRow2Controls({ writingMode, orientation }: Row2Values): string[] {
  const changed: string[] = [];
  if (writingMode !== "horizontal") {
    changed.push("Writing mode");
    if (orientation !== "auto") {
      changed.push("Glyph orientation");
    }
  }
  return changed;
}

/** The expand button's description while the dot shows ("Changed: Writing mode"), else `null`. */
export function changedText(changed: readonly string[]): string | null {
  return changed.length > 0 ? `Changed: ${changed.join(", ")}` : null;
}
