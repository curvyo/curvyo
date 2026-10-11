import { useCallback, useEffect, useRef, useState } from "react";

import { useBusyWindow } from "@/hooks/useRailCommands";
import {
  DUPLICATE_PRESS_MS,
  FINISH_PEN_NOTICE_MS,
  type HistoryDirection,
  historyKeyDirection,
  isMac,
  menuNames,
  menuPressPlan,
  noticeDurationMs,
  noticeText,
  WORKING_NOTICE_DELAY_MS,
  workingNotice,
} from "@/lib/historyText";
import { wipedNotice } from "@/lib/historyRows";
import { invoke, isTauri, listen } from "@/platform/host";
import type { WasmSession } from "@/lib/editorSession";

/** A line in the canvas notice slot (`docs/design-system.md`, "Canvas notice slot"). */
export interface CanvasNotice {
  /** Changes with every notice, so the clock restarts. */
  id: number;
  text: string;
  ms: number;
  /** Where it shows: the canvas notice slot, or 4 px under the History tab's button. */
  source: "canvas" | "panel";
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
  showNotice: (text: string, ms?: number, source?: "canvas" | "panel") => void;
  /** Re-reads the names of the next undo and redo (and the native menu). Call after any change
   * of the session. */
  refresh: () => void;
  /** Runs an undo or redo from a button or the menu; `source` says where the notice shows. */
  press: (direction: HistoryDirection, source?: "canvas" | "panel") => void;
  /** Wipes the history (the confirm was pressed): the notice says how many steps went. Returns the
   * steps removed, -1 when the session ignored it, -2 for a shared document and -3 when the
   * log could not be rewritten. */
  wipe: () => number;
  /** Whether this key event is the second arrival of a press the native menu already handled. */
  isDuplicateKey: (event: KeyboardEvent | React.KeyboardEvent) => boolean;
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
 * long-operation state of a large step, the keys outside the canvas (the tool rail and the
 * Properties panel), and the native Edit menu. Every decision (the gate, the stacks, what is
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
  const lastPress = useRef<{ direction: HistoryDirection; at: number } | null>(null);
  const onChangeRef = useRef(onChange);
  const onAppliedRef = useRef(onApplied);
  useEffect(() => {
    onChangeRef.current = onChange;
    onAppliedRef.current = onApplied;
  }, [onChange, onApplied]);
  useBusyWindow(busy);

  const showNotice = useCallback(
    (text: string, ms?: number, source: "canvas" | "panel" = "canvas") => {
      counter.current += 1;
      setNotice({ id: counter.current, text, ms: ms ?? noticeDurationMs(text), source });
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

  const sentToMenu = useRef<string>("");
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
    if (isTauri) {
      const key = JSON.stringify(next);
      if (key !== sentToMenu.current) {
        sentToMenu.current = key;
        void invoke("set_history_menu", next).catch((error: unknown) => {
          // The native menu falls behind; the keys and buttons still work.
          console.error("set_history_menu failed", error);
        });
      }
    }
  }, [getSession]);

  const markPressed = useCallback((direction: HistoryDirection) => {
    lastPress.current = { direction, at: performance.now() };
  }, []);

  const pressedJustNow = useCallback((direction: HistoryDirection) => {
    const last = lastPress.current;
    return (
      last !== null &&
      last.direction === direction &&
      performance.now() - last.at < DUPLICATE_PRESS_MS
    );
  }, []);

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
        case "redo": {
          markPressed(outcome);
          onAppliedRef.current?.();
          const text = noticeText(getSession()?.take_history_notice() ?? "");
          if (text !== null) {
            showNotice(text);
          }
          return true;
        }
        case "undo-busy":
        case "redo-busy": {
          const direction = outcome === "undo-busy" ? "undo" : "redo";
          markPressed(direction);
          onAppliedRef.current?.();
          void runBusy(direction);
          return true;
        }
        case "hint-finish-pen":
          showNotice(getSession()?.finish_pen_first_notice() ?? "", FINISH_PEN_NOTICE_MS);
          return true;
        default:
          return false;
      }
    },
    [getSession, markPressed, runBusy, showNotice],
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
      if (busy || pressedJustNow(direction)) {
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
  }, [busy, containerRef, getSession, handleKeyOutcome, pressedJustNow]);

  // The native Edit menu. A click arrives as `history-menu`; so does the macOS accelerator, which
  // can also reach the page as a key event, and the first one wins (criterion 8). The listener is
  // registered once and reads `press` through a ref, so a busy undo never leaves a gap.
  const menuPressRef = useRef<(direction: HistoryDirection) => void>(() => {});
  useEffect(() => {
    menuPressRef.current = (direction) => {
      const textEntry = isTextEntry(document.activeElement);
      const plan = menuPressPlan({ duplicate: pressedJustNow(direction), textEntry });
      if (plan === "text-undo") {
        document.execCommand(direction);
      } else if (plan === "document") {
        markPressed(direction);
        press(direction);
      }
    };
  }, [markPressed, press, pressedJustNow]);
  useEffect(() => {
    if (!isTauri) {
      return undefined;
    }
    const unlisten = listen<string>("history-menu", (event) => {
      menuPressRef.current(event.payload === "redo" ? "redo" : "undo");
    });
    void unlisten.catch((error: unknown) => console.error("history-menu listen failed", error));
    return () => {
      void unlisten.then((stop) => stop()).catch(() => undefined);
    };
  }, []);

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

  const isDuplicateKey = useCallback(
    (event: KeyboardEvent | React.KeyboardEvent) => {
      const direction = historyKeyDirection(event, MAC);
      return direction !== null && pressedJustNow(direction);
    },
    [pressedJustNow],
  );

  return {
    notice,
    busy,
    names,
    showNotice,
    refresh,
    press,
    wipe,
    isDuplicateKey,
    handleKeyOutcome,
  };
}
