/**
 * The canvas overlay of a selected test grid's picks (`specs/0146` `ux-notes.md` 6): the session
 * gives each marked cell's box in canvas CSS px (`pick_marks_json`); these pure functions decode it
 * and decide the frame offset and what the chip shows at the current zoom.
 */
import { asPickKind, type PickKind } from "./picks.ts";

/** One marked cell: its box, the pitch and gap on screen, and its picks in order. */
export interface PickMark {
  x: number;
  y: number;
  width: number;
  height: number;
  pitch: number;
  gap: number;
  kinds: PickKind[];
}

/** The marks in `text` (the session's JSON); none for anything that is not a list of marks. */
export function decodePickMarks(text: string): PickMark[] {
  let parsed: unknown;
  try {
    parsed = JSON.parse(text);
  } catch {
    return [];
  }
  if (!Array.isArray(parsed)) return [];
  const out: PickMark[] = [];
  for (const raw of parsed as unknown[]) {
    if (typeof raw !== "object" || raw === null) return [];
    const m = raw as Record<string, unknown>;
    const numbers = [m.x, m.y, m.width, m.height, m.pitch, m.gap];
    if (!numbers.every((n) => typeof n === "number" && Number.isFinite(n)) || !Array.isArray(m.kinds)) return [];
    const kinds = m.kinds.map(asPickKind);
    if (kinds.length === 0 || kinds.some((k) => k === null)) return [];
    out.push({
      x: m.x as number,
      y: m.y as number,
      width: m.width as number,
      height: m.height as number,
      pitch: m.pitch as number,
      gap: m.gap as number,
      kinds: kinds as PickKind[],
    });
  }
  return out;
}

/** How far the frame's inner edge sits outside the cell's box: `clamp(g / 2 - 3, 1, 2)` px (UX 6.2). */
export function frameOffset(gap: number): number {
  return Math.min(2, Math.max(1, gap / 2 - 3));
}

/** The chip's width for `count` glyphs: 16, 30, 44 or 58 px (UX 6.3). */
export function chipWidth(count: number): number {
  return 16 + 14 * (count - 1);
}

/** How many of a cell's picks the chip shows at this zoom: all when the row fits the pitch, the first
 * only when a 16 px chip fits, else none (the frames stay). */
export function chipGlyphs(count: number, pitch: number): number {
  if (chipWidth(count) <= pitch) return count;
  return pitch >= 16 ? 1 : 0;
}
