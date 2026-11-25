// The library browser's Items grid (`specs/0152-library-browser` criteria 4 to 7, 11, 23 and 29,
// UX notes sections 0.1, 0.5, 6 and 9): equal tiles in columns from the left, no cards;
// virtualised by rows (in view and one screen ahead in the scroll direction); one Tab stop with
// roving focus, arrows, Home, End, pages, Enter to load, Space to select, a printable character or
// Backspace to the search field.
import { forwardRef, useImperativeHandle, useLayoutEffect, useMemo, useRef, useState } from "react";

import { BrowserTile } from "@/components/libraryBrowser/BrowserTile";
import {
  GRID_GAP_PX,
  GRID_PAD_PX,
  type ScrollDirection,
  columnsFor,
  rowsInView,
  tileHeight,
  tileWidth,
} from "@/lib/libraryBrowserLayout";
import { type TileWords, tileWords } from "@/lib/libraryBrowserQuery";

export interface GridHandle {
  /** Focuses the first tile (Down or Enter in the search field, UX U18). */
  focusFirst: () => void;
  /** Focuses item `index` if listed, else the current tile, else the first; scrolls it in. */
  focusItem: (index: number | null) => void;
  /** The first visible item, the scroll position as an identity (UX U39). */
  anchor: () => number | null;
  /** Scrolls item `index` to the top of the grid. */
  reveal: (index: number) => void;
}

interface GridProps {
  order: Uint32Array;
  empty: string;
  size: number;
  /** A change scrolls to the top (criterion 16). */
  filterKey: string;
  /** The list's version: the words are asked again. */
  version: number;
  current: number | null;
  onSelect: (index: number) => void;
  onLoad: (index: number, byKeyboard: boolean) => void;
  onShowPages: (index: number) => void;
  onHover: (index: number | null) => void;
  onDragStart: (event: React.PointerEvent<HTMLDivElement>, words: TileWords) => void;
  onType: (text: string) => void;
  /** The empty state's buttons: Clear all, Show sets (H14); null when not offered. */
  onClearAll: (() => void) | null;
  onShowSets: (() => void) | null;
}

