/** Shows the Trace view on the canvas: the View pill's mode and the peek and cycle keys; a new
 * view is reported to `onViewMode` (the Mask view fetches its mask). */
import { useCallback, useEffect } from "react";

import type { WasmSession } from "@/lib/editorSession";
import { TRACE_VIEWS, isCycleKey, isPeekKey, type TraceViewMode } from "@/lib/traceText";

export function useTraceViewMode(
  active: boolean,
  getSession: () => WasmSession | null,
  onChange: () => void,
  onViewMode: (mode: string) => void,
): { viewMode: TraceViewMode; setViewMode: (mode: TraceViewMode) => void } {
  const setViewMode = useCallback(
    (mode: TraceViewMode) => {
      getSession()?.set_trace_view_mode(mode);
      onChange();
      onViewMode(mode);
    },
    [getSession, onChange, onViewMode],
  );

  // `\` held peeks at the image, Shift+`\` cycles the views (criterion 51), with focus on the
  // canvas or a bar button, never in a text field; release, window blur and leaving the tool
  // end the peek (UX notes section 13).
  useEffect(() => {
    if (!active) {
      return undefined;
    }
    let peeking = false;
    const end = () => {
      if (peeking) {
        peeking = false;
        getSession()?.set_trace_peek(false);
        onChange();
      }
    };
    const typing = (target: EventTarget | null) =>
      target instanceof HTMLElement &&
      target.closest('input, textarea, select, [role="spinbutton"], [contenteditable]:not([contenteditable="false"])') !== null;
    const onKeyDown = (event: KeyboardEvent) => {
      if (typing(event.target)) {
        return;
      }
      if (isCycleKey(event)) {
        event.preventDefault();
        const session = getSession();
        session?.cycle_trace_view_mode();
        onChange();
        onViewMode(session?.trace_view_mode() ?? "");
      } else if (isPeekKey(event)) {
        event.preventDefault();
        peeking = true;
        getSession()?.set_trace_peek(true);
        onChange();
      } else if (event.key === "\\" || event.key === "|") {
        event.preventDefault();
      }
    };
    const onKeyUp = (event: KeyboardEvent) => {
      if (event.key === "\\" || event.key === "|" || event.code === "Backslash") {
        end();
      }
    };
    window.addEventListener("keydown", onKeyDown);
    window.addEventListener("keyup", onKeyUp);
    window.addEventListener("blur", end);
    return () => {
      window.removeEventListener("keydown", onKeyDown);
      window.removeEventListener("keyup", onKeyUp);
      window.removeEventListener("blur", end);
      end();
    };
  }, [active, getSession, onChange, onViewMode]);

  // The view the session draws, read on every render (each sync renders); entering the tool
  // starts on Overlay.
  const sessionMode = active ? (getSession()?.trace_view_mode() as TraceViewMode | undefined) : undefined;
  const viewMode: TraceViewMode = sessionMode && TRACE_VIEWS.some((v) => v.mode === sessionMode) ? sessionMode : "overlay";
  return { viewMode, setViewMode };
}
