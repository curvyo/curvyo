import { useCallback, useEffect, useMemo, useRef, useState } from "react";

import type { WasmSession } from "@/lib/editorSession";
import {
  NO_STATUS,
  TRACE_VIEWS,
  type TraceViewMode,
  failureReason,
  failureText,
  isCycleKey,
  isPeekKey,
  previewIsReduced,
  TRACE_PARAMS_FALLBACK,
  TRACE_PRESETS_FALLBACK,
  type TraceChoice,
  type TraceParam,
  type TraceSlot,
  type TraceStatusText,
  type TracePreset,
  type TraceValues,
  editChoice,
  imagesLabel,
  pickPreset,
  presetLabel,
  readAdvancedOpen,
  readChoice,
  readSummary,
  settingsJson,
  statusText,
  tracedNotice,
  writeAdvancedOpen,
  writeChoice,
} from "@/lib/traceText";
import { TraceCancelled, type TracePixels, type TraceRunner, createTraceRunner } from "@/lib/traceJobs";
import { trace_params, trace_presets } from "@/wasm-bindings/curvyo_editor_wasm.js";

/** Debounce of a value change (criterion 10). */
const DEBOUNCE_MS = 150;
/** The progress pill with its x and the old outline at 50 % after this long (UX notes 21.2). */
const BUSY_MS = 400;
/** The longest side of the reduced copy; a picture this small is traced once. */
const REDUCED_SIDE = 1000;

type ShowNotice = (text: string, ms?: number, source?: "canvas" | "panel", kind?: "status" | "alert") => void;

/** The Trace tool in the frontend (`specs/0025-raster-trace` criteria 1 to 5, 9 to 13, 15, 16,
 * 46, 51 to 54): the bar's state, the worker jobs with their timers, the progress slot, the
 * view and the peek, Create paths and its notice.
 * Every document decision is the session's; the trace runs in the worker. */
export interface TraceApi {
  /** The Trace tool is the active tool. */
  active: boolean;
  /** The Select bar shows the Trace button (only images selected). */
  available: boolean;
  /** What is selected, to the tool. */
  selection: "empty" | "other" | "images";
  /** "Tracing 1 of 3 images", or empty for one image. */
  imagesLabel: string;
  params: readonly TraceParam[];
  /** The presets in menu order (criterion 59). */
  presets: readonly TracePreset[];
  values: TraceValues;
  /** The remembered choice: the preset the values came from and whether they were edited. */
  choice: TraceChoice;
  /** The Preset button's text: the preset's name or "Custom" (criterion 40). */
  presetLabel: string;
  /** The Advanced block is open (criterion 41). */
  advancedOpen: boolean;
  /** The state row's line and muted line, the counts of the Advanced block, and whether Enter
   * may create (criteria 11, 71). */
  status: TraceStatusText;
  /** The counts belong to an older result: a trace runs or was stopped (UX notes 20.4). */
  countsStale: boolean;
  /** What the progress slot of row 1 shows (criteria 10, 53, 54; UX notes 21.2). */
  slot: TraceSlot;
  /** The view on the canvas as the View pill shows it (criterion 51). */
  viewMode: TraceViewMode;
  /** A segment of the View pill. */
  setViewMode: (mode: TraceViewMode) => void;
  /** Trace again after a stop, or Retry after a failure. */
  retrace: () => void;
  /** Create paths waits for full-size traces. */
  creating: boolean;
  /** Working-image pixels per millimetre of the shown image, for the Specks readout. */
  pxPerMm: number;
  /** A value from a field; `dragging` traces the reduced copy only (criterion 10). */
  setValue: <K extends keyof TraceValues>(name: K, value: TraceValues[K], dragging?: boolean) => void;
  /** A pick in the Preset menu (criteria 40, 60, 66): every preset value at once. */
  pick: (preset: TracePreset) => void;
  /** Opens or closes the Advanced block (remembered). */
  setAdvancedOpen: (open: boolean) => void;
  /** The end of a drag: the full-size trace follows. */
  endDrag: () => void;
  /** Stop (criterion 10): ends the job; the preview is out of date. */
  stop: () => void;
  /** Create paths (criteria 12, 13, 46). */
  create: () => void;
  /** Enter and Escape on the canvas in the tool: `true` when the key was taken. */
  keyDown: (event: KeyboardEvent) => boolean;
}

