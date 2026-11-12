import type { SketchRow } from "@/lib/sketchPages";

/** Plain display data produced by the Rust page controller. */
export interface PageRow {
  id: string;
  name: string;
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
