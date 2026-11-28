import type { SketchRow } from "@/lib/sketchPages";

/** Plain display data produced by the Rust page controller. */
export interface PageRow {
  id: string;
  name: string;
  /** "From Box job › Lid" for an imported page (`specs/0137` criterion 7). */
  cameFrom?: string | null;
  size: string;
  active: boolean;
  frameOnly: boolean;
  /** `drawing`, `sketch`, an output process id or `unknown` (`specs/0098` criterion 6). */
  kind: string;
  /** The kind word of the chip. */
  kindWord: string;
  /** The Lucide name of the chip's glyph. */
  kindIcon: string;
  /** An output page whose extension is switched off. */
  kindOff: boolean;
  /** The page's size comes from its machine or its kind is unknown: no size fields. */
  fixedSize: boolean;
  /** The machine name of a laser page's copy, `null` on another page. */
  machine: string | null;
  /** The full id of a laser page's manufacturing, `null` on another page. */
  manufacturing: string | null;
  x: number;
  y: number;
  screenX: number;
  screenY: number;
  screenWidth: number;
  screenHeight: number;
  /** A sketch page's state and clone count (`specs/0099` criteria 14, 16), `null` otherwise. */
  sketch: SketchRow | null;
  /** The page a page-row drag would drop onto now (its frame cue, UX notes §2). */
  dropTarget: boolean;
  /** A laser page's Operations view toggle: pressed or not (`specs/0111` criterion 15); `null` on
   * any other page. */
  operationsView?: boolean | null;
  /** The same label's Toolpath view toggle (`specs/0029` criterion 8): pressed or not; `null` on
   * any other page. Exclusive with the Operations view. */
  toolpathView?: boolean | null;
  /** A move-only selection's object belongs here: the label is semibold (`specs/0106` UX §2). */
  home?: boolean;
}
export function readPages(json: string): PageRow[] {
  return JSON.parse(json) as PageRow[];
}

/** How a screen reader reads a page: "Keychain, drawing page" (`specs/0098` UX notes §1). */
export function pageReading(name: string, word: string): string {
  return `${name}, ${word.toLowerCase()} page`;
}

/** Whether the page label draws the Operations view toggle: a laser page only, `operationsView`
 * is pressed or not there and `null` elsewhere (`specs/0111` criterion 15). */
export function operationsToggle(page: PageRow): { pressed: boolean } | null {
  return typeof page.operationsView === "boolean" ? { pressed: page.operationsView } : null;
}

/** The accessible name and tooltip of the toggle; the pressed state is `aria-pressed`. */
export const OPERATIONS_VIEW_NAME = "Operations view";

/** Whether the page label draws the Toolpath view toggle after the Operations view toggle. */
export function toolpathToggle(page: PageRow): { pressed: boolean } | null {
  return typeof page.toolpathView === "boolean" ? { pressed: page.toolpathView } : null;
}

/** The accessible name and tooltip of the Toolpath view toggle. */
export const TOOLPATH_VIEW_NAME = "Toolpath view";

/** The page label's height and its gap to the frame or to a bar above it, px. */
export const PAGE_LABEL_HEIGHT = 16;
export const PAGE_LABEL_GAP = 4;

/** Where a page's name label sits, as its top relative to the page frame: 20 px above the frame,
 * or, where a bar covers that place and the page reaches below the bar, 4 px below the bar's
 * bottom (inside the frame), so the label is never under a bar of any height (`specs/0159` F50,
 * Proposal). `bars` is the bars' box in the same coordinates as the page, or null for none. */
export function pageLabelTop(
  page: Pick<PageRow, "screenX" | "screenY" | "screenWidth" | "screenHeight">,
  bars: { left: number; right: number; bottom: number } | null,
): number {
  const above = -(PAGE_LABEL_HEIGHT + PAGE_LABEL_GAP);
  if (!bars) return above;
  const overlaps = page.screenX < bars.right && page.screenX + page.screenWidth > bars.left;
  const covered = page.screenY + above < bars.bottom + PAGE_LABEL_GAP;
  const reachesBelow = page.screenY + page.screenHeight > bars.bottom + PAGE_LABEL_GAP + PAGE_LABEL_HEIGHT;
  return overlaps && covered && reachesBelow ? bars.bottom + PAGE_LABEL_GAP - page.screenY : above;
}
