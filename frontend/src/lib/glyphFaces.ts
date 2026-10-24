// The glyph table's engine calls (`specs/0082-glyph-table` criteria 10, 16, 17, 19 and 21,
// `adrs.md` decisions 2 and 3): the table of a face for a query, its cells drawn in one call per
// batch and kept per face and character (whatever the Size and width, at most 32 MiB of path
// text), the detail bar's names, and the Unicode names file fetched once on the first Glyphs
// open.
import { wasmReady } from "@/lib/editorSession";
import { faceId, previewEngine } from "@/lib/previewFaces";
import type { FontFace as ListedFace } from "@/lib/textText";

/** A face's table for a query, as the engine groups it. */
export interface GlyphTableData {
  total: number;
  shown: number;
  single_line: boolean;
  groups: { name: string; cells: number[] }[];
}

/** One cell's drawing in cap-height units (y down, origin on the baseline). */
export interface CellDrawing {
  d: string;
  adv: number;
  empty: boolean;
}

/** The detail bar's names of one character. */
export interface GlyphDetail {
  name: string | null;
  glyph_name: string | null;
  block: string;
  abbreviation: string | null;
}

/** The path text kept per face at most (`adrs.md` decision 3). */
const MAX_CELL_TEXT = 32 * 1024 * 1024;

/** Drawn cells per face id, the least recently shown first, and their path text length. */
const drawn = new Map<string, { cells: Map<number, CellDrawing | null>; size: number }>();

/** The table of `face` for `query`; `null` while the face is not held. */
export function glyphTableOf(face: ListedFace, query: string): GlyphTableData | null {
  const json = previewEngine().glyph_table(faceId(face), query);
  return json ? (JSON.parse(json) as GlyphTableData) : null;
}

/** The drawing of `cp` in `face` if drawn: a cell, `null` for no glyph, `undefined` not yet. */
export function cachedCell(face: ListedFace, cp: number): CellDrawing | null | undefined {
  return drawn.get(faceId(face))?.cells.get(cp);
}

/** Draws the cells of `cps` not drawn yet in one engine call; returns whether any was drawn. */
export function drawCells(face: ListedFace, cps: readonly number[]): boolean {
  const id = faceId(face);
  let held = drawn.get(id);
  if (!held) {
    held = { cells: new Map(), size: 0 };
    drawn.set(id, held);
  }
  const missing = cps.filter((cp) => !held.cells.has(cp));
  if (missing.length === 0) {
    return false;
  }
  const json = previewEngine().glyph_cells(id, Uint32Array.from(missing));
  if (!json) {
    return false;
  }
  const cells = JSON.parse(json) as (CellDrawing | null)[];
  missing.forEach((cp, i) => {
    const cell = cells[i] ?? null;
    held.cells.set(cp, cell);
    held.size += cell?.d.length ?? 0;
  });
  // A very large face may pass the cap: the cells drawn first go first.
  for (const [cp, cell] of held.cells) {
    if (held.size <= MAX_CELL_TEXT) {
      break;
    }
    held.cells.delete(cp);
    held.size -= cell?.d.length ?? 0;
  }
  return true;
}

/** The detail bar's names of `cp` in `face`. */
export function glyphDetailOf(face: ListedFace, cp: number): GlyphDetail | null {
  const json = previewEngine().glyph_detail(faceId(face), cp);
  return json ? (JSON.parse(json) as GlyphDetail) : null;
}

/** The shipped names file (`tools/unicode-data/README.md`). */
const NAMES_FILE = "unicode/names-17.0.0.bin";

let names: "idle" | "loading" | "loaded" | "failed" = "idle";
const listeners = new Set<() => void>();

/** Whether the character names are loaded. */
export function namesLoaded(): boolean {
  return names === "loaded";
}

/** Whether the names are still on the way (a search by name may find more once they arrive). */
export function namesLoading(): boolean {
  return names === "idle" || names === "loading";
}

export function subscribeNames(listener: () => void): () => void {
  listeners.add(listener);
  return () => listeners.delete(listener);
}

/** Fetches the names file once (the first Glyphs open, criterion 17) and hands it to the engine;
 * the listeners hear when it is in. A failed fetch leaves search by name empty. */
export function loadUnicodeNames(): void {
  if (names !== "idle") {
    return;
  }
  names = "loading";
  void Promise.all([wasmReady(), fetch(`${import.meta.env.BASE_URL}${NAMES_FILE}`)])
    .then(async ([, response]) => {
      if (!response.ok) {
        throw new Error(`${NAMES_FILE}: ${response.status}`);
      }
      const bytes = new Uint8Array(await response.arrayBuffer());
      names = previewEngine().set_unicode_names(bytes) ? "loaded" : "failed";
    })
    .catch(() => {
      names = "failed";
    })
    .finally(() => {
      for (const listener of listeners) {
        listener();
      }
    });
}

/** Details asked for by face and code point (accessible names, empty-cell labels, the detail
 * bar); dropped when the names arrive and when they grow past `MAX_DETAILS`. */
const details = new Map<string, GlyphDetail | null>();
const MAX_DETAILS = 20_000;

/** The detail of `cp` in `face`, asked of the engine once. */
export function cachedDetail(face: ListedFace, cp: number): GlyphDetail | null {
  const key = `${faceId(face)}|${cp}`;
  let detail = details.get(key);
  if (detail === undefined) {
    if (details.size >= MAX_DETAILS) {
      details.clear();
    }
    detail = glyphDetailOf(face, cp);
    details.set(key, detail);
  }
  return detail;
}

subscribeNames(() => details.clear());
