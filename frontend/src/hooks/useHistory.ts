import { useCallback, useEffect, useRef, useState } from "react";

import { useBusyWindow } from "@/hooks/useRailCommands";
import {
  FINISH_PEN_NOTICE_MS,
  type HistoryDirection,
  historyKeyDirection,
  isMac,
  menuNames,
  noticeDurationMs,
  noticeText,
  WORKING_NOTICE_DELAY_MS,
  workingNotice,
} from "@/lib/historyText";
import { objectWipedNotice, wipedNotice } from "@/lib/historyRows";
import type { WasmSession } from "@/lib/editorSession";

/** A line in the canvas notice slot (`docs/design-system.md`, "Canvas notice slot"). */
export interface CanvasNotice {
  /** Changes with every notice, so the clock restarts. */
  id: number;
  text: string;
  ms: number;
  /** Where it shows: the canvas notice slot, or 4 px under the History tab's button. */
  source: "canvas" | "panel";
  /** A status is polite and takes the slot for 3 s; an alert is a refusal and stays 8 s. */
  kind: "status" | "alert";
}

/** The step names an undo and a redo would act on, `null` when a stack is empty. */
export interface HistoryNames {
  undo: string | null;
  redo: string | null;
}

export interface HistoryApi {
  /** The notice on screen, or `null`. */
  notice: CanvasNotice | null;
  /** A large undo or redo is running: wait cursor, input ignored, `aria-busy`. */
  busy: boolean;
  names: HistoryNames;
  /** Shows `text` in the canvas notice slot, replacing the one on screen. */
  showNotice: (
    text: string,
    ms?: number,
    source?: "canvas" | "panel",
    kind?: "status" | "alert",
  ) => void;
  /** Re-reads the names of the next undo and redo. Call after any change of the session. */
  refresh: () => void;
  /** Runs an undo or redo from a button; `source` says where the notice shows. */
  press: (direction: HistoryDirection, source?: "canvas" | "panel") => void;
  /** Wipes the history (the confirm was pressed): the notice says how many steps went. Returns the
   * steps removed, -1 when the session ignored it, -2 for a shared document and -3 when the
   * log could not be rewritten. */
  wipe: () => number;
  /** Hides the selected object's earlier history: the notice says how many rows went. Returns
   * the rows hidden, -1 when the session ignored it and -3 when the log could not take the step. */
  wipeObject: () => number;
  /** Handles the outcome string `key_down` returned when it belongs to history; `false`
   * for every other outcome. */
  handleKeyOutcome: (outcome: string) => boolean;
}

/** Whether this is macOS, where Cmd+Y is no redo key. */
export const MAC = isMac(navigator.platform);

/** Resolves after `count` painted frames. */
function afterFrames(count: number): Promise<void> {
  return new Promise((resolve) => {
    const step = (left: number) => {
      if (left <= 0) {
        resolve();
        return;
      }
      requestAnimationFrame(() => step(left - 1));
    };
    step(count);
  });
}

/** Whether keys typed here belong to the element itself (its own text undo). */
function isTextEntry(target: EventTarget | null): boolean {
  return (
    target instanceof HTMLElement &&
    target.closest(
      'textarea, select, [contenteditable]:not([contenteditable="false"]), input:not([type="checkbox"]):not([type="radio"]):not([type="button"]):not([type="range"])',
    ) !== null
  );
}

/**
 * Undo and redo in the frontend (`specs/0020-undo-redo`): the canvas notice of every press, the
 * long-operation state of a large step and the keys outside the canvas (the tool rail and the
 * Properties panel). There is no Edit menu (`specs/0051-sidebar-and-no-menu/` criterion 35): the keys
 * and the History tab's buttons are the routes. Every decision (the gate, the stacks, what is
 * skipped) is the session's; this hook forwards keys and shows what comes back.
 *
 * `onChange` runs after a press changed the document, to re-read the session's state.
 * `onApplied` runs when an undo or redo is started, to end feedback that belongs to the step it
 * takes back (the nudge readout).
 */
