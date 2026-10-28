import { useCallback, useEffect, useMemo, useRef, useState } from "react";

import type { PanelOpenApi } from "@/hooks/usePanelOpen";

/** The name of the About Trace tab in the Properties panel's strip. */
export const TRACE_INFO_TAB = "trace-info";

/** The Trace tool's About Trace tab (`specs/0025-raster-trace` criterion 70, UX notes 21.1). */
export interface TraceInfoApi {
  /** The tab is in the strip: the Trace tool is active, whatever is selected. */
  available: boolean;
  /** The tab is the active one, in place of the session's active tab (also with the body hidden). */
  active: boolean;
  /** The tab is active and the body is shown: the "i" button is pressed. */
  shown: boolean;
  /** The tab was chosen in the strip: it becomes active and the body shows it. */
  choose: () => void;
  /** The "i" button: shows the tab, or returns to the tab and body state before it. */
  toggle: () => void;
  /** Another tab was chosen, by a press or its shortcut: the tab is left as that action leaves
   * the panel. */
  dismiss: () => void;
}

/**
 * Whether the About Trace tab is active: never by itself, not remembered. Choosing it stores
 * whether the body was shown; the "i" button pressed again and leaving the tool return to the
 * session's tab with the body as it was. The session's active tab is never changed by it, so
 * leaving shows the tab the "Active tab" rule holds.
 */
export function useTraceInfo(panelOpen: PanelOpenApi, toolActive: boolean): TraceInfoApi {
  const [active, setActive] = useState(false);
  const activeRef = useRef(false);
  /** Whether the body was shown when the tab was chosen. */
  const bodyWasOpen = useRef(true);
  const { open: bodyOpen, isOpen, setOpen: setBodyOpen } = panelOpen;

  const dismiss = useCallback(() => {
    activeRef.current = false;
    setActive(false);
  }, []);

  const close = useCallback(() => {
    if (!activeRef.current) {
      return;
    }
    dismiss();
    if (!bodyWasOpen.current) {
      setBodyOpen(false, false);
    }
  }, [dismiss, setBodyOpen]);

  const choose = useCallback(() => {
    if (!activeRef.current) {
      bodyWasOpen.current = isOpen();
      activeRef.current = true;
      setActive(true);
    }
    setBodyOpen(true, false);
  }, [isOpen, setBodyOpen]);

  const toggle = useCallback(() => {
    if (activeRef.current && isOpen()) {
      close();
    } else {
      choose();
    }
  }, [choose, close, isOpen]);

  useEffect(() => {
    if (!toolActive) {
      close();
    }
  }, [toolActive, close]);

  const shown = active && bodyOpen;
  return useMemo(
    () => ({ available: toolActive, active, shown, choose, toggle, dismiss }),
    [toolActive, active, shown, choose, toggle, dismiss],
  );
}
