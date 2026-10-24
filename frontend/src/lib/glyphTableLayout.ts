// The glyph table's geometry (`specs/0082-glyph-table` criteria 6, 15, 18, 20 and 21, UX notes
// P2, P3, P6, §3 and §7): square cells of side max(40px, 2 x size) with 4px gaps from the left,
// block header rows of 28px, the rows in the page as keyed items, the sticky header, and where a
// key or a click leaves the focus.

/** Grid padding, the gap between cells, a header row and the space before the next header. */
export const GLYPH_PAD_PX = 12;
export const GLYPH_GAP_PX = 4;
export const GLYPH_HEADER_PX = 28;
export const GLYPH_BLOCK_GAP_PX = 12;
/** The detail bar's fixed height, its 1px line included (UX notes P4). */
export const DETAIL_BAR_PX = 97;
/** The baseline's depth in a cell, as a share of its side (UX notes §3). */
export const CELL_BASELINE = 0.68;

/** A cell's side for a Size (cap height in px). */
export function cellSide(size: number): number {
  return Math.max(40, 2 * size);
}

/** The columns of cells that fit a scroll area `gridWidth` wide (its inner width, padding
 * included): columns of exactly the cell side, laid from the left. */
export function glyphColumns(gridWidth: number, size: number): number {
  const side = cellSide(size);
  const inner = Math.max(0, gridWidth - 2 * GLYPH_PAD_PX);
  return Math.max(1, Math.floor((inner + GLYPH_GAP_PX) / (side + GLYPH_GAP_PX)));
}

/** One group of the table: a block (or Recent) and its cells' code points. */
export interface GlyphGroupCells {
  name: string;
  cells: number[];
}

/** A row of the laid-out grid: a block header, or up to `columns` cells from flat index `first`. */
export type GlyphRow =
  | { kind: "header"; group: number; top: number }
  | { kind: "cells"; group: number; first: number; count: number; top: number };

export interface GlyphGridLayout {
  side: number;
  columns: number;
  rows: GlyphRow[];
  /** The whole grid's height in px. */
  height: number;
  /** Every cell's code point in table order. */
  cells: number[];
  /** Per flat cell index, its row. */
  rowOf: number[];
  /** Per group, the flat index of its first cell. */
  groupStart: number[];
}

/** Lays out `groups` in `columns` columns of cells of `side` px. */
export function layoutGlyphGrid(groups: readonly GlyphGroupCells[], columns: number, side: number): GlyphGridLayout {
  const rows: GlyphRow[] = [];
  const cells: number[] = [];
  const rowOf: number[] = [];
  const groupStart: number[] = [];
  let top = GLYPH_PAD_PX;
  groups.forEach((group, index) => {
    if (index > 0) {
      top += GLYPH_BLOCK_GAP_PX;
    }
    rows.push({ kind: "header", group: index, top });
    top += GLYPH_HEADER_PX;
    groupStart.push(cells.length);
    for (let at = 0; at < group.cells.length; at += columns) {
      const count = Math.min(columns, group.cells.length - at);
      if (at > 0) {
        top += GLYPH_GAP_PX;
      }
      rows.push({ kind: "cells", group: index, first: cells.length, count, top });
      for (let i = 0; i < count; i += 1) {
        rowOf.push(rows.length - 1);
        cells.push(group.cells[at + i]);
      }
      top += side;
    }
  });
  return { side, columns, rows, height: top + GLYPH_PAD_PX, cells, rowOf, groupStart };
}

/** A row's height. */
export function rowHeight(row: GlyphRow, side: number): number {
  return row.kind === "header" ? GLYPH_HEADER_PX : side;
}

/** The rows to put in the page: those in view and one screen below (criterion 21). */
export function glyphRowsInPage(layout: GlyphGridLayout, scrollTop: number, viewHeight: number): { first: number; last: number } {
  const { rows, side } = layout;
  if (rows.length === 0) {
    return { first: 0, last: -1 };
  }
  const bottom = scrollTop + 2 * Math.max(0, viewHeight);
  let first = 0;
  let lo = 0;
  let hi = rows.length - 1;
  while (lo <= hi) {
    const mid = (lo + hi) >> 1;
    if (rows[mid].top + rowHeight(rows[mid], side) < scrollTop) {
      lo = mid + 1;
    } else {
      first = mid;
      hi = mid - 1;
    }
  }
  let last = first;
  while (last + 1 < rows.length && rows[last + 1].top <= bottom) {
    last += 1;
  }
  return { first, last };
}

/** One header or cell of the page, with a key that stays the same while the page scrolls. */
export type GlyphPageItem =
  | { kind: "header"; key: string; top: number }
  | { kind: "cell"; key: string; index: number; cp: number; column: number; top: number };

/** The page's items per group, in order (criterion 20: a cell keeps its key, and so its element
 * and the focus, when rows above it leave the page). */
