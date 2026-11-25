// The library browser's geometry (`specs/0152-library-browser` criteria 2, 4 and 10, UX notes
// sections 3, 6 and 7): the wide or narrow layout, tiles of exactly Size + 16 px in columns from
// the left, the cards' own columns, the grid rows to put in the page, and the split pane's
// narrowest width as the font preview layout's size.

import { GRID_PAD_PX as FONT_GRID_PAD_PX, PANE_CHROME_PX } from "./fontPreviewLayout.ts";

/** From this pane width the facets are a column beside the grid (UX U4). */
export const WIDE_PANE_PX = 640;
/** The facet column's width. */
export const FACET_COLUMN_PX = 216;
/** The grid's padding and gaps. */
export const GRID_PAD_PX = 12;
export const GRID_GAP_PX = 8;
/** Under this Size a tile is its preview alone (UX U11). */
export const TEXT_FROM_SIZE = 96;
/** The cards' fixed size (UX U13). */
export const CARD_WIDTH_PX = 208;
export const CARD_HEIGHT_PX = 112;
/** The Not loaded heading's height. */
export const CARDS_HEADING_PX = 28;
/** The detail bar's height in the wide and the narrow layout. */
export const DETAIL_WIDE_PX = 113;
export const DETAIL_NARROW_PX = 89;

/** Whether a pane `width` wide gets the facet column. */
export function isWide(width: number): boolean {
  return width >= WIDE_PANE_PX;
}

/** A tile's width at `size`: the preview and 8 px padding each side. */
export function tileWidth(size: number): number {
  return size + 16;
}

/** A tile's height at `size`: with the name and meta lines from 96 px, else the preview alone. */
export function tileHeight(size: number): number {
  return size >= TEXT_FROM_SIZE ? size + 56 : size + 16;
}

/** The columns of items `width` wide fit in a grid of `gridWidth` (its inner width). */
export function columnsFor(gridWidth: number, itemWidth: number): number {
  const inner = Math.max(0, gridWidth - 2 * GRID_PAD_PX);
  return Math.max(1, Math.floor((inner + GRID_GAP_PX) / (itemWidth + GRID_GAP_PX)));
}

/** Which way the grid scrolled last: down (1) or up (-1). */
export type ScrollDirection = 1 | -1;

/** The rows in the page: those in view and one screen ahead in the scroll direction, never one
 * behind (criterion 6, ADR 0086 decision 7): a tile in the page asks for its picture. */
export function rowsInView(
  scrollTop: number,
  viewHeight: number,
  rowHeight: number,
  rowCount: number,
  direction: ScrollDirection = 1,
): { first: number; last: number } {
  if (rowCount === 0) {
    return { first: 0, last: -1 };
  }
  const top = direction === 1 ? scrollTop : scrollTop - viewHeight;
  const bottom = direction === 1 ? scrollTop + 2 * viewHeight : scrollTop + viewHeight;
  const first = Math.max(0, Math.floor(top / rowHeight));
  const last = Math.min(rowCount - 1, Math.max(first, Math.ceil(bottom / rowHeight) - 1));
  return { first, last };
}

/** The `size` the font preview's pane layout takes so that its narrowest split pane holds one
 * tile column at the browser's `size` (it is max(240, 5 x size) + padding there). */
export function paneLayoutSize(size: number): number {
  const pane = tileWidth(size) + 2 * GRID_PAD_PX + PANE_CHROME_PX;
  return Math.max(16, (pane - 2 * FONT_GRID_PAD_PX - PANE_CHROME_PX) / 5);
}
