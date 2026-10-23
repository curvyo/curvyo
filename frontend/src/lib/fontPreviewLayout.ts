// The font preview's geometry (`specs/0077-font-preview` criteria 2 to 4, 12, 13, 23 and 27, UX
// notes §1 to §3 and §6): the pane's width in split or full view, the dragged split, the header's rows, the tile grid's
// columns, the one-line sample area and tile height, and the grid rows to put in the page.

import { SIZE_RANGE } from "./fontPreviewState.ts";

/** The width the drawing keeps beside a split pane; below it the preview is full view only
 * (criterion 3). */
export const DRAWING_MIN_PX = 360;
/** The split pane's share of the canvas area (criterion 3, customer decision 2026-10-23). */
const SPLIT_SHARE = 0.35;
/** The pane's px outside the grid's scroll area: its 1px left border and a 17px scrollbar
 * gutter (the widest classic scrollbar), so one full column always fits. */
export const PANE_CHROME_PX = 18;
/** Below this pane width the header takes two rows (UX notes §2). */
export const ONE_ROW_HEADER_PX = 560;

/** Where the pane sits for a canvas area `areaWidth` wide. */
export interface PaneLayout {
  /** The pane fills the canvas area. */
  full: boolean;
  /** The Full view button is in the tree (not when the drawing would keep under 360px, criterion 3). */
  canSplit: boolean;
  /** The pane's width in px. */
  width: number;
}

/** The pane width that holds exactly one tile column at `size`: the column, the grid's padding
 * and the pane's chrome. */
export function oneColumnPane(size: number): number {
  return minColumnWidth(size) + 2 * GRID_PAD_PX + PANE_CHROME_PX;
}

/** The px a focused divider moves per Left or Right arrow (criterion 27). */
export const SPLIT_KEY_STEP_PX = 16;

/** A pane width held to one tile column at `size` and to a drawing of at least 360px. */
function clampedPane(areaWidth: number, size: number, width: number): number {
  return Math.min(Math.max(width, oneColumnPane(size)), areaWidth - DRAWING_MIN_PX);
}

/** The split pane's width: the dragged share of the canvas area (`split`), else 35 % of it or one
 * tile column if that is wider; held to one column and a 360px drawing (criteria 3 and 27). */
function splitWidth(areaWidth: number, size: number, split?: number): number {
  const wanted = split === undefined ? Math.max(areaWidth * SPLIT_SHARE, oneColumnPane(size)) : areaWidth * split;
  return Math.round(clampedPane(areaWidth, size, wanted));
}

/** Whether a split leaves the drawing at least 360px beside one tile column (criterion 3). */
function splitFits(areaWidth: number, size: number): boolean {
  return areaWidth - oneColumnPane(size) >= DRAWING_MIN_PX;
}

/** The share of the canvas area to keep after the divider was dragged (or moved by key) to a pane
 * `width` px wide, held to one column and a 360px drawing (criterion 27). */
export function splitAfterDrag(areaWidth: number, size: number, width: number): number {
  return Math.round(clampedPane(areaWidth, size, width)) / areaWidth;
}

/** The pane: in split view the dragged share `split` or 35 % of the canvas area, at least one
 * tile column at `size`; full view when chosen, when a forced full view is `held`, or when a split
 * would leave the drawing under 360px (criteria 3 and 27). */
export function paneLayout(
  areaWidth: number,
  size: number,
  wantFull: boolean,
  held = false,
  split?: number,
): PaneLayout {
  const canSplit = splitFits(areaWidth, size);
  const full = wantFull || held || !canSplit;
  return { full, canSplit, width: full ? areaWidth : splitWidth(areaWidth, size, split) };
}

/** Whether the forced full view holds: it starts when a split of the open pane's measured area
 * would leave the drawing under 360px and ends only when the preview closes (or the maker
 * switches, `switchedView`), so widening the window does not switch the view by itself
 * (criterion 3). */
export function heldFull(held: boolean, open: boolean, areaWidth: number, size: number): boolean {
  return open && (held || (areaWidth > 0 && !splitFits(areaWidth, size)));
}

/** The maker's Full view/Split view switch: it ends a hold; it toggles the stored choice unless it
 * only leaves a held full view, so the stored split choice is not overwritten (criterion 3). */
export function switchedView(wantFull: boolean, held: boolean): { wantFull: boolean; held: false } {
  return { wantFull: held && !wantFull ? false : !wantFull, held: false };
}

/** The px of the drawing the pane lies over: the split width; nothing in full view, where the
 * drawing is hidden under the pane, and nothing when closed. The drawing's canvas element is
 * this much wider than its visible box, so the split never resizes it (customer report
 * 2026-10-23: each canvas resize flickered on the Linux desktop). */
export function drawingLoss(open: boolean, pane: PaneLayout): number {
  return open && !pane.full ? pane.width : 0;
}

/** The CSS variable the canvas area sets to that overhang, in px; the drawing's canvas element
 * adds it to its width. */
export const CANVAS_OVERHANG_VAR = "--canvas-overhang";

/** The canvas width change between two states, for `keep_view_origin_for_panel_toggle` (negative
 * when the drawing narrows), so the drawing neither moves nor re-zooms (criteria 2 and 5). */
