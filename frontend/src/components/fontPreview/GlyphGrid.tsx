// The glyph table's grid (`specs/0082-glyph-table` criteria 5 to 8, 10, 11, 15, 18, 20 and 21, UX
// notes P2, P3, P6, §3, §4 and §6): the Recent row and one group per Unicode block with its
// header (the one at the top sticks), square cells from the left, virtualised by rows (the rows in
// view and one screen below), drawn by the engine one batch per page; roving focus with the grid
// keys, Enter or Space inserts, a printable key or Backspace goes to the search field.
import { memo, useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";

import { GlyphDrawing } from "@/components/fontPreview/GlyphCell";
import type { GlyphTableState } from "@/hooks/useGlyphTable";
import { cachedCell, cachedDetail, drawCells, namesLoading } from "@/lib/glyphFaces";
import {
  GLYPH_GAP_PX,
  GLYPH_HEADER_PX,
  GLYPH_PAD_PX,
  type GlyphGroupCells,
  cellSide,
  glyphColumns,
  glyphMove,
  glyphPageItems,
  glyphRowsInPage,
  layoutGlyphGrid,
  recentCells,
  scrollToCell,
  stickyHeader,
} from "@/lib/glyphTableLayout";
import {
  NAMES_LOADING_TEXT,
  cellName,
  emptyCellLabel,
  missingFaceText,
  needsNames,
  noCharactersText,
  noMatchText,
  typesIntoSearch,
} from "@/lib/glyphTableText";
import type { FontFace as ListedFace } from "@/lib/textText";

/** The Recent group's name. */
const RECENT = "Recent";

interface GlyphGridProps {
  state: GlyphTableState;
  face: ListedFace | null;
  /** "Great Vibes Regular" and "Great Vibes", for the empty texts. */
  faceName: string;
  family: string;
  query: string;
  size: number;
  recent: readonly string[];
  /** A click (`byKeyboard` false) or Enter, Space on a cell. */
  onInsert: (cp: number, byKeyboard: boolean) => void;
  /** The cell under the pointer, and the current (focused) cell, for the detail bar. */
  onHover: (cp: number | null) => void;
  onCurrent: (cp: number | null) => void;
  /** A printable key or Backspace typed in the grid. */
  onType: (key: string) => void;
  /** Set by the grid: focuses its first cell (Down or Enter in the search field). */
  focusFirstRef: React.RefObject<(() => void) | null>;
}

function useArea(ref: React.RefObject<HTMLDivElement | null>) {
  const [area, setArea] = useState({ width: 300, height: 400 });
  useLayoutEffect(() => {
    const el = ref.current;
    if (!el) {
      return undefined;
    }
    const read = () =>
      setArea((before) =>
        before.width === el.clientWidth && before.height === el.clientHeight
          ? before
          : { width: el.clientWidth, height: el.clientHeight },
      );
    read();
    const observer = new ResizeObserver(read);
    observer.observe(el);
    return () => observer.disconnect();
  }, [ref]);
  return area;
}

export const GlyphGrid = memo(function GlyphGrid(props: GlyphGridProps) {
  const { state, face, faceName, family, size, recent, query, onInsert, onHover, onCurrent, onType, focusFirstRef } = props;
  const scrollRef = useRef<HTMLDivElement | null>(null);
  const area = useArea(scrollRef);
  const [scrollTop, setScrollTop] = useState(0);
  const [revision, setRevision] = useState(0);
  const [active, setActive] = useState(0);
  const pendingFocus = useRef<number | null>(null);
  const searching = query.trim() !== "";
  const table = state.table;
  const singleLine = table?.single_line ?? false;

  const side = cellSide(size);
  const columns = glyphColumns(area.width, size);

  // The Recent row keeps its order while the pointer is over it (UX notes P3).
  const [overRecent, setOverRecent] = useState(false);
  const [shownRecent, setShownRecent] = useState(recent);
  useEffect(() => {
    if (!overRecent) {
      setShownRecent(recent);
    }
  }, [recent, overRecent]);

  const groups = useMemo<GlyphGroupCells[]>(() => {
    const blocks = table?.groups ?? [];
    const listed = new Set(blocks.flatMap((group) => group.cells));
    const recentRow = recentCells(shownRecent, listed, columns, searching);
    return recentRow.length > 0 ? [{ name: RECENT, cells: recentRow }, ...blocks] : blocks;
  }, [table, shownRecent, columns, searching]);
  const layout = useMemo(() => layoutGlyphGrid(groups, columns, side), [groups, columns, side]);
  const hasRecent = groups[0]?.name === RECENT;
  const page = glyphRowsInPage(layout, scrollTop, area.height);

  // A new subject face or search starts at the top (criterion 10).
  const resetKey = `${state.faceKey}\u0000${query}`;
  useLayoutEffect(() => {
    if (scrollRef.current) {
      scrollRef.current.scrollTop = 0;
    }
    setScrollTop(0);
    setActive(0);
  }, [resetKey]);

  // When the Recent row appears or changes height, the scroll position moves with it, so the cell
  // under the pointer or the focus stays where it is (UX notes P3, "Row stability").
  const firstBlock = layout.rows.find((row) => row.kind === "header" && row.group === 1);
  const recentHeight = hasRecent && firstBlock ? firstBlock.top - GLYPH_PAD_PX : 0;
  const recentCount = hasRecent ? groups[0].cells.length : 0;
  const shownCells = table ? table.shown : 0;
  const lastRecent = useRef({ key: resetKey, height: recentHeight, count: recentCount, shown: shownCells });
  useLayoutEffect(() => {
    const before = lastRecent.current;
    lastRecent.current = { key: resetKey, height: recentHeight, count: recentCount, shown: shownCells };
    // Only a change of the table already shown: not the first table of a face or search.
    if (before.key !== resetKey || before.shown === 0) {
      return;
    }
    const el = scrollRef.current;
    if (el && recentHeight !== before.height) {
      el.scrollTop += recentHeight - before.height;
      setScrollTop(el.scrollTop);
    }
    if (recentCount !== before.count) {
      setActive((index) => Math.max(0, index + recentCount - before.count));
    }
  }, [resetKey, recentHeight, recentCount, shownCells]);

  // The cells in the page are drawn in one engine call (criterion 21).
  const pageCells = useMemo(() => {
    const cps: number[] = [];
    for (let i = page.first; i <= page.last; i += 1) {
      const row = layout.rows[i];
      if (row?.kind === "cells") {
        cps.push(...layout.cells.slice(row.first, row.first + row.count));
      }
    }
    return cps;
  }, [layout, page.first, page.last]);
  useEffect(() => {
    if (face && table && drawCells(face, pageCells)) {
      setRevision((r) => r + 1);
    }
  }, [face, table, pageCells]);

  const activeIndex = Math.min(active, Math.max(0, layout.cells.length - 1));
  useEffect(() => {
    onCurrent(layout.cells.length > 0 ? layout.cells[activeIndex] : null);
  }, [activeIndex, layout, onCurrent]);

  const focusCell = (index: number) => {
    setActive(index);
    const el = scrollRef.current;
    if (el) {
      const to = scrollToCell(layout, index, el.scrollTop, el.clientHeight);
      if (to !== null) {
        el.scrollTop = to;
        setScrollTop(el.scrollTop);
      }
    }
    // In the page already: focused now; and again once the scroll has rendered the page, in case
    // the cell was not mounted yet.
    scrollRef.current?.querySelector<HTMLElement>(`[data-cell-index="${index}"]`)?.focus({ preventScroll: true });
    pendingFocus.current = index;
  };
  useLayoutEffect(() => {
    if (pendingFocus.current !== null) {
      const el = scrollRef.current?.querySelector<HTMLElement>(`[data-cell-index="${pendingFocus.current}"]`);
      if (el) {
        if (document.activeElement !== el) {
          el.focus({ preventScroll: true });
        }
        pendingFocus.current = null;
      }
    }
  });
  useEffect(() => {
    focusFirstRef.current = layout.cells.length > 0 ? () => focusCell(0) : null;
  });

  const onKeyDown = (event: React.KeyboardEvent<HTMLDivElement>) => {
    if (layout.cells.length === 0) {
      return;
    }
    const ctrl = event.ctrlKey || event.metaKey;
    const next = glyphMove(event.key, ctrl, activeIndex, layout, area.height);
    if (next !== null) {
      event.preventDefault();
      focusCell(next);
    } else if (event.key === "Enter" || event.key === " ") {
      event.preventDefault();
      onInsert(layout.cells[activeIndex], true);
    } else if (typesIntoSearch(event.key, ctrl, event.altKey, event.metaKey && !event.ctrlKey)) {
      event.preventDefault();
      onType(event.key);
    }
  };

  const sticky = stickyHeader(layout, scrollTop);
  const headerText = (group: number) => `${groups[group]?.name ?? ""} · ${(groups[group]?.cells.length ?? 0).toLocaleString("en-US")}`;

  let empty: string | null = null;
  if (state.status === "missing") {
    empty = missingFaceText(family);
  } else if (state.status === "ready" && table) {
    if (table.total === 0) {
      empty = noCharactersText(faceName);
    } else if (table.shown === 0) {
      empty = needsNames(query) && namesLoading() ? NAMES_LOADING_TEXT : noMatchText(family, query);
    }
  }

  // Until the face is read: skeleton cells for one screen, no header (UX notes §6).
  if (state.status === "loading" || state.status === "none") {
    const count = columns * Math.max(1, Math.ceil(area.height / (side + GLYPH_GAP_PX)));
    return (
      <div ref={scrollRef} aria-busy="true" className="relative min-h-0 flex-1 overflow-hidden bg-[var(--canvas-bg)]">
        {Array.from({ length: state.status === "loading" ? count : 0 }, (_, i) => (
          <div
            key={i}
            className="absolute rounded-[5px] border border-[var(--tile-edge)] bg-[var(--tile-bg)]"
            style={{
              left: GLYPH_PAD_PX + (i % columns) * (side + GLYPH_GAP_PX),
              top: GLYPH_PAD_PX + Math.floor(i / columns) * (side + GLYPH_GAP_PX),
              width: side,
              height: side,
            }}
          >
            <GlyphDrawing cell={undefined} side={side} cap={size} singleLine={false} label="" />
          </div>
        ))}
      </div>
    );
  }

  return (
    <div
      ref={scrollRef}
      role="listbox"
      aria-label={`Characters of ${faceName}`}
      onScroll={(event) => setScrollTop(event.currentTarget.scrollTop)}
      onKeyDown={onKeyDown}
      onPointerLeave={() => {
        onHover(null);
        setOverRecent(false);
      }}
      data-revision={revision}
      className="relative min-h-0 flex-1 overflow-y-auto bg-[var(--canvas-bg)] [scrollbar-gutter:stable]"
    >
      {empty !== null ? (
        <p className="pt-6 text-center text-sm text-[var(--panel-muted-fg)]">{empty}</p>
      ) : (
        <>
          {sticky ? (
            <div
              aria-hidden
              className="pointer-events-none sticky top-0 z-10 -mb-7 flex h-7 items-end bg-[var(--canvas-bg)] pb-1.5 text-xs font-semibold text-[var(--panel-muted-fg)]"
              style={{ transform: `translateY(${sticky.offset}px)`, paddingLeft: GLYPH_PAD_PX }}
            >
              {headerText(sticky.group)}
            </div>
          ) : null}
          <div style={{ position: "relative", height: layout.height }}>
            {glyphPageItems(layout, page).map(({ group, items }) => (
              <div key={groups[group]?.name ?? group} role="group" aria-label={headerText(group)} className="contents">
                {items.map((item) => {
                  if (item.kind === "header") {
                    return (
                      <div
                        key={item.key}
                        aria-hidden
                        className="absolute inset-x-0 flex items-end pb-1.5 text-xs font-semibold text-[var(--panel-muted-fg)]"
                        style={{ top: item.top, height: GLYPH_HEADER_PX, paddingLeft: GLYPH_PAD_PX }}
                      >
                        {headerText(group)}
                      </div>
                    );
                  }
                  const { index, cp } = item;
                  const cell = face ? cachedCell(face, cp) : undefined;
                  const detail = face ? cachedDetail(face, cp) : null;
                  return (
                    <div
                      key={item.key}
                      role="option"
                      aria-selected={index === activeIndex}
                      aria-label={cellName(detail?.name ?? null, cp)}
                      aria-setsize={layout.cells.length}
                      aria-posinset={index + 1}
                      tabIndex={index === activeIndex ? 0 : -1}
                      data-cell-index={index}
                      onFocus={() => setActive(index)}
                      onPointerEnter={() => {
                        onHover(cp);
                        setOverRecent(hasRecent && group === 0);
                      }}
                      onClick={(event) => {
                        setActive(index);
                        onInsert(cp, event.detail === 0);
                      }}
                      className="absolute cursor-default overflow-hidden rounded-[5px] border border-[var(--tile-edge)] bg-[var(--tile-bg)] text-[var(--toolbar-icon)] outline-none hover:bg-[var(--editor-accent-hover)] focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-[var(--editor-accent)]"
                      style={{
                        left: GLYPH_PAD_PX + item.column * (side + GLYPH_GAP_PX),
                        top: item.top,
                        width: side,
                        height: side,
                      }}
                    >
                      <GlyphDrawing
                        cell={cell}
                        side={side}
                        cap={size}
                        singleLine={singleLine}
                        label={emptyCellLabel(detail?.abbreviation ?? null, cp)}
                      />
                    </div>
                  );
                })}
              </div>
            ))}
          </div>
        </>
      )}
    </div>
  );
});
