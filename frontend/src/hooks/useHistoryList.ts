import { useCallback, useEffect, useRef, useState } from "react";

import {
  type HistoryRowData,
  PAGE_SIZE,
  pagesFor,
  parseRows,
} from "@/lib/historyRows";
import type { WasmSession } from "@/lib/editorSession";

export interface HistoryList {
  /** The number of steps in the global history. */
  total: number;
  /** The shown id of the newest step in effect, for the "Now" row; empty without a step. */
  nowId: string;
  /** The row at `index` (0 is the newest), or `undefined` while its page has not arrived. */
  rowAt: (index: number) => HistoryRowData | undefined;
  /** Asks for the pages that hold rows `first` to `last`. */
  ensure: (first: number, last: number) => void;
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
 * again. Nothing is read while the tab is not shown.
 */
export function useHistoryList(
  getSession: () => WasmSession | null,
  syncRevision: number,
  shown: boolean,
): HistoryList {
  const pages = useRef(new Map<number, HistoryRowData[]>());
  const wanted = useRef({ first: 0, last: PAGE_SIZE - 1 });
  const stamp = useRef<number | null>(null);
  const added = useRef(0);
  const [state, setState] = useState(() => ({ total: 0, nowId: "", version: 0, readAt: new Date() }));

  const load = useCallback(
    (first: number, last: number) => {
      const session = getSession();
      if (!session) {
        return;
      }
      let changed = false;
      for (const page of pagesFor(first, last)) {
        if (!pages.current.has(page)) {
          pages.current.set(
            page,
            parseRows(session.history_rows(page * PAGE_SIZE, PAGE_SIZE)),
          );
          changed = true;
        }
      }
      if (changed) {
        setState((previous) => ({ ...previous, version: previous.version + 1 }));
      }
    },
    [getSession],
  );

  // The log changed: read the count and the visible pages again.
  useEffect(() => {
    const session = getSession();
    if (!shown || !session) {
      return;
    }
    const now = session.history_stamp();
    if (stamp.current === now) {
      return;
    }
    stamp.current = now;
    pages.current.clear();
    const total = session.history_len();
    setState((previous) => {
      added.current += Math.max(0, total - previous.total);
      return {
        total,
        nowId: session.history_now_id(),
        version: previous.version + 1,
        readAt: new Date(),
      };
    });
    load(wanted.current.first, wanted.current.last);
  }, [shown, syncRevision, getSession, load]);

  const rowAt = useCallback((index: number) => {
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

  const takeAdded = useCallback(() => {
    const count = added.current;
    added.current = 0;
    return count;
  }, []);

  return {
    total: state.total,
    nowId: state.nowId,
    readAt: state.readAt,
    rowAt,
    ensure,
    takeAdded,
  };
}