export function drawingWidthDelta(before: number, after: number): number {
  return before - after;
}

/** The header's rows for a pane `width` wide. */
export function headerRows(width: number): 1 | 2 {
  return width < ONE_ROW_HEADER_PX ? 2 : 1;
}

/** Grid padding and the gap between tiles. */
export const GRID_PAD_PX = 12;
export const GRID_GAP_PX = 12;
/** A tile's parts (UX notes §3): the footer is two rows, the name and the star, then the tag
 * line, the tag row reserved on every tile so all tiles of one Size are equally high (customer
 * request 2026-10-23: the tags get a row of their own). */
const TILE_TOP_PX = 12;
const TILE_FOOTER_GAP_PX = 8;
export const TILE_NAME_ROW_PX = 28;
export const TILE_TAG_ROW_PX = 24;
export const TILE_FOOTER_PX = TILE_NAME_ROW_PX + TILE_TAG_ROW_PX;
const TILE_BOTTOM_PX = 8;
/** The em of a face whose cap height is not known yet, as a share of the em. */
export const NOMINAL_CAP = 0.7;
/** Height of the sample's one line, in em. */
export const LINE_EM = 1.2;
/** The baseline's depth in the line, in em: every face sits on the same baseline. */
export const BASELINE_EM = 0.95;
/** Room above and below the line box, in em (no clipped ascenders). */
export const AREA_SLACK_EM = 0.25;

/** A tile's 16px side paddings, beside its tag row. */
const TILE_SIDES_PX = 32;

/** The room of a tile's tag row in a column `columnWidth` wide (criterion 14): chips beyond it
 * go into "+N" as in the list. */
export function tileTagRoom(columnWidth: number): number {
  return Math.max(0, columnWidth - TILE_SIDES_PX);
}

/** The narrowest column for a size: max(240px, 5 x size) (criterion 13). */
export function minColumnWidth(size: number): number {
  return Math.max(240, 5 * size);
}

/** Columns and their width for a grid `gridWidth` wide (the scroll area's inner width). */
export function gridColumns(gridWidth: number, size: number): { count: number; width: number } {
  const inner = Math.max(0, gridWidth - 2 * GRID_PAD_PX);
  const count = Math.max(1, Math.floor((inner + GRID_GAP_PX) / (minColumnWidth(size) + GRID_GAP_PX)));
  return { count, width: Math.max(0, (inner - GRID_GAP_PX * (count - 1)) / count) };
}

/** How far every tile's sample is drawn, in cap heights: a column is narrower than two of the
 * narrowest columns and a gap (else a second one would fit), widest at the smallest Size. One
 * drawing per face and sample then serves every Size and pane width; CSS scales and clips it. */
export const TILE_DRAW_CAPS = Math.ceil((2 * minColumnWidth(SIZE_RANGE.min) + GRID_GAP_PX) / SIZE_RANGE.min);

/** The CSS left and width of a tile in `column` of `count`, as shares of the grid's width: a
 * divider drag that keeps the column count re-flows the tiles in CSS only. */
export function tileColumnCss(column: number, count: number): { left: string; width: string } {
  const share = `(100% - ${2 * GRID_PAD_PX + (count - 1) * GRID_GAP_PX}px) / ${count}`;
  return {
    left: `calc(${GRID_PAD_PX}px + ${column} * (${share} + ${GRID_GAP_PX}px))`,
    width: `calc(${share})`,
  };
}

/** The em in px of a size (the cap height on screen) for the nominal cap height. */
export function emPx(size: number): number {
  return size / NOMINAL_CAP;
}

/** The sample area's height: one line box and the slack above and below (customer decision
 * 2026-10-23: one line, clipped at the tile's edge). */
export function sampleAreaHeight(size: number): number {
  return Math.ceil(emPx(size) * (LINE_EM + 2 * AREA_SLACK_EM));
}

/** A tile's height: equal for every tile of one size (criterion 13). */
export function tileHeight(size: number): number {
  return TILE_TOP_PX + sampleAreaHeight(size) + TILE_FOOTER_GAP_PX + TILE_FOOTER_PX + TILE_BOTTOM_PX;
}

/** The grid rows in the page: those in view plus one screen ahead (criterion 23), a row being a
 * tile and the gap under it. */
export function rowsInPage(
  scrollTop: number,
  viewHeight: number,
  rowHeight: number,
  rowCount: number,
): { first: number; last: number } {
  if (rowCount <= 0 || rowHeight <= 0) {
    return { first: 0, last: -1 };
  }
  const top = Math.max(0, scrollTop - GRID_PAD_PX);
  const first = Math.min(rowCount - 1, Math.floor(top / rowHeight));
  const last = Math.min(rowCount - 1, Math.floor((top + 2 * viewHeight) / rowHeight));
  return { first, last };
}

/** Whether a measured grid area `b` changes nothing the grid lays out against `a`: the same
 * height and, while the divider is dragged (`resizing`), the same column count at `size`, else
 * the same width. */
export function sameGridArea(
  a: { width: number; height: number },
  b: { width: number; height: number },
  resizing: boolean,
  size: number,
): boolean {
  if (a.height !== b.height) {
    return false;
  }
  return resizing ? gridColumns(a.width, size).count === gridColumns(b.width, size).count : a.width === b.width;
}