export function glyphPageItems(layout: GlyphGridLayout, page: { first: number; last: number }): { group: number; items: GlyphPageItem[] }[] {
  const out: { group: number; items: GlyphPageItem[] }[] = [];
  for (const row of layout.rows.slice(page.first, page.last + 1)) {
    if (out.at(-1)?.group !== row.group) {
      out.push({ group: row.group, items: [] });
    }
    const items = out[out.length - 1].items;
    if (row.kind === "header") {
      items.push({ kind: "header", key: "header", top: row.top });
    } else {
      layout.cells.slice(row.first, row.first + row.count).forEach((cp, column) => {
        items.push({ kind: "cell", key: String(cp), index: row.first + column, cp, column, top: row.top });
      });
    }
  }
  return out;
}

/** Whether the focus stays on the cell after an insert (criterion 12): by keyboard, or by a click
 * in full view where the canvas is hidden; a click in split view returns it to the text. */
export function focusStaysOnCell(byKeyboard: boolean, full: boolean): boolean {
  return byKeyboard || full;
}

/** The sticky header at `scrollTop` (UX notes P6): the group whose header scrolled past the top,
 * and how far the next header pushes it up (0 or negative px). `null` above the first header. */
export function stickyHeader(layout: GlyphGridLayout, scrollTop: number): { group: number; offset: number } | null {
  const headers = layout.rows.filter((row) => row.kind === "header");
  let current: GlyphRow | null = null;
  let next: GlyphRow | null = null;
  for (const header of headers) {
    if (header.top <= scrollTop) {
      current = header;
    } else {
      next = header;
      break;
    }
  }
  if (!current) {
    return null;
  }
  const offset = next ? Math.min(0, next.top - scrollTop - GLYPH_HEADER_PX) : 0;
  return { group: current.group, offset };
}

function cellRows(layout: GlyphGridLayout): number[] {
  return layout.rows.flatMap((row, index) => (row.kind === "cells" ? [index] : []));
}

/** The flat index the key moves the focus to from `index`, or `null` for a key that does not
 * move (criterion 20, UX notes P5 and §4): Left and Right in reading order, Up and Down by a
 * visual row in the same column (a shorter row: its last cell), Home and End in the row,
 * Ctrl+Home and Ctrl+End to the first and last cell, Page Up and Page Down by `pageHeight` px,
 * Ctrl+Up and Ctrl+Down to the first cell of the previous or next block. */
export function glyphMove(key: string, ctrl: boolean, index: number, layout: GlyphGridLayout, pageHeight: number): number | null {
  const count = layout.cells.length;
  if (count === 0) {
    return null;
  }
  const at = Math.min(count - 1, Math.max(0, index));
  const rowIndex = layout.rowOf[at];
  const row = layout.rows[rowIndex];
  if (row.kind !== "cells") {
    return null;
  }
  const column = at - row.first;
  const inRow = (target: GlyphRow) => (target.kind === "cells" ? target.first + Math.min(column, target.count - 1) : at);
  const rows = cellRows(layout);
  const position = rows.indexOf(rowIndex);
  switch (key) {
    case "ArrowLeft":
      return Math.max(0, at - 1);
    case "ArrowRight":
      return Math.min(count - 1, at + 1);
    case "ArrowUp":
    case "ArrowDown": {
      if (ctrl) {
        const group = row.group + (key === "ArrowUp" ? -1 : 1);
        const start = layout.groupStart[group];
        return start === undefined ? at : start;
      }
      const target = rows[position + (key === "ArrowUp" ? -1 : 1)];
      return target === undefined ? at : inRow(layout.rows[target]);
    }
    case "Home":
      return ctrl ? 0 : row.first;
    case "End":
      return ctrl ? count - 1 : row.first + row.count - 1;
    case "PageUp":
    case "PageDown": {
      const y = row.top + (key === "PageUp" ? -1 : 1) * Math.max(layout.side, pageHeight);
      let best = rows[0];
      for (const candidate of rows) {
        if (Math.abs(layout.rows[candidate].top - y) < Math.abs(layout.rows[best].top - y)) {
          best = candidate;
        }
      }
      return inRow(layout.rows[best]);
    }
    default:
      return null;
  }
}

/** The scroll position that shows the cell `index` below the sticky header, or `null` when it
 * is in view already. */
export function scrollToCell(layout: GlyphGridLayout, index: number, scrollTop: number, viewHeight: number): number | null {
  const row = layout.rows[layout.rowOf[index]];
  if (!row) {
    return null;
  }
  if (row.top - GLYPH_HEADER_PX < scrollTop) {
    return Math.max(0, row.top - GLYPH_HEADER_PX);
  }
  if (row.top + layout.side > scrollTop + viewHeight) {
    return row.top + layout.side + GLYPH_PAD_PX - viewHeight;
  }
  return null;
}

/** The Recent row's cells (criterion 18, UX notes P3): the recent characters the face lists, at
 * most two rows of `columns`; none while the search holds text. */
export function recentCells(recent: readonly string[], listed: ReadonlySet<number>, columns: number, searching: boolean): number[] {
  if (searching) {
    return [];
  }
  return recent
    .map((c) => c.codePointAt(0) ?? -1)
    .filter((cp) => listed.has(cp))
    .slice(0, 2 * Math.max(1, columns));
}