export const BrowserGrid = forwardRef<GridHandle, GridProps>(function BrowserGrid(props, ref) {
  const { order, size, current } = props;
  const scrollRef = useRef<HTMLDivElement>(null);
  const [area, setArea] = useState({ width: 400, height: 400 });
  const [scrollTop, setScrollTop] = useState(0);
  const [direction, setDirection] = useState<ScrollDirection>(1);
  const [focus, setFocus] = useState(0);
  /** The position to focus once a render holds it. */
  const pendingFocus = useRef<number | null>(null);

  useLayoutEffect(() => {
    const el = scrollRef.current;
    if (!el) {
      return undefined;
    }
    const read = () => setArea({ width: el.clientWidth, height: el.clientHeight });
    read();
    const observer = new ResizeObserver(read);
    observer.observe(el);
    return () => observer.disconnect();
  }, []);

  // A filter, sort or search change starts at the top (criterion 16).
  const firstKey = useRef(props.filterKey);
  useLayoutEffect(() => {
    if (firstKey.current === props.filterKey) {
      return;
    }
    firstKey.current = props.filterKey;
    if (scrollRef.current) {
      scrollRef.current.scrollTop = 0;
    }
    setScrollTop(0);
    setFocus(0);
  }, [props.filterKey]);

  const n = order.length;
  const cols = columnsFor(area.width, tileWidth(size));
  const tileRow = tileHeight(size) + GRID_GAP_PX;
  const tileRows = Math.ceil(n / cols);
  const emptyPx = props.empty ? 120 : 0;
  const height = GRID_PAD_PX + tileRows * tileRow + emptyPx + GRID_PAD_PX;
  const inView = rowsInView(scrollTop - GRID_PAD_PX, area.height, tileRow, tileRows, direction);
  const focusAt = Math.min(focus, Math.max(0, n - 1));

  const shownTiles = useMemo(() => {
    const from = inView.first * cols;
    const to = Math.min(n, (inView.last + 1) * cols);
    const positions: number[] = [];
    for (let p = from; p < to; p += 1) {
      positions.push(p);
    }
    if (focusAt < n && (focusAt < from || focusAt >= to)) {
      positions.push(focusAt);
    }
    const words = tileWords(positions.map((p) => order[p] ?? 0));
    return positions.map((p, i) => ({ position: p, words: words[i] })).filter((t) => t.words !== undefined);
    // `version` re-asks the words after a new list.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [order, inView.first, inView.last, cols, n, focusAt, props.version]);

  const topOf = (position: number) => GRID_PAD_PX + Math.floor(position / cols) * tileRow;

  const moveTo = (position: number, select = true) => {
    if (n === 0) {
      return;
    }
    const target = Math.max(0, Math.min(n - 1, position));
    const el = scrollRef.current;
    if (el) {
      const top = topOf(target);
      if (top < el.scrollTop) {
        el.scrollTop = top - GRID_PAD_PX;
      } else if (top + tileRow > el.scrollTop + el.clientHeight) {
        el.scrollTop = top + tileRow - el.clientHeight;
      }
    }
    pendingFocus.current = target;
    setFocus(target);
    if (select) {
      props.onSelect(order[target] ?? 0);
    }
  };

  useLayoutEffect(() => {
    // Only the render that holds the asked position: an earlier one (a store update rendered
    // ahead of the focus change, as after Back) would focus the old position.
    if (pendingFocus.current === null || pendingFocus.current !== focusAt) {
      return;
    }
    pendingFocus.current = null;
    // The option itself: its wrapper cannot take the focus.
    const el = scrollRef.current?.querySelector<HTMLElement>(`[data-position="${focusAt}"] > [role="option"]`);
    el?.focus({ preventScroll: true });
  });

  useImperativeHandle(ref, () => ({
    focusFirst: () => moveTo(0),
    focusItem: (index) => {
      const at = index === null ? -1 : order.indexOf(index);
      const fallback = current === null ? -1 : order.indexOf(current);
      moveTo(at >= 0 ? at : fallback >= 0 ? fallback : 0, false);
    },
    anchor: () => {
      const el = scrollRef.current;
      if (!el || n === 0) {
        return null;
      }
      const row = Math.max(0, Math.floor((el.scrollTop - GRID_PAD_PX) / tileRow));
      return order[Math.min(n - 1, row * cols)] ?? null;
    },
    reveal: (index) => {
      const at = order.indexOf(index);
      const el = scrollRef.current;
      if (el && at >= 0) {
        el.scrollTop = topOf(at) - GRID_PAD_PX;
        setScrollTop(el.scrollTop);
      }
    },
  }));

  const onKeyDown = (event: React.KeyboardEvent<HTMLDivElement>) => {
    const perPage = Math.max(1, Math.floor(area.height / tileRow)) * cols;
    const moves: Record<string, number> = {
      ArrowRight: focusAt + 1,
      ArrowLeft: focusAt - 1,
      ArrowDown: focusAt + cols,
      ArrowUp: focusAt - cols,
      PageDown: focusAt + perPage,
      PageUp: focusAt - perPage,
      Home: 0,
      End: n - 1,
    };
    // Alt+Left is Back (the pane's), not a move.
    const next = event.altKey ? undefined : moves[event.key];
    if (next !== undefined && n > 0) {
      event.preventDefault();
      moveTo(next);
      return;
    }
    if (event.key === "Enter" && n > 0) {
      event.preventDefault();
      props.onLoad(order[focusAt] ?? 0, true);
      return;
    }
    if (event.key === " " && n > 0) {
      // Space selects; it never loads (H4).
      event.preventDefault();
      props.onSelect(order[focusAt] ?? 0);
      return;
    }
    if (event.key === "Backspace" && !event.altKey && !event.ctrlKey && !event.metaKey) {
      // Backspace no longer goes back (H6): it edits the search like a printable key.
      event.preventDefault();
      props.onType("\b");
      return;
    }
    if (event.key.length === 1 && !event.ctrlKey && !event.metaKey && !event.altKey) {
      event.preventDefault();
      props.onType(event.key);
    }
  };

  const tileW = tileWidth(size);
  const tileH = tileHeight(size);
  return (
    <div
      ref={scrollRef}
      role="listbox"
      aria-label="Library items"
      className="relative min-h-0 flex-1 overflow-y-auto bg-[var(--canvas-bg)] [overscroll-behavior:contain]"
      onScroll={(event) => {
        const top = event.currentTarget.scrollTop;
        if (top !== scrollTop) {
          setDirection(top > scrollTop ? 1 : -1);
        }
        setScrollTop(top);
      }}
      onKeyDown={onKeyDown}
    >
      <div style={{ height, position: "relative" }}>
        {shownTiles.map(({ position, words }) =>
          words ? (
            <div key={words.index} data-position={position} onFocus={() => setFocus(position)}>
              <BrowserTile
                words={words}
                size={size}
                current={current === words.index}
                focused={position === focusAt}
                style={{
                  top: topOf(position),
                  left: GRID_PAD_PX + (position % cols) * (tileW + GRID_GAP_PX),
                  width: tileW,
                  height: tileH,
                }}
                onPress={(index) => {
                  setFocus(position);
                  props.onSelect(index);
                }}
                onLoad={(index) => props.onLoad(index, false)}
                onShowPages={props.onShowPages}
                onHover={props.onHover}
                onDragStart={props.onDragStart}
              />
            </div>
          ) : null,
        )}
        {props.empty ? (
          <div className="absolute right-0 left-0 flex flex-col items-center gap-3 px-4 text-center" style={{ top: GRID_PAD_PX + 12 }}>
            <p className="text-sm text-[var(--panel-muted-fg)]">{props.empty}</p>
            {props.onShowSets ? (
              <button type="button" onClick={props.onShowSets} className={EMPTY_BUTTON}>
                Show sets
              </button>
            ) : null}
            {props.onClearAll ? (
              <button type="button" onClick={props.onClearAll} className={EMPTY_BUTTON}>
                Clear all
              </button>
            ) : null}
          </div>
        ) : null}
      </div>
    </div>
  );
});

const EMPTY_BUTTON =
  "h-7 rounded-[5px] px-3 text-sm text-[var(--toolbar-icon)] ring-1 ring-[color-mix(in_srgb,var(--toolbar-icon)_60%,transparent)] hover:bg-[var(--editor-accent-hover)]";
