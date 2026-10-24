// The rows of the Fonts tab's virtualised tree (`specs/0074-text-objects` UX notes §4,
// `specs/0075-font-library` UX tags amendment §2 and §3): which rows the blocks give, their
// heights (a family row of two lines, an expanded one by its wrapped tag lines), how a tag line
// fits its chips, and the block header pinned at the scroll top. Pure, so the tests read them.
import type { FontBlockRows, FontFace as ListedFace, FontFamily, FontsTabData } from "@/lib/textText";

export const HEADER_PX = 24;
/** A family row: name line and tag line. */
export const FAMILY_PX = 52;
export const NAME_LINE_PX = 28;
export const TAG_LINE_PX = 24;
export const FACE_PX = 28;

/** The tag line starts under the name (after the chevron) and ends 8 px from the edge. */
const TAG_LINE_INDENT = 28;
const TAG_LINE_GUTTER = 8;
/** The Add tag button and the gap between chips. */
export const ADD_TAG_PX = 20;
const CHIP_GAP = 4;

/** A tag on a family row: its id (built-in id or own name), name and source code. */
export interface RowTag {
  id: string;
  name: string;
  /** "you", "shipped", "declared:script", "detected:1", "" for an own tag. */
  source: string;
}

/** One row of the tree. */
export type Row =
  | { key: string; kind: "header"; block: FontBlockRows; height: number }
  | { key: string; kind: "missing"; name: string; height: number }
  | { key: string; kind: "family"; family: FontFamily; tags: RowTag[]; open: boolean; height: number }
  | { key: string; kind: "face"; face: ListedFace; family: FontFamily; height: number };

/** A family's row key: its block and name. */
export function familyKey(family: FontFamily): string {
  return `${family.block}:${family.family}`;
}

/** Measures text at 12 px. */
export type Measure = (text: string) => number;

let canvas: CanvasRenderingContext2D | null = null;

/** The width of `text` at 12 px in the page's UI font; an estimate where no canvas exists. */
export function textWidth(text: string): number {
  if (!canvas && typeof document !== "undefined") {
    canvas = document.createElement("canvas").getContext("2d");
    if (canvas) {
      canvas.font = `12px ${getComputedStyle(document.body).fontFamily}`;
    }
  }
  return canvas ? canvas.measureText(text).width : text.length * 6.5;
}

/** A tag chip: 1 px border, 6 px before the name, the 18 px remove zone, 1 px border. */
export function chipWidth(name: string, measure: Measure = textWidth): number {
  return Math.ceil(8 + measure(name) + 18);
}

/** The overflow chip "+N": 1 px border and 6 px padding each side. */
export function moreWidth(count: number, measure: Measure = textWidth): number {
  return Math.ceil(14 + measure(`+${count}`));
}

/** The room of a tag line in a tree `treeWidth` wide. */
export function tagLineRoom(treeWidth: number): number {
  return treeWidth - TAG_LINE_INDENT - TAG_LINE_GUTTER;
}

/** How many of `tags` a one-line tag line shows before "+N", leaving room for Add tag. */
export function shownTags(tags: RowTag[], room: number, measure: Measure = textWidth): number {
  const space = room - ADD_TAG_PX - CHIP_GAP;
  const widths = tags.map((tag) => chipWidth(tag.name, measure));
  const total = widths.reduce((sum, w) => sum + w + CHIP_GAP, 0);
  if (total <= space) {
    return tags.length;
  }
  let used = 0;
  let shown = 0;
  while (shown < tags.length) {
    const next = used + widths[shown] + CHIP_GAP;
    if (next + moreWidth(tags.length - shown - 1, measure) > space) {
      break;
    }
    used = next;
    shown += 1;
  }
  return shown;
}

/** The lines of an expanded family's tag line: every chip, Add tag at the end of the last. */
export function tagLines(tags: RowTag[], room: number, measure: Measure = textWidth): number {
  let lines = 1;
  let x = 0;
  for (const width of [...tags.map((tag) => chipWidth(tag.name, measure)), ADD_TAG_PX]) {
    if (x > 0 && x + width > room) {
      lines += 1;
      x = 0;
    }
    x += width + CHIP_GAP;
  }
  return lines;
}

interface RowInputs {
  data: FontsTabData;
  blocks: FontBlockRows[];
  /** Whether the family of this key is expanded. */
  isOpen: (key: string) => boolean;
  tags: (family: string) => RowTag[];
  /** The tree's inner width, measured (px). */
  treeWidth: number;
}

/** The rows: the Missing block, then each block's header and families; an expanded family shows
 * all its tag lines and adds its faces. */
export function buildRows({ data, blocks, isOpen, tags, treeWidth }: RowInputs): Row[] {
  const out: Row[] = [];
  if (data.current?.missing) {
    out.push({
      key: "missing-header",
      kind: "header",
      block: { block: "curvyo", title: "Missing", families: [], total: 1 },
      height: HEADER_PX,
    });
    out.push({ key: "missing", kind: "missing", name: `${data.current.family} ${data.current.style}`, height: FACE_PX });
  }
  for (const block of blocks) {
    out.push({ key: `header:${block.block}`, kind: "header", block, height: HEADER_PX });
    for (const family of block.families) {
      const key = familyKey(family);
      const carried = tags(family.family);
      const open = isOpen(key);
      const height = open ? NAME_LINE_PX + TAG_LINE_PX * tagLines(carried, tagLineRoom(treeWidth)) : FAMILY_PX;
      out.push({ key, kind: "family", family, tags: carried, open, height });
      if (!open) {
        continue;
      }
      for (const face of family.faces) {
        out.push({ key: `face:${face.index}`, kind: "face", face, family, height: FACE_PX });
      }
    }
  }
  return out;
}

/** Each row's top and the total height. */
export function rowTops(rows: Row[]): { at: number[]; total: number } {
  const at: number[] = [];
  let y = 0;
  for (const row of rows) {
    at.push(y);
    y += row.height;
  }
  return { at, total: y };
}

/** The header of the block at the scroll top once its own row has scrolled away, and how far
 * the next header pushes it up; `null` while the header row is in view. */
export function pinnedHeader(
  rows: Row[],
  tops: { at: number[] },
  scrollTop: number,
): { row: Extract<Row, { kind: "header" }>; push: number } | null {
  let at = -1;
  for (let i = 0; i < rows.length && tops.at[i] <= scrollTop; i += 1) {
    if (rows[i].kind === "header") {
      at = i;
    }
  }
  const row = rows[at];
  if (at < 0 || row.kind !== "header" || tops.at[at] >= scrollTop) {
    return null;
  }
  const next = rows.findIndex((r, i) => i > at && r.kind === "header");
  const push = next < 0 ? 0 : Math.min(0, tops.at[next] - scrollTop - HEADER_PX);
  return { row, push };
}
