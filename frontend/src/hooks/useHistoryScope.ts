import { useEffect, useMemo, useRef, useState } from "react";

import {
  choiceAfterSelection,
  effectiveScope,
  type HistoryScope,
  type ObjectHistoryInfo,
  parseObjectInfo,
} from "@/lib/historyRows";
import type { WasmSession } from "@/lib/editorSession";

/** The scroll position of each scope, kept for the session (`0041` criterion 1). */
export interface ScrollMemory {
  document: number;
  object: number;
}

export interface HistoryScopeState {
  /** The scope that shows. */
  scope: HistoryScope;
  /** Whether the scope group is in the tree: exactly one object is selected. */
  hasGroup: boolean;
  /** A press on the group. */
  choose: (scope: HistoryScope) => void;
  /** The selected object's subject and buttons, `null` unless exactly one object is selected. */
  info: ObjectHistoryInfo | null;
  scrollMemory: React.RefObject<ScrollMemory>;
}

/**
 * The scope of the History tab and the Object scope's subject (`specs/0041-object-history`
 * criterion 1). It lives in the panel, not in the tab, so a press on Document survives a change to
 * another tab and back; it is session state, never saved. The subject is read from the session
 * only while the History tab is the active one and one object is selected.
 */
export function useHistoryScope(
  getSession: () => WasmSession | null,
  syncRevision: number,
  selectionCount: number,
  historyShown: boolean,
): HistoryScopeState {
  const [choice, setChoice] = useState<HistoryScope>("object");
  const scrollMemory = useRef<ScrollMemory>({ document: 0, object: 0 });

  // Leaving the single-object state resets a press on Document for the next time.
  useEffect(() => {
    setChoice((previous) => choiceAfterSelection(selectionCount, previous));
  }, [selectionCount]);

  const info = useMemo(() => {
    const session = getSession();
    if (!historyShown || !session || selectionCount !== 1) {
      return null;
    }
    return parseObjectInfo(session.object_history_info());
    // The subject follows the log and the selection.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [getSession, historyShown, selectionCount, syncRevision]);

  return {
    scope: effectiveScope(selectionCount, choice),
    hasGroup: selectionCount === 1,
    choose: setChoice,
    info,
    scrollMemory,
  };
}
