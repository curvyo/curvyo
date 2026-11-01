import { useCallback, useEffect, useMemo, useRef, useState } from "react";

import { textRefusalOutlines } from "@/lib/textText";
import { type ActionNotice, useActionNotice } from "@/hooks/useActionNotice";
import type { EditorSession } from "@/hooks/useEditorSession";
import {
  breakApartRefusalText,
  combineOutlinesObjects,
  combineRefusalText,
  combineWarning,
  commandDimmed,
  cutOutlinesObjects,
  cutRefusalText,
  splitRefusalText,
  type PathAvailabilityState,
  type PathCommand,
} from "@/lib/pathText";
import {
  type BooleanAvailabilityState,
  type BooleanOp,
  refusalOutlinesObjects,
  refusalText,
} from "@/lib/booleanText";
import {
  type BooleanCardCommand,
  isStackCommand,
  stackRefusalOutlinesObjects,
  stackRefusalText,
} from "@/lib/stackText";
import {
  type WeldAvailabilityState,
  weldRefusalOutlines,
  weldRefusalText,
} from "@/lib/weldText";

export interface BooleanCommands {
  availability: BooleanAvailabilityState;
  /** What the Weld button shows: its own rule, one or more objects (`0068`). */
  weldAvailability: WeldAvailabilityState;
  /** A kernel call is about to run or running: wait cursor, input ignored. */
  busy: boolean;
  /** Runs `command` on the selection. Does nothing while dimmed or busy. */
  apply: (command: BooleanCardCommand) => void;
}

/** What the Path card shows and does (`specs/0035-combine-and-break-apart/`). */
export interface PathCommands {
  availability: PathAvailabilityState;
  /** A command is about to run or running: wait cursor, input ignored. */
  busy: boolean;
  /** Runs `command` on the selection. Does nothing while dimmed or busy. */
  apply: (command: PathCommand) => void;
}

/** Everything the tool rail's command cards need: both cards and the one notice they share (one
 * status region and one alert region for the whole rail). */
export interface RailCommands {
  booleans: BooleanCommands;
  path: PathCommands;
  notice: ActionNotice | null;
}

