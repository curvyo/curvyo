import { useCallback, useMemo, useState } from "react";

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
  /** Runs a panel call, then re-reads everything that may have changed. */
  act: (call: (session: WasmSession) => string) => PanelReplyData;
  /** Re-reads the view only (expand, collapse). */
  refresh: () => void;
}

const EMPTY: LayersViewData = parseLayersView("");

/**
 * The Layers tab's state (`specs/0050-layers-and-objects-panel` criteria 3 to 9 and 48): the
 * session's view of the rows `first` to `first + count`, read while rendering whenever the
 * session, the window or a panel call changed it (a second read of the same state returns the
 * same text, so nothing is stored here). A call that writes or selects goes through `act`, which
 * re-reads the whole editor so the canvas, the rail and the other tabs follow in the same frame.
 */
export function useLayersList(
  getSession: () => WasmSession | null,
  syncRevision: number,
  sync: () => void,
  first: number,
  count: number,
): LayersListApi {
  const [local, setLocal] = useState(0);

  const view = useMemo(() => {
    const session = getSession();
    return session ? parseLayersView(session.layers_view(first, count)) : EMPTY;
    // `syncRevision` and `local` name the changes of the session the read depends on.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [getSession, syncRevision, local, first, count]);

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

  return { view, act, refresh };
}