export function useHistory(
  getSession: () => WasmSession | null,
  onChange: () => void,
  containerRef: React.RefObject<HTMLDivElement | null>,
  onApplied?: () => void,
): HistoryApi {
  const [notice, setNotice] = useState<CanvasNotice | null>(null);
  const [busy, setBusy] = useState(false);
  const [names, setNames] = useState<HistoryNames>({ undo: null, redo: null });
  const counter = useRef(0);
  const onChangeRef = useRef(onChange);
  const onAppliedRef = useRef(onApplied);
  useEffect(() => {
    onChangeRef.current = onChange;
    onAppliedRef.current = onApplied;
  }, [onChange, onApplied]);
  useBusyWindow(busy);

  const showNotice = useCallback(
    (
      text: string,
      ms?: number,
      source: "canvas" | "panel" = "canvas",
      kind: "status" | "alert" = "status",
    ) => {
      counter.current += 1;
      setNotice({ id: counter.current, text, ms: ms ?? noticeDurationMs(text), source, kind });
    },
    [],
  );

  // A notice ends after its time, or when the next one replaces it.
  useEffect(() => {
    if (!notice) {
      return undefined;
    }
    const timer = window.setTimeout(() => setNotice(null), notice.ms);
    return () => window.clearTimeout(timer);
  }, [notice]);

  const refresh = useCallback(() => {
    const session = getSession();
    if (!session) {
      return;
    }
    const next = menuNames(
      session.history_target_name("undo"),
      session.history_target_name("redo"),
    );
    setNames((previous) =>
      previous.undo === next.undo && previous.redo === next.redo ? previous : next,
    );
  }, [getSession]);

  /** Runs a press the session announced as large: the state first, a frame to paint it, then the
   * call. */
  const runBusy = useCallback(
    async (direction: HistoryDirection, source: "canvas" | "panel" = "canvas") => {
      setBusy(true);
      const timer = window.setTimeout(
        () => showNotice(workingNotice(direction), 60_000, source),
        WORKING_NOTICE_DELAY_MS,
      );
      await afterFrames(2);
      try {
        const session = getSession();
        const text = noticeText(session?.history_press(direction) ?? "");
        if (text !== null) {
          showNotice(text, undefined, source);
        }
      } finally {
        window.clearTimeout(timer);
        setBusy(false);
        onChangeRef.current();
      }
    },
    [getSession, showNotice],
  );

  const press = useCallback(
    (direction: HistoryDirection, source: "canvas" | "panel" = "canvas") => {
      const session = getSession();
      if (!session || busy) {
        return;
      }
      onAppliedRef.current?.();
      if (session.history_needs_busy_state(direction) && session.history_target_name(direction)) {
        void runBusy(direction, source);
        return;
      }
      const text = noticeText(session.history_press(direction));
      if (text !== null) {
        showNotice(text, undefined, source);
      }
      onChangeRef.current();
    },
    [busy, getSession, runBusy, showNotice],
  );

  const handleKeyOutcome = useCallback(
    (outcome: string): boolean => {
      switch (outcome) {
        case "undo":
        case "redo":
        case "object-undo":
        case "object-redo": {
          onAppliedRef.current?.();
          const text = noticeText(getSession()?.take_history_notice() ?? "");
          if (text !== null) {
            showNotice(text);
          }
          return true;
        }
        case "undo-busy":
        case "redo-busy":
        case "object-undo-busy":
        case "object-redo-busy": {
          const direction = outcome.replace(/-busy$/, "") as HistoryDirection;
          onAppliedRef.current?.();
          void runBusy(direction);
          return true;
        }
        case "hint-object-tool":
          showNotice(getSession()?.object_tool_notice() ?? "", FINISH_PEN_NOTICE_MS);
          return true;
        case "hint-finish-pen":
          showNotice(getSession()?.finish_pen_first_notice() ?? "", FINISH_PEN_NOTICE_MS);
          return true;
        default:
          return false;
      }
    },
    [getSession, runBusy, showNotice],
  );

  // Undo and redo with the focus outside the canvas, in the tool rail or the Properties panel
  // (criterion 1). The canvas has its own key handler; a text field keeps its own undo.
  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      const direction = historyKeyDirection(event, MAC);
      if (direction === null || event.defaultPrevented) {
        return;
      }
      const target = event.target;
      if (isTextEntry(target)) {
        return;
      }
      const container = containerRef.current;
      if (container && target instanceof Node && container.contains(target)) {
        return;
      }
      if (busy) {
        event.preventDefault();
        return;
      }
      const session = getSession();
      if (!session) {
        return;
      }
      const outcome = session.key_down(
        event.key,
        event.shiftKey,
        event.ctrlKey || event.metaKey,
        event.altKey,
        event.repeat,
        false,
        MAC,
        event.timeStamp,
      );
      if (outcome === "ignored") {
        return;
      }
      event.preventDefault();
      handleKeyOutcome(outcome);
      onChangeRef.current();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [busy, containerRef, getSession, handleKeyOutcome]);

  const wipe = useCallback(() => {
    const removed = getSession()?.history_wipe() ?? -1;
    if (removed >= 0) {
      showNotice(wipedNotice(removed));
    } else if (removed === -3) {
      showNotice("Could not wipe the history. Nothing was changed.");
    }
    onChangeRef.current();
    return removed;
  }, [getSession, showNotice]);

  const wipeObject = useCallback(() => {
    const hidden = getSession()?.object_history_wipe() ?? -1;
    if (hidden >= 0) {
      showNotice(objectWipedNotice(hidden));
    } else if (hidden === -3) {
      showNotice("Could not hide the history of this object. Nothing was changed.");
    }
    onChangeRef.current();
    return hidden;
  }, [getSession, showNotice]);

  return {
    notice,
    busy,
    names,
    showNotice,
    refresh,
    press,
    wipe,
    wipeObject,
    handleKeyOutcome,
  };
}
