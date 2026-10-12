import { useCallback, useEffect, useState } from "react";

import {
  type LayersViewData,
  type PanelReplyData,
  parseLayersView,
  parseReply,
} from "@/lib/layerRows";
import type { WasmSession } from "@/lib/editorSession";

export interface LayersListApi {
  /** The tab and the rows of the window as the session sees them now. */
  view: LayersViewData;
  /** The window of rows the list draws (criterion 48: only these cross from Rust). */
  setWindow: (first: number, count: number) => void;
  /** Runs a panel call, then re-reads everything that may have changed. */
  act: (call: (session: WasmSession) => string) => PanelReplyData;
  /** Re-reads the view only (expand, collapse, hover). */
  refresh: () => void;
}

const EMPTY: LayersViewData = parseLayersView("");

/**
 * The Layers tab's state (`specs/0050-layers-and-objects-panel` criteria 3 to 9 and 48): the
 * session's view of the window of rows in sight, re-read on every change of the session while
 * the tab is shown. A call that writes or selects goes through `act`, which re-reads the whole
 * editor so the canvas, the rail and the other tabs follow in the same frame.
 */
export function useLayersList(
  getSession: () => WasmSession | null,
  syncRevision: number,
  shown: boolean,
  sync: () => void,
): LayersListApi {
  const [range, setRange] = useState({ first: 0, count: 60 });
  const [local, setLocal] = useState(0);
  const [view, setView] = useState<LayersViewData>(EMPTY);

  useEffect(() => {
    if (!shown) {
      return;
    }
    const session = getSession();
    if (session) {
      setView(parseLayersView(session.layers_view(range.first, range.count)));
    }
  }, [getSession, shown, syncRevision, local, range.first, range.count]);

  const setWindow = useCallback((first: number, count: number) => {
    setRange((previous) =>
      previous.first === first && previous.count === count ? previous : { first, count },
    );
  }, []);

  const refresh = useCallback(() => setLocal((n) => n + 1), []);

  const act = useCallback(
    (call: (session: WasmSession) => string) => {
      const session = getSession();
      if (!session) {
        return parseReply("");
      }
      const reply = parseReply(call(session));
      sync();
      refresh();
      return reply;
    },
    [getSession, sync, refresh],
  );

  return { view, setWindow, act, refresh };
}