/** The settings schema from the wasm module, or the copy in `traceText` before it loads. */
function loadParams(): readonly TraceParam[] {
  try {
    const parsed = JSON.parse(trace_params()) as TraceParam[];
    return parsed.length > 0 ? parsed : TRACE_PARAMS_FALLBACK;
  } catch {
    return TRACE_PARAMS_FALLBACK;
  }
}

/** The presets from the wasm module, or the copy in `traceText` before it loads. */
function loadPresets(): readonly TracePreset[] {
  try {
    const parsed = JSON.parse(trace_presets()) as TracePreset[];
    return parsed.length > 0 ? parsed : TRACE_PRESETS_FALLBACK;
  } catch {
    return TRACE_PRESETS_FALLBACK;
  }
}

/** The pixels of `id` for the worker, or `null` while the picture is not readable. */
function pixelsOf(session: WasmSession, id: string, reduced: boolean): TracePixels | null {
  const raw = session.trace_source(id, reduced);
  if (!raw) {
    return null;
  }
  const pixels = { width: raw.width, height: raw.height, rgba: raw.rgba, map: Array.from(raw.map) };
  raw.free();
  return pixels;
}

interface ViewState {
  selection: "empty" | "other" | "images";
  images: string[];
  shown: string;
}

const NO_VIEW: ViewState = { selection: "empty", images: [], shown: "" };

function readView(session: WasmSession | null): ViewState {
  const raw = session?.trace_view();
  if (!raw) {
    return NO_VIEW;
  }
  const view = { selection: raw.selection as ViewState["selection"], images: raw.images, shown: raw.shown };
  raw.free();
  return view;
}

