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
  xText: string;
  yText: string;
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
}
export function readPages(json: string): PageRow[] {
  return JSON.parse(json) as PageRow[];
}

/** How a screen reader reads a page: "Keychain, drawing page" (`specs/0098` UX notes §1). */
export function pageReading(name: string, word: string): string {
  return `${name}, ${word.toLowerCase()} page`;
}
