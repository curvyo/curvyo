// The scrolling of the Fonts tab's virtualised tree (`specs/0074-text-objects` UX notes §4,
// `specs/0075-font-library` UX notes §3 and §4): its height, the rows in view and their faces
// loaded after 100 ms, a new strip cell or search starting at the top, the first visible row kept in
// place when rows above it change, focusing a row and revealing a new family after Add font.
import { useCallback, useEffect, useLayoutEffect, useRef, useState } from "react";

import { loadFace } from "@/lib/fontFaceLoader";
import type { Row } from "@/lib/fontRows";

/** A row loads its face once it has been in view this long, so fast scrolling loads nothing. */
const LOAD_AFTER_MS = 100;

export function useFontTreeScroll(
  rows: Row[],
  tops: { at: number[]; total: number },
  /** The chosen group and the search: a change scrolls to the top. */
  filter: string,
  setFocusKey: (key: string) => void,
  /** The tree's inner width, for the tag lines' height. */
  onWidth: (width: number) => void,
) {
  const listRef = useRef<HTMLDivElement | null>(null);
  const [scrollTop, setScrollTop] = useState(0);
  const [height, setHeight] = useState(400);
  const [, setLoadedRevision] = useState(0);
  const anchorRef = useRef<{ key: string; offset: number } | null>(null);
  const revealRef = useRef<{ key: string; focus: boolean } | null>(null);

  // The list element comes and goes (the empty states), so it is observed by a
  // callback ref; an old observer would report 0 px and draw only the first rows.
  const [listEl, setListEl] = useState<HTMLDivElement | null>(null);
  const setList = useCallback((el: HTMLDivElement | null) => {
    listRef.current = el;
    setListEl(el);
  }, []);
  useEffect(() => {
    if (!listEl) {
      return undefined;
    }
    const observer = new ResizeObserver(() => {
      if (listEl.clientHeight > 0) {
        setHeight(listEl.clientHeight);
        onWidth(listEl.clientWidth);
      }
    });
    observer.observe(listEl);
    return () => observer.disconnect();
  }, [listEl, onWidth]);

  const filterRef = useRef(filter);
  useLayoutEffect(() => {
    const list = listRef.current;
    if (!list) {
      return;
    }
    if (filter !== filterRef.current) {
      filterRef.current = filter;
      list.scrollTop = 0;
      setScrollTop(0);
      return;
    }
    const anchor = anchorRef.current;
    const at = anchor ? rows.findIndex((row) => row.key === anchor.key) : -1;
    if (anchor && at >= 0 && Math.abs(tops.at[at] - anchor.offset - list.scrollTop) > 0.5) {
      list.scrollTop = tops.at[at] - anchor.offset;
      setScrollTop(list.scrollTop);
    }
  }, [rows, tops, filter]);

  const onScroll = useCallback(
    (event: React.UIEvent<HTMLDivElement>) => {
      const top = event.currentTarget.scrollTop;
      setScrollTop(top);
      const at = tops.at.findIndex((t) => t >= top);
      anchorRef.current = at >= 0 ? { key: rows[at].key, offset: tops.at[at] - top } : null;
    },
    [rows, tops],
  );

  const first = Math.max(0, tops.at.findIndex((top, i) => top + rows[i].height > scrollTop) - 2);
  const visible: number[] = [];
  for (let i = first; i < rows.length && tops.at[i] < scrollTop + height + 64; i += 1) {
    visible.push(i);
  }

  // Faces of rows that stay in view load after 100 ms.
  const visibleKey = visible.map((i) => rows[i].key).join("|");
  useEffect(() => {
    const timer = window.setTimeout(() => {
      for (const i of visible) {
        const row = rows[i];
        const face = row.kind === "family" ? row.family.faces[0] : row.kind === "face" ? row.face : null;
        if (face) {
          loadFace(face, () => setLoadedRevision((r) => r + 1));
        }
      }
    }, LOAD_AFTER_MS);
    return () => window.clearTimeout(timer);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [visibleKey]);

  const focusRow = useCallback(
    (key: string, place: "nearest" | "third" = "nearest") => {
      setFocusKey(key);
      const index = rows.findIndex((row) => row.key === key);
      const list = listRef.current;
      if (list && index >= 0) {
        const top = tops.at[index];
        if (place === "third") {
          list.scrollTop = Math.max(0, top - list.clientHeight / 3);
        } else if (top < list.scrollTop) {
          list.scrollTop = top;
        } else if (top + rows[index].height > list.scrollTop + list.clientHeight) {
          list.scrollTop = top + rows[index].height - list.clientHeight;
        }
      }
      window.requestAnimationFrame(() =>
        listRef.current?.querySelector<HTMLElement>(`[data-row-key="${CSS.escape(key)}"]`)?.focus(),
      );
    },
    [rows, tops, setFocusKey],
  );

  // A new family after Add font: scrolled to the top third, focused after a key.
  useEffect(() => {
    const pending = revealRef.current;
    if (!pending || !rows.some((row) => row.key === pending.key)) {
      return;
    }
    revealRef.current = null;
    const index = rows.findIndex((row) => row.key === pending.key);
    const list = listRef.current;
    if (list) {
      list.scrollTop = Math.max(0, tops.at[index] - list.clientHeight / 3);
      setScrollTop(list.scrollTop);
    }
    if (pending.focus) {
      focusRow(pending.key, "third");
    }
  }, [rows, tops, focusRow]);

  const reveal = useCallback((key: string, focus: boolean) => {
    revealRef.current = { key, focus };
  }, []);

  return { listRef, setList, height, scrollTop, visible, onScroll, focusRow, reveal };
}