export function useTrace(
  getSession: () => WasmSession | null,
  onChange: () => void,
  syncRevision: number,
  tool: string,
  showNotice: ShowNotice,
  returnFocus: () => void,
): TraceApi {
  const active = tool === "trace";
  const params = useMemo(() => loadParams(), []);
  const presets = useMemo(() => loadPresets(), []);
  const [view, setView] = useState<ViewState>(NO_VIEW);
  const [available, setAvailable] = useState(false);
  const [choice, setChoice] = useState<TraceChoice>(() => readChoice(params, presets));
  const values = choice.values;
  const setValues = useCallback((update: (v: TraceValues) => TraceValues) => {
    setChoice((c) => ({ ...c, values: update(c.values) }));
  }, []);
  const [advancedOpen, setAdvancedOpenState] = useState(() => readAdvancedOpen());
  const [status, setStatus] = useState<TraceStatusText>(NO_STATUS);
  const [busy, setBusy] = useState(false);
  /** The running job's whole percent, `null` before the first report or without one. */
  const [percent, setPercent] = useState<number | null>(null);
  const [creating, setCreating] = useState(false);
  const [stopped, setStopped] = useState(false);
  const [failure, setFailure] = useState<string | null>(null);
  /** The picture is not read yet (UX notes 21.2: the pill with "..."). */
  const [loading, setLoading] = useState(false);
  /** The image the job is on (0-based): the first until Create paths moves on (criterion 46). */
  const [progress, setProgress] = useState(0);
  const [pxPerMm, setPxPerMm] = useState(0);
  const runnerRef = useRef<TraceRunner | null>(null);
  const generation = useRef(0);
  const timers = useRef<number[]>([]);
  const debounce = useRef<number | null>(null);
  const dragging = useRef(false);
  /** Each image's Threshold: its automatic value until the maker sets one (criterion 9). */
  const thresholds = useRef(new Map<string, number>());
  /** The values each kept full-size result was traced with. */
  const results = useRef(new Map<string, string>());
  const choiceRef = useRef(choice);
  useEffect(() => {
    choiceRef.current = choice;
  }, [choice]);
  /** The view as last read; the sync effect below keeps it and the state together. */
  const viewRef = useRef(view);

  const runner = useCallback((): TraceRunner => {
    runnerRef.current ??= createTraceRunner();
    return runnerRef.current;
  }, []);
  useEffect(() => () => runnerRef.current?.dispose(), []);

  const clearTimers = useCallback(() => {
    for (const timer of timers.current) {
      window.clearTimeout(timer);
    }
    timers.current = [];
    setBusy(false);
    setPercent(null);
  }, []);

  /** The timers of a job; `known` (over 4 Mpx at full size) shows the progress pill at once
   * (criterion 52). After 400 ms the pill shows and the old outline is drawn at 50 % (criterion 10). */
  const startTimers = useCallback(
    (known = false) => {
      clearTimers();
      const busyNow = () => {
        setBusy(true);
        getSession()?.set_trace_preview_stale(true);
        onChange();
      };
      if (known) {
        busyNow();
        return;
      }
      timers.current.push(window.setTimeout(busyNow, BUSY_MS));
    },
    [clearTimers, getSession, onChange],
  );

  /** Ends whatever runs: the debounce, the jobs and their timers. */
  const cancelAll = useCallback(() => {
    generation.current += 1;
    if (debounce.current !== null) {
      window.clearTimeout(debounce.current);
      debounce.current = null;
    }
    runnerRef.current?.cancel();
    clearTimers();
  }, [clearTimers]);

  const key = (threshold: number, v: TraceValues) => settingsJson(v, threshold);

  /** A failed job (criterion 54): "Failed" with a plain reason in the slot; values and document
   * stay. */
  const fail = useCallback(
    (error: unknown, pixels: number) => {
      clearTimers();
      setCreating(false);
      setFailure(failureText(failureReason(String(error), pixels)));
      setLoading(false);
      setStatus(NO_STATUS);
      getSession()?.set_trace_preview_stale(true);
      onChange();
    },
    [clearTimers, getSession, onChange],
  );

  /** The Threshold of `id`, its automatic value on first use. */
  const thresholdOf = useCallback(
    async (session: WasmSession, id: string): Promise<number | null> => {
      const known = thresholds.current.get(id);
      if (known !== undefined) {
        return known;
      }
      const pixels = pixelsOf(session, id, false);
      if (!pixels) {
        return null;
      }
      const threshold = await runner().threshold(pixels);
      thresholds.current.set(id, threshold);
      return threshold;
    },
    [runner],
  );

  /** The preview of the shown image: the reduced copy first, the full size after (criterion 10). */
  const runPreview = useCallback(
    async (fullAfter: boolean) => {
      const session = getSession();
      const id = viewRef.current.shown;
      if (!session || !id) {
        return;
      }
      const mine = ++generation.current;
      const stale = () => mine !== generation.current;
      setStopped(false);
      setFailure(null);
      setLoading(false);
      startTimers();
      const onProgress = (p: number) => {
        if (!stale()) {
          setPercent(p);
        }
      };
      let pixels = 0;
      try {
        const threshold = await thresholdOf(session, id);
        if (stale()) {
          return;
        }
        if (threshold === null) {
          setStatus(NO_STATUS);
          clearTimers();
          setLoading(true);
          return;
        }
        setValues((v) => (v.threshold === threshold ? v : { ...v, threshold }));
        const v = { ...choiceRef.current.values, threshold };
        const settings = settingsJson(v, threshold);
        const full = pixelsOf(session, id, false);
        if (!full) {
          setStatus(NO_STATUS);
          clearTimers();
          setLoading(true);
          return;
        }
        setPxPerMm(1 / Math.sqrt(Math.abs(full.map[0] * full.map[3] - full.map[1] * full.map[2])));
        pixels = full.width * full.height;
        // Over 4 Mpx the preview is the reduced copy only; Create paths traces at full size
        // (criterion 52).
        const reducedOnly = previewIsReduced(full.width, full.height);
        if (reducedOnly) {
          startTimers(true);
        }
        const small = full.width <= REDUCED_SIDE && full.height <= REDUCED_SIDE;
        if (!small) {
          const reduced = pixelsOf(session, id, true);
          if (reduced) {
            const reducedPixels = reduced.width * reduced.height;
            const out = await runner().trace(reduced, settings, onProgress);
            if (stale()) {
              return;
            }
            session.set_trace_preview(id, out.wire);
            if (reducedOnly) {
              setStatus(statusText(readSummary(out.wire), out.seconds, reducedPixels, true));
            }
          }
          if (!fullAfter || reducedOnly) {
            clearTimers();
            onChange();
            return;
          }
        }
        const out = await runner().trace(full, settings, onProgress);
        if (stale()) {
          return;
        }
        session.set_trace_preview(id, out.wire);
        session.set_trace_result(id, out.wire);
        results.current.set(id, key(threshold, v));
        setStatus(statusText(readSummary(out.wire), out.seconds, full.width * full.height));
        clearTimers();
        onChange();
      } catch (error) {
        if (!(error instanceof TraceCancelled) && !stale()) {
          fail(error, pixels);
        }
      }
    },
    [getSession, startTimers, clearTimers, thresholdOf, runner, setValues, onChange, fail],
  );

  const schedule = useCallback(
    (delay: number) => {
      cancelAll();
      const fullAfter = !dragging.current;
      if (delay <= 0) {
        void runPreview(fullAfter);
        return;
      }
      debounce.current = window.setTimeout(() => {
        debounce.current = null;
        void runPreview(fullAfter);
      }, delay);
    },
    [cancelAll, runPreview],
  );

  // The session's view after every sync; a new image or tool starts a preview at once.
  useEffect(() => {
    const session = getSession();
    setAvailable(Boolean(session?.trace_available()));
    const next = active ? readView(session) : NO_VIEW;
    const previous = viewRef.current;
    if (
      next.selection === previous.selection &&
      next.shown === previous.shown &&
      next.images.join() === previous.images.join()
    ) {
      return;
    }
    viewRef.current = next;
    setView(next);
    cancelAll();
    results.current.clear();
    session?.clear_trace_results();
    setStatus(NO_STATUS);
    setCreating(false);
    setProgress(0);
    setStopped(false);
    setFailure(null);
    setLoading(false);
    if (active && next.shown) {
      const known = thresholds.current.get(next.shown);
      setValues((v) => ({ ...v, threshold: known ?? null }));
      void runPreview(true);
    }
  }, [syncRevision, active, getSession, cancelAll, runPreview, setValues]);

  /** Takes a new choice: remembered, the kept results dropped, a preview after the debounce. */
  const apply = useCallback(
    (next: TraceChoice) => {
      const session = getSession();
      choiceRef.current = next;
      setChoice(next);
      writeChoice(next);
      results.current.clear();
      session?.clear_trace_results();
      session?.set_trace_preview_stale(false);
      setStopped(false);
      setFailure(null);
      setLoading(false);
      schedule(DEBOUNCE_MS);
    },
    [getSession, schedule],
  );

  const setValue = useCallback(
    <K extends keyof TraceValues>(name: K, value: TraceValues[K], isDragging = false) => {
      dragging.current = isDragging;
      if (name === "threshold" && viewRef.current.shown && typeof value === "number") {
        thresholds.current.set(viewRef.current.shown, value);
      }
      apply(editChoice(choiceRef.current, name, value));
    },
    [apply],
  );

  const pick = useCallback(
    (preset: TracePreset) => {
      dragging.current = false;
      apply(pickPreset(choiceRef.current, preset));
    },
    [apply],
  );

  const setAdvancedOpen = useCallback((open: boolean) => {
    setAdvancedOpenState(open);
    writeAdvancedOpen(open);
  }, []);

  const endDrag = useCallback(() => {
    if (dragging.current) {
      dragging.current = false;
      schedule(0);
    }
  }, [schedule]);

  /** Stop or the first Escape (criteria 10, 53): the old outline at 50 %, "Stopped" with Retry in
   * the slot, Enter traces again. */
  const stop = useCallback(() => {
    cancelAll();
    setCreating(false);
    setLoading(false);
    setStopped(true);
    getSession()?.set_trace_preview_stale(true);
    setStatus((s) => ({ ...s, line: "", muted: "", canCreate: false }));
    onChange();
  }, [cancelAll, getSession, onChange]);

  /** Trace again or Retry: the preview at once, with the values as they are. */
  const retrace = useCallback(() => {
    dragging.current = false;
    setStopped(false);
    setFailure(null);
    schedule(0);
  }, [schedule]);

  const setViewMode = useCallback(
    (mode: TraceViewMode) => {
      getSession()?.set_trace_view_mode(mode);
      onChange();
    },
    [getSession, onChange],
  );

  const create = useCallback(() => {
    if (stopped || failure !== null) {
      // Stopped or failed: Enter, Ctrl+Enter and a double-click trace again (UX notes 19).
      retrace();
      return;
    }
    const session = getSession();
    const current = viewRef.current;
    if (!session || creating || current.images.length === 0) {
      return;
    }
    const threshold = choiceRef.current.values.threshold;
    if (threshold === null || (!status.canCreate && results.current.size > 0)) {
      return;
    }
    const v = { ...choiceRef.current.values, threshold };
    const wanted = key(threshold, v);
    setCreating(true);
    setProgress(0);
    const mine = ++generation.current;
    startTimers();
    let pixels = 0;
    void (async () => {
      try {
        let paths = 0;
        let nodes = 0;
        for (const [index, id] of current.images.entries()) {
          if (results.current.get(id) === wanted) {
            continue;
          }
          setProgress(index);
          const full = pixelsOf(session, id, false);
          if (!full) {
            setStatus(NO_STATUS);
            setCreating(false);
            clearTimers();
            setLoading(true);
            return;
          }
          pixels = full.width * full.height;
          // Over 4 Mpx the wait is known: the progress pill at once (criterion 52).
          if (previewIsReduced(full.width, full.height)) {
            startTimers(true);
          }
          setStatus((s) => ({ ...s, line: "", canCreate: false }));
          const out = await runner().trace(full, wanted, (p) => {
            if (mine === generation.current) {
              setPercent(p);
            }
          });
          if (mine !== generation.current) {
            return;
          }
          session.set_trace_result(id, out.wire);
          results.current.set(id, wanted);
        }
        const outcome = session.create_traced_paths();
        if (outcome.kind === "created") {
          paths = outcome.paths;
          nodes = outcome.nodes;
          showNotice(tracedNotice(outcome.images, paths, nodes), 3000, "canvas", "status");
        }
        outcome.free();
        clearTimers();
        setCreating(false);
        onChange();
        returnFocus();
      } catch (error) {
        clearTimers();
        setCreating(false);
        if (!(error instanceof TraceCancelled) && mine === generation.current) {
          fail(error, pixels);
        }
      }
    })();
  }, [getSession, creating, stopped, failure, retrace, status.canCreate, startTimers, clearTimers, runner, showNotice, onChange, returnFocus, fail]);

  // Leaving the tool ends every job.
  useEffect(() => {
    if (!active) {
      cancelAll();
      setCreating(false);
      setStopped(false);
      setFailure(null);
      setLoading(false);
    }
  }, [active, cancelAll]);


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
        getSession()?.cycle_trace_view_mode();
        onChange();
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
  }, [active, getSession, onChange]);

  const keyDown = useCallback(
    (event: KeyboardEvent): boolean => {
      if (!active) {
        return false;
      }
      const plain = !event.shiftKey && !event.altKey && !event.ctrlKey && !event.metaKey;
      if (event.key === "Enter" && plain && !event.repeat) {
        create();
        return true;
      }
      if (event.key === "Escape" && (creating || runnerRef.current?.busy || debounce.current !== null)) {
        stop();
        return true;
      }
      return false;
    },
    [active, create, creating, stop],
  );

  // The view the session draws, read on every render (each sync renders); entering the tool
  // starts on Overlay.
  const sessionMode = active ? (getSession()?.trace_view_mode() as TraceViewMode | undefined) : undefined;
  const viewMode: TraceViewMode = sessionMode && TRACE_VIEWS.some((v) => v.mode === sessionMode) ? sessionMode : "overlay";

  const slot: TraceSlot =
    failure !== null
      ? { kind: "failed", text: failure }
      : stopped
        ? { kind: "stopped" }
        : busy || loading
          ? { kind: "running", percent: loading ? null : percent }
          : { kind: "none" };
  return {
    active,
    available,
    selection: view.selection,
    imagesLabel: imagesLabel(progress, view.images.length),
    params,
    presets,
    values,
    choice,
    presetLabel: presetLabel(choice, presets),
    advancedOpen,
    status,
    countsStale: busy || stopped || creating,
    slot,
    viewMode,
    setViewMode,
    retrace,
    creating,
    pxPerMm,
    setValue,
    pick,
    setAdvancedOpen,
    endDrag,
    stop,
    create,
    keyDown,
  };
}
