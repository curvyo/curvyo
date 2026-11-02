// Recognise text in the frontend (`specs/0088-trace-recognition` criteria 3, 4, 6, 7 and 10; UX
// notes §1 to §7): whether its bar replaces the Select bar, the tolerance preference, the run
// stepped one line and face source at a time so the bar can paint its progress and Escape can
// stop it, the faces the run asks for read and handed in, and the notices. Every decision is the
// session's.
import { type RefObject, useCallback, useEffect, useRef, useState } from "react";

import type { WasmSession } from "@/lib/editorSession";
import { favouritesAmong } from "@/hooks/fontLibraryPage";
import { listedFileBytes } from "@/hooks/useText";
import type { FontFace } from "@/lib/textText";
import {
  guessedWarning,
  facesReadout,
  noticeMs,
  nothingText,
  parseOutcome,
  readTolerance,
  searchedReadout,
  writeTolerance,
} from "@/lib/recogniseText";
import { PROGRESS_AFTER_MS, progressHoldLeft } from "@/lib/traceText";

/** The finish button's state (UX notes §3): at rest, running (percent after 400 ms), stopped or
 * failed with Retry. */
export type RecogniseSlot =
  | { kind: "none" }
  | { kind: "running"; percent: number | null }
  | { kind: "stopped" }
  | { kind: "failed"; text: string };

/** The warning under the Select bar's button after a run that guessed letters. */
export interface RecogniseBarNotice {
  id: number;
  text: string;
}

export interface RecogniseApi {
  /** The Recognise bar replaces the Select bar. */
  open: boolean;
  /** Enters the bar; from a key the focus goes to the Tolerance field. */
  enter: (byKeyboard: boolean) => void;
  /** Leaves the bar back to the Select bar, the selection kept. */
  leave: () => void;
  /** The Select bar shows the button: a path with a closed outline is selected (UX notes §1). */
  available: boolean;
  /** What the selection holds for the bar: closed paths, nothing, or only other objects. */
  selection: "paths" | "empty" | "other";
  tolerance: number;
  setTolerance: (value: number) => void;
  /** "5 face sources", or "Searched: system fonts" after nothing within tolerance. */
  readout: string;
  slot: RecogniseSlot;
  /** The bar's notice row ("No text within ..."), empty when none. */
  notice: string;
  /** The guessed-letters warning under the Select bar's button; null for a clean run. */
  barNotice: RecogniseBarNotice | null;
  /** Shows `text` as that notice (Trace's guessed-letters warning, `specs/0091` criterion 5). */
  showNotice: (text: string) => void;
  run: () => void;
  stop: () => void;
  /** The Tolerance field, for the focus on a keyboard entry. */
  fieldRef: RefObject<HTMLDivElement | null>;
}

/** The family names of the listed faces, each once. */
function familiesOf(faces: FontFace[]): string[] {
  return [...new Set(faces.map((face) => face.family))];
}

const yieldToPaint = () => new Promise<void>((resolve) => window.setTimeout(resolve, 0));

