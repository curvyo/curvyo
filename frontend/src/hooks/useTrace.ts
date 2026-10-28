import { useCallback, useEffect, useMemo, useRef, useState } from "react";

import type { WasmSession } from "@/lib/editorSession";
import {
  PREVIEW_OUT_OF_DATE,
  TRACE_PARAMS_FALLBACK,
  TRACING,
  LOADING_IMAGE,
  type TraceParam,
  type TraceValues,
  creatingLabel,
  imagesLabel,
  readSummary,
  readValues,
  statusText,
  tracedNotice,
  writeValues,
} from "@/lib/traceText";
import { TraceCancelled, type TracePixels, type TraceRunner, createTraceRunner } from "@/lib/traceJobs";
import { trace_params } from "@/wasm-bindings/curvyo_editor_wasm.js";

/** Debounce of a value change (criterion 10). */
const DEBOUNCE_MS = 150;
/** "Tracing..." and the progress line after this long (criterion 10). */
const BUSY_MS = 400;
/** The Stop button after this long (criterion 10). */
const STOP_MS = 1500;
/** The longest side of the reduced copy; a picture this small is traced once. */
const REDUCED_SIDE = 1000;

type ShowNotice = (text: string, ms?: number, source?: "canvas" | "panel", kind?: "status" | "alert") => void;