/** Resolves after `count` painted frames. */
export function afterFrames(count: number): Promise<void> {
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

/** Presses and keys are ignored while an operation runs (criterion 47a). */
function swallow(event: Event) {
  event.preventDefault();
  event.stopPropagation();
}

const SWALLOWED_EVENTS = [
  "pointerdown",
  "pointerup",
  "click",
  "dblclick",
  "keydown",
  "keyup",
  "wheel",
];

/**
 * The busy state of a running operation: the wait cursor over the whole window and every press,
 * key and wheel turn swallowed in the capture phase until it ends (criterion 47a).
 */
export function useBusyWindow(busy: boolean) {
  useEffect(() => {
    if (!busy) {
      return undefined;
    }
    const root = document.documentElement;
    root.dataset.booleanBusy = "true";
    root.setAttribute("aria-busy", "true");
    // Not passive, so a wheel turn can be cancelled.
    const options = { capture: true, passive: false };
    for (const name of SWALLOWED_EVENTS) {
      window.addEventListener(name, swallow, options);
    }
    return () => {
      delete root.dataset.booleanBusy;
      root.removeAttribute("aria-busy");
      for (const name of SWALLOWED_EVENTS) {
        window.removeEventListener(name, swallow, options);
      }
    };
  }, [busy]);
}

const NO_PATH_AVAILABILITY: PathAvailabilityState = {
  needsTwo: true,
  hasCompound: false,
  canCut: false,
};

/**
 * What the command cards of the tool rail show and do (`specs/0016-boolean-operations/`,
 * `specs/0035-combine-and-break-apart/`): the availability the session reports, the busy state
 * around a call, and the notice of the last command. The decisions (what is enabled, what a
 * command does, why it was refused) are the session's; this hook only displays them.
 */
export function useRailCommands(editor: EditorSession): RailCommands {
  const { getSession, syncRevision, tool, selectionCount, applyBoolean, applyStack, applyWeld } =
    editor;
  const { applyCombine, applySplit, applyBreakApart, applyCut } = editor;
  const [busy, setBusy] = useState(false);
  const busyRef = useRef(false);
  /** Whether the red outline of a refusal is on the canvas. */
  const outlinedRef = useRef(false);
  useBusyWindow(busy);

  const booleanAvailability = useMemo<BooleanAvailabilityState>(() => {
    const raw = getSession()?.boolean_availability();
    if (!raw) {
      return { needsTwo: true, open: 0 };
    }
    const state = { needsTwo: raw.needs_two, open: raw.open };
    raw.free();
    return state;
    // The session is read again after every sync (`syncRevision`).
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [getSession, syncRevision]);
  const weldAvailability = useMemo<WeldAvailabilityState>(() => {
    const raw = getSession()?.weld_availability();
    if (!raw) {
      return { needsOne: true, open: 0 };
    }
    const state = { needsOne: raw.needs_one, open: raw.open };
    raw.free();
    return state;
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [getSession, syncRevision]);
  const pathAvailability = useMemo<PathAvailabilityState>(() => {
    const raw = getSession()?.path_availability();
    if (!raw) {
      return NO_PATH_AVAILABILITY;
    }
    const state = {
      needsTwo: raw.needs_two,
      hasCompound: raw.has_compound,
      canCut: raw.can_cut,
    };
    raw.free();
    return state;
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [getSession, syncRevision]);
  const booleanRef = useRef(booleanAvailability);
  booleanRef.current = booleanAvailability;
  const weldRef = useRef(weldAvailability);
  weldRef.current = weldAvailability;
  const pathRef = useRef(pathAvailability);
  pathRef.current = pathAvailability;

  const clearOutline = useCallback(() => {
    if (outlinedRef.current) {
      outlinedRef.current = false;
      getSession()?.clear_boolean_refusal();
    }
  }, [getSession]);
  const { notice, show } = useActionNotice(tool, selectionCount, clearOutline);

  /** Shows the refusal or warning of a command, if any. A success has no notice: its result on
   * the canvas and the History step are the feedback (customer rule of 2026-10-31). The sentence
   * of a clone repair (`specs/0049-linked-clone` criterion 31) is taken so that it does not wait
   * for the next command. */
  const showRefusal = useCallback(
    (text: string | null, anchor?: ActionNotice["anchor"]) => {
      getSession()?.take_clone_repair_notice();
      if (text !== null) {
        show("refusal", text, undefined, anchor);
      }
    },
    [getSession, show],
  );

  /** Runs `run` after the busy state has reached the screen, then shows its notice. */
  const runBusy = useCallback((run: () => void) => {
    busyRef.current = true;
    setBusy(true);
    void (async () => {
      try {
        // Let the busy state reach the screen before the call takes the thread.
        await afterFrames(2);
        run();
      } finally {
        busyRef.current = false;
        setBusy(false);
      }
    })();
  }, []);

  const apply = useCallback(
    (command: BooleanCardCommand) => {
      if (busyRef.current) {
        return;
      }
      if (command === "weld") {
        if (weldRef.current.needsOne) {
          return;
        }
        runBusy(() => {
          const result = applyWeld();
          outlinedRef.current = weldRefusalOutlines(result);
          showRefusal(weldRefusalText(result));
        });
        return;
      }
      if (booleanRef.current.needsTwo) {
        return;
      }
      if (isStackCommand(command)) {
        runBusy(() => {
          const result = applyStack(command);
          outlinedRef.current = stackRefusalOutlinesObjects(result);
          showRefusal(stackRefusalText(command, result));
        });
        return;
      }
      const op: BooleanOp = command;
      runBusy(() => {
        const result = applyBoolean(op);
        outlinedRef.current = refusalOutlinesObjects(result);
        showRefusal(refusalText(op, result));
      });
    },
    [applyBoolean, applyStack, applyWeld, runBusy, showRefusal],
  );

  const applyPath = useCallback(
    (command: PathCommand) => {
      if (busyRef.current || commandDimmed(command, pathRef.current)) {
        return;
      }
      runBusy(() => {
        if (command === "combine") {
          const result = applyCombine();
          outlinedRef.current = combineOutlinesObjects(result);
          showRefusal(combineRefusalText(result) ?? combineWarning(result), "path");
        } else if (command === "split") {
          const result = applySplit();
          outlinedRef.current =
            result.kind === "groups" ||
            result.kind === "out_of_range" ||
            textRefusalOutlines(result.kind);
          showRefusal(splitRefusalText(result), "path");
        } else if (command === "break_apart") {
          const result = applyBreakApart();
          outlinedRef.current = result.kind === "groups" || textRefusalOutlines(result.kind);
          showRefusal(breakApartRefusalText(result), "path");
        } else {
          const result = applyCut();
          outlinedRef.current = cutOutlinesObjects(result);
          showRefusal(cutRefusalText(result), "path");
        }
      });
    },
    [applyBreakApart, applyCombine, applyCut, applySplit, runBusy, showRefusal],
  );

  return {
    booleans: { availability: booleanAvailability, weldAvailability, busy, apply },
    path: { availability: pathAvailability, busy, apply: applyPath },
    notice,
  };
}