export function useRecognise(
  getSession: () => WasmSession | null,
  onChange: () => void,
  options: {
    tool: string;
    selectionCount: number;
    syncRevision: number;
    faces: FontFace[];
    containerRef: RefObject<HTMLElement | null>;
  },
): RecogniseApi {
  const { tool, selectionCount, syncRevision, faces, containerRef } = options;
  const [open, setOpen] = useState(false);
  const [tolerance, setToleranceState] = useState(() => readTolerance(typeof localStorage === "undefined" ? null : localStorage));
  const [slot, setSlot] = useState<RecogniseSlot>({ kind: "none" });
  const [notice, setNotice] = useState("");
  const [searched, setSearched] = useState<string | null>(null);
  const [barNotice, setBarNotice] = useState<RecogniseBarNotice | null>(null);
  const runId = useRef(0);
  const counter = useRef(0);
  const fieldRef = useRef<HTMLDivElement | null>(null);
  const focusField = useRef(false);

  const session = getSession();
  // Read on every sync: the availability, the selection's state for the bar and the sources.
  void syncRevision;
  const available = tool === "select" && (session?.recognise_available() ?? false);
  const selection: RecogniseApi["selection"] = available ? "paths" : selectionCount === 0 ? "empty" : "other";
  const favouritesJson = JSON.stringify([...favouritesAmong(familiesOf(faces))]);
  const sources = open && session ? session.recognise_sources(favouritesJson) : 0;
  const readout = searchedReadout(searched) ?? facesReadout(sources);

  const clearResult = useCallback(() => {
    setNotice("");
    setSearched(null);
    setSlot((before) => (before.kind === "stopped" || before.kind === "failed" ? { kind: "none" } : before));
  }, []);

  const stop = useCallback(() => {
    runId.current += 1;
    getSession()?.recognise_stop();
    setSlot({ kind: "stopped" });
  }, [getSession]);

  const leave = useCallback(() => {
    runId.current += 1;
    getSession()?.recognise_stop();
    setOpen(false);
    setSlot({ kind: "none" });
    setNotice("");
    setSearched(null);
  }, [getSession]);

  const enter = useCallback(
    (byKeyboard: boolean) => {
      setBarNotice(null);
      setNotice("");
      setSearched(null);
      setSlot({ kind: "none" });
      focusField.current = byKeyboard;
      setOpen(true);
    },
    [],
  );

  // A keyboard entry puts the focus on the Tolerance field once the bar is in the tree.
  useEffect(() => {
    if (open && focusField.current) {
      focusField.current = false;
      fieldRef.current?.querySelector<HTMLElement>("[tabindex], input, button")?.focus();
    }
  }, [open]);

  // Another tool than Select closes the bar, and a selection change clears the result of the
  // last run (UX notes §3, §4): state adjusted while rendering, when what it follows changed.
  const [seen, setSeen] = useState({ tool, selectionCount });
  if (seen.tool !== tool || seen.selectionCount !== selectionCount) {
    setSeen({ tool, selectionCount });
    if (seen.tool !== tool && tool !== "select" && open) {
      setOpen(false);
      setSlot({ kind: "none" });
    }
    if (seen.selectionCount !== selectionCount) {
      setNotice("");
      setSearched(null);
      if (slot.kind === "stopped" || slot.kind === "failed") {
        setSlot({ kind: "none" });
      }
    }
  }
  // A run in flight under another tool ends (its steps would be ignored).
  useEffect(() => {
    if (!open) {
      runId.current += 1;
    }
  }, [open]);

  const setTolerance = useCallback(
    (value: number) => {
      setToleranceState(value);
      writeTolerance(typeof localStorage === "undefined" ? null : localStorage, value);
      clearResult();
    },
    [clearResult],
  );

  const run = useCallback(() => {
    const started = getSession();
    if (!started) {
      return;
    }
    setNotice("");
    setSearched(null);
    if (!started.recognise_start(tolerance, favouritesJson)) {
      setSlot({ kind: "none" });
      return;
    }
    runId.current += 1;
    const mine = runId.current;
    let shownAt: number | null = null;
    setSlot({ kind: "running", percent: null });
    const progressTimer = window.setTimeout(() => {
      shownAt = performance.now();
      setSlot({ kind: "running", percent: 0 });
    }, PROGRESS_AFTER_MS);
    const settle = async () => {
      window.clearTimeout(progressTimer);
      const left = progressHoldLeft(shownAt, performance.now());
      if (left > 0) {
        await new Promise((resolve) => window.setTimeout(resolve, left));
      }
    };
    void (async () => {
      try {
        for (;;) {
          await yieldToPaint();
          if (runId.current !== mine) {
            window.clearTimeout(progressTimer);
            return;
          }
          const current = getSession();
          if (!current) {
            return;
          }
          const outcome = parseOutcome(current.recognise_step());
          if (outcome.kind === "loading") {
            // The faces a face source needs, read and handed in; one that cannot be read is
            // passed over at the next step.
            const ids = current.wanted_font_files();
            await Promise.allSettled(
              ids.map((id) => listedFileBytes(id).then((bytes) => getSession()?.add_font_file(id, bytes))),
            );
            continue;
          }
          if (outcome.kind === "running") {
            if (shownAt !== null) {
              setSlot({ kind: "running", percent: outcome.total > 0 ? Math.floor((100 * outcome.done) / outcome.total) : 0 });
            }
            continue;
          }
          await settle();
          if (runId.current !== mine) {
            return;
          }
          if (outcome.kind === "applied") {
            const text = guessedWarning(outcome);
            counter.current += 1;
            setOpen(false);
            setSlot({ kind: "none" });
            setBarNotice(text === null ? null : { id: counter.current, text });
            onChange();
            containerRef.current?.focus({ preventScroll: true });
            return;
          }
          if (outcome.kind === "nothing") {
            setSlot({ kind: "none" });
            setNotice(nothingText(outcome, tolerance));
            setSearched(outcome.no_lines ? null : outcome.searched);
            onChange();
            return;
          }
          // Ignored: the selection or the document changed under the run.
          setSlot({ kind: "none" });
          onChange();
          return;
        }
      } catch (error) {
        window.clearTimeout(progressTimer);
        getSession()?.recognise_stop();
        setSlot({ kind: "failed", text: `Recognise text failed: ${String(error)}. Your drawing is unchanged.` });
      }
    })();
  }, [getSession, onChange, tolerance, favouritesJson, containerRef]);

  // Enter runs and Escape stops, then leaves (UX notes §7); a typed field keeps its own keys.
  useEffect(() => {
    if (!open) {
      return undefined;
    }
    const keyDown = (event: KeyboardEvent) => {
      const target = event.target as HTMLElement | null;
      if (target && (target.tagName === "INPUT" || target.tagName === "TEXTAREA" || target.isContentEditable)) {
        return;
      }
      if (event.key === "Escape") {
        event.preventDefault();
        event.stopPropagation();
        if (slot.kind === "running") {
          stop();
        } else {
          leave();
          containerRef.current?.focus({ preventScroll: true });
        }
      } else if (event.key === "Enter" && target === containerRef.current && slot.kind !== "running") {
        event.preventDefault();
        event.stopPropagation();
        run();
      }
    };
    window.addEventListener("keydown", keyDown, true);
    return () => window.removeEventListener("keydown", keyDown, true);
  }, [open, slot.kind, stop, leave, run, containerRef]);

  // The notice under the button ends after its time or at the next press or key.
  useEffect(() => {
    if (!barNotice) {
      return undefined;
    }
    const end = () => setBarNotice(null);
    const timer = window.setTimeout(end, noticeMs(barNotice.text));
    const listen = window.setTimeout(() => {
      window.addEventListener("pointerdown", end, true);
      window.addEventListener("keydown", end, true);
    }, 0);
    return () => {
      window.clearTimeout(timer);
      window.clearTimeout(listen);
      window.removeEventListener("pointerdown", end, true);
      window.removeEventListener("keydown", end, true);
    };
  }, [barNotice]);

  const showNotice = useCallback((text: string) => {
    counter.current += 1;
    setBarNotice({ id: counter.current, text });
  }, []);

  return {
    open,
    enter,
    leave,
    available,
    selection,
    tolerance,
    setTolerance,
    readout,
    slot,
    notice,
    barNotice,
    showNotice,
    run,
    stop,
    fieldRef,
  };
}