/** The Trace tool in the frontend (`specs/0025-raster-trace` criteria 1 to 5, 9 to 13, 15, 16,
 * 46): the bar's state, the worker jobs with their timers, Stop, Create paths and its notice.
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
  values: TraceValues;
  /** The status line, its muted second line, and whether Create paths may run. */
  status: { line: string; muted: string; canCreate: boolean };
  /** A job runs past 400 ms: "Tracing..." and the progress line. */
  busy: boolean;
  /** A job runs past 1.5 s: the Stop button. */
  stopShown: boolean;
  /** Create paths waits for full-size traces. */
  creating: boolean;
  /** Working-image pixels per millimetre of the shown image, for the Specks readout. */
  pxPerMm: number;
  /** A value from a field; `dragging` traces the reduced copy only (criterion 10). */
  setValue: (name: TraceParam["name"], value: number, dragging: boolean) => void;
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
  const [view, setView] = useState<ViewState>(NO_VIEW);
  const [available, setAvailable] = useState(false);
  const [values, setValues] = useState<TraceValues>(() => readValues(params));
  const [status, setStatus] = useState({ line: "", muted: "", canCreate: false });
  const [busy, setBusy] = useState(false);
  const [stopShown, setStopShown] = useState(false);
  const [creating, setCreating] = useState(false);
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
  const valuesRef = useRef(values);
  useEffect(() => {
    valuesRef.current = values;
  }, [values]);
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
    setStopShown(false);
  }, []);

  const startTimers = useCallback(() => {
    clearTimers();
    timers.current.push(window.setTimeout(() => setBusy(true), BUSY_MS));
    timers.current.push(window.setTimeout(() => setStopShown(true), STOP_MS));
  }, [clearTimers]);

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

  const key = (threshold: number, v: TraceValues) => `${threshold}|${v.smooth}|${v.specks}`;

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
      startTimers();
      try {
        const threshold = await thresholdOf(session, id);
        if (stale()) {
          return;
        }
        if (threshold === null) {
          setStatus({ line: LOADING_IMAGE, muted: "", canCreate: false });
          clearTimers();
          return;
        }
        setValues((v) => (v.threshold === threshold ? v : { ...v, threshold }));
        const v = { ...valuesRef.current, threshold };
        const full = pixelsOf(session, id, false);
        if (!full) {
          setStatus({ line: LOADING_IMAGE, muted: "", canCreate: false });
          clearTimers();
          return;
        }
        setPxPerMm(1 / Math.sqrt(Math.abs(full.map[0] * full.map[3] - full.map[1] * full.map[2])));
        const small = full.width <= REDUCED_SIDE && full.height <= REDUCED_SIDE;
        if (!small) {
          const reduced = pixelsOf(session, id, true);
          if (reduced) {
            const out = await runner().trace(reduced, threshold, v.smooth, v.specks);
            if (stale()) {
              return;
            }
            session.set_trace_preview(id, out.wire);
          }
          if (!fullAfter) {
            clearTimers();
            return;
          }
        }
        const out = await runner().trace(full, threshold, v.smooth, v.specks);
        if (stale()) {
          return;
        }
        session.set_trace_preview(id, out.wire);
        session.set_trace_result(id, out.wire);
        results.current.set(id, key(threshold, v));
        setStatus(statusText(readSummary(out.wire), out.seconds, full.width * full.height));
        clearTimers();
      } catch (error) {
        if (!(error instanceof TraceCancelled) && !stale()) {
          setStatus({ line: `Trace failed: ${String(error)}. Your drawing is unchanged.`, muted: "", canCreate: false });
          clearTimers();
        }
      }
    },
    [getSession, startTimers, clearTimers, thresholdOf, runner],
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
    setStatus({ line: "", muted: "", canCreate: false });
    setCreating(false);
    setProgress(0);
    if (active && next.shown) {
      const known = thresholds.current.get(next.shown);
      setValues((v) => ({ ...v, threshold: known ?? null }));
      void runPreview(true);
    }
  }, [syncRevision, active, getSession, cancelAll, runPreview]);

  const setValue = useCallback(
    (name: TraceParam["name"], value: number, isDragging: boolean) => {
      dragging.current = isDragging;
      const session = getSession();
      const next = { ...valuesRef.current, [name]: value };
      if (name === "threshold" && viewRef.current.shown) {
        thresholds.current.set(viewRef.current.shown, value);
      }
      valuesRef.current = next;
      setValues(next);
      writeValues(next);
      results.current.clear();
      session?.clear_trace_results();
      session?.set_trace_preview_stale(false);
      schedule(DEBOUNCE_MS);
    },
    [getSession, schedule],
  );

  const endDrag = useCallback(() => {
    if (dragging.current) {
      dragging.current = false;
      schedule(0);
    }
  }, [schedule]);

  const stop = useCallback(() => {
    cancelAll();
    setCreating(false);
    getSession()?.set_trace_preview_stale(true);
    setStatus((s) => ({ ...s, muted: PREVIEW_OUT_OF_DATE }));
    onChange();
  }, [cancelAll, getSession, onChange]);

  const create = useCallback(() => {
    const session = getSession();
    const current = viewRef.current;
    if (!session || creating || current.images.length === 0) {
      return;
    }
    const threshold = valuesRef.current.threshold;
    if (threshold === null || (!status.canCreate && results.current.size > 0)) {
      return;
    }
    const v = { ...valuesRef.current, threshold };
    const wanted = key(threshold, v);
    setCreating(true);
    setProgress(0);
    const mine = ++generation.current;
    startTimers();
    void (async () => {
      try {
        let paths = 0;
        let nodes = 0;
        for (const [index, id] of current.images.entries()) {
          if (results.current.get(id) === wanted) {
            continue;
          }
          setProgress(index);
          setStatus({ line: creatingLabel(index, current.images.length), muted: "", canCreate: false });
          const full = pixelsOf(session, id, false);
          if (!full) {
            setStatus({ line: LOADING_IMAGE, muted: "", canCreate: false });
            setCreating(false);
            clearTimers();
            return;
          }
          const out = await runner().trace(full, threshold, v.smooth, v.specks);
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
        if (!(error instanceof TraceCancelled)) {
          setStatus({ line: `Trace failed: ${String(error)}. Your drawing is unchanged.`, muted: "", canCreate: false });
        }
      }
    })();
  }, [getSession, creating, status.canCreate, startTimers, clearTimers, runner, showNotice, onChange, returnFocus]);

  // Leaving the tool ends every job.
  useEffect(() => {
    if (!active) {
      cancelAll();
      setCreating(false);
    }
  }, [active, cancelAll]);

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

  const busyLine = busy && !status.line.startsWith("Tracing") ? { ...status, line: TRACING } : status;
  return {
    active,
    available,
    selection: view.selection,
    imagesLabel: imagesLabel(progress, view.images.length),
    params,
    values,
    status: busyLine,
    busy,
    stopShown,
    creating,
    pxPerMm,
    setValue,
    endDrag,
    stop,
    create,
    keyDown,
  };
}
