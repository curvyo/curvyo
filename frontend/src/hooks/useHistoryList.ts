import { useCallback, useEffect, useRef, useState } from "react";

import {
  EMPTY_SHAPE,
  type HistoryItemData,
  type ListShape,
  parseItems,
  parseShape,
} from "@/lib/historyItems";
import { type HistoryScope, PAGE_SIZE, pagesFor, rowsAdded } from "@/lib/historyRows";
import type { WasmSession } from "@/lib/editorSession";

export interface HistoryList {
  /** The number of items: the step rows of the global history or of the selected object, with
   * the header rows of its branches and the "more" row (`specs/0042-history-branches`). */
  total: number;
  /** The heights and lanes of the items. */
  shape: ListShape;
  /** The shown id of the newest step in effect, for the "Now" row; empty without a step. */
  nowId: string;
  /** The item at `index` (0 is the top), or `undefined` while its page has not arrived. */
  itemAt: (index: number) => HistoryItemData | undefined;
  /** A press or Enter on a header row or the "more" row: folds or unfolds it. */
  toggle: (index: number) => void;
  /** Asks for the pages that hold rows `first` to `last`. */
  ensure: (first: number, last: number) => void;
  /** The `scope:object` the pages hold rows for: it differs from what is wanted until the rows of
   * a new scope or object have been read. */
  loadedFor: string;
  /** When the rows were read: "today" for the times in the rows. */
  readAt: Date;
  /** How many rows arrived at the top since the list was last read, for keeping a scrolled list
   * in place; read once per change. */
  takeAdded: () => number;
}

/**
 * The global history as the History tab reads it (`specs/0020-undo-redo` criteria 43 and 44): the
 * session counts the steps and returns rows in pages of fifty, newest first. A page is read once;
 * every change of the log (a step, an undo, a merge) drops the pages and reads the visible ones
 * again. Nothing is read while the tab is not shown. The Object scope reads the selected object's
 * rows (`specs/0041-object-history`) through the same pages; another scope or another object
 * drops them (`source`).
 */
export function useHistoryList(
  getSession: () => WasmSession | null,
  syncRevision: number,
  shown: boolean,
  scope: HistoryScope,
  source: string,
): HistoryList {
  const pages = useRef(new Map<number, HistoryItemData[]>());
  const wanted = useRef({ first: 0, last: PAGE_SIZE - 1 });
  const stamp = useRef<number | null>(null);
  const reading = useRef("");
  const added = useRef(0);
  const totalSeen = useRef(0);
  const [folds, setFolds] = useState(0);
  const [state, setState] = useState(() => ({
    total: 0,
    shape: EMPTY_SHAPE,
    nowId: "",
    version: 0,
    readAt: new Date(),
    loadedFor: "",
  }));
  const object = scope === "object";

  const load = useCallback(
    (first: number, last: number) => {
      const session = getSession();
      if (!session) {
        return;
      }
      let changed = false;
      for (const page of pagesFor(first, last)) {
        if (!pages.current.has(page)) {
          pages.current.set(page, parseItems(session.history_items(object, page * PAGE_SIZE, PAGE_SIZE)));
          changed = true;
        }
      }
      if (changed) {
        setState((previous) => ({ ...previous, version: previous.version + 1 }));
      }
    },
    [getSession, object],
  );

  // The log changed: read the count and the visible pages again.
  useEffect(() => {
    const session = getSession();
    if (!shown || !session) {
      return;
    }
    const now = session.history_stamp();
    const reads = `${scope}:${source}`;
    const foldKey = `${reads}:${folds}`;
    if (stamp.current === now && reading.current === foldKey) {
      return;
    }
    const sourceChanged = !reading.current.startsWith(`${reads}:`);
    const folded = !sourceChanged && stamp.current === now;
    stamp.current = now;
    reading.current = foldKey;
    pages.current.clear();
    const shape = parseShape(session.history_layout(object));
    const total = shape.total;
    // Counted outside the updater: React calls an updater twice in development. Another scope
    // or object starts a new list: nothing "arrived" at its top.
    if (sourceChanged) {
      added.current = 0;
      totalSeen.current = total;
    }
    // A fold changes the count without a new step: nothing arrived.
    added.current += folded ? 0 : rowsAdded(totalSeen.current, total);
    totalSeen.current = total;
    const nowId = session.history_now_id();
    setState((previous) => ({
      total,
      shape,
      nowId,
      version: previous.version + 1,
      readAt: new Date(),
      loadedFor: reads,
    }));
    load(wanted.current.first, wanted.current.last);
  }, [shown, syncRevision, getSession, load, scope, source, object, folds]);

  const itemAt = useCallback((index: number) => {
    return pages.current.get(Math.floor(index / PAGE_SIZE))?.[index % PAGE_SIZE];
    // `state.version` is what makes a caller read again after pages arrived.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [state.version]);

  const ensure = useCallback(
    (first: number, last: number) => {
      wanted.current = { first, last };
      load(first, last);
    },
    [load],
  );

  const toggle = useCallback(
    (index: number) => {
      if (getSession()?.history_toggle(object, index)) {
        setFolds((n) => n + 1);
      }
    },
    [getSession, object],
  );

  const takeAdded = useCallback(() => {
    const count = added.current;
    added.current = 0;
    return count;
  }, []);

  return {
    total: state.total,
    shape: state.shape,
    nowId: state.nowId,
    readAt: state.readAt,
    loadedFor: state.loadedFor,
    itemAt,
    toggle,
    ensure,
    takeAdded,
  };
}
