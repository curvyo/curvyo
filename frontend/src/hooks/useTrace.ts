import { useCallback, useEffect, useMemo, useRef, useState } from "react";

import type { WasmSession } from "@/lib/editorSession";
import {
  NO_STATUS,
  PICK_COLOURS,
  zeroPathsLine,
  failureReason,
  failureText,
  previewIsReduced,
  type TraceChoice,
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
  withSwatchNote,
  writeAdvancedOpen,
  writeChoice,
} from "@/lib/traceText";
import { TraceCancelled, type TraceRunner, createTraceRunner } from "@/lib/traceJobs";
import { NO_VIEW, loadParams, loadPresets, pixelsOf, readView, type ViewState } from "@/lib/traceSource";
import type { ShowNotice, TraceApi } from "./traceApi";
import { useTraceMask } from "./useTraceMask";
import { useTraceMethod } from "./useTraceMethod";
import { useTraceProgress } from "./useTraceProgress";
import { useTraceViewMode } from "./useTraceViewMode";

export type { TraceApi } from "./traceApi";

/** Debounce of a value change (criterion 10). */
const DEBOUNCE_MS = 150;
/** The longest side of the reduced copy; a picture this small is traced once. */
const REDUCED_SIDE = 1000;


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
  const debounce = useRef<number | null>(null);
  const dragging = useRef(false);
  /** Each image's Threshold: its automatic value until the maker sets one (criterion 9). */
  const thresholds = useRef(new Map<string, number>());
  /** Each image's suggested background (`specs/0089` criterion 16; empty: transparent). */
  const backgrounds = useRef(new Map<string, string>());
  /** The values each kept full-size result was traced with. */
  const results = useRef(new Map<string, string>());
  const choiceRef = useRef(choice);
  useEffect(() => {
    choiceRef.current = choice;
  }, [choice]);
  /** The view as last read; the sync effect below keeps it and the state together. */
  const viewRef = useRef(view);

  const { busy, percent, setPercent, startTimers, clearTimers } = useTraceProgress(getSession, onChange);

  const runner = useCallback((): TraceRunner => {
    runnerRef.current ??= createTraceRunner();
    return runnerRef.current;
  }, []);
  useEffect(() => () => runnerRef.current?.dispose(), []);
  const { maskStep, last: maskSource } = useTraceMask(getSession, onChange, runner);


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

  const key = (threshold: number, v: TraceValues, id: string) =>
    settingsJson(v, threshold, backgrounds.current.get(id) ?? "");

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

  /** The Threshold of `id`, its automatic value on first use, and its suggested background. */
  const thresholdOf = useCallback(
    async (session: WasmSession, id: string): Promise<number | null> => {
      const known = thresholds.current.get(id);
      if (known !== undefined && backgrounds.current.has(id)) {
        return known;
      }
      const pixels = pixelsOf(session, id, false);
      if (!pixels) {
        return null;
      }
      const defaults = await runner().defaults(pixels);
      backgrounds.current.set(id, defaults.background);
      if (known !== undefined) {
        return known;
      }
      thresholds.current.set(id, defaults.threshold);
      return defaults.threshold;
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
          setLoading(true);
          return;
        }
        const suggested = backgrounds.current.get(id) ?? "";
        const resolve = (v: TraceValues): TraceValues =>
          v.threshold === threshold && (!v.backgroundAuto || v.background === suggested)
            ? v
            : { ...v, threshold, background: v.backgroundAuto ? suggested : v.background };
        setValues(resolve);
        const v = resolve(choiceRef.current.values);
        choiceRef.current = { ...choiceRef.current, values: v };
        const settings = key(threshold, v, id);
        const full = pixelsOf(session, id, false);
        if (!full) {
          setStatus(NO_STATUS);
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
            await maskStep(id, true, settings, stale);
            if (stale()) {
              return;
            }
            const reducedPixels = reduced.width * reduced.height;
            const out = await runner().trace(reduced, settings, onProgress);
            if (stale()) {
              return;
            }
            session.set_trace_preview(id, out.wire);
            if (reducedOnly) {
              setStatus(statusText(readSummary(out.wire), out.seconds, reducedPixels, true, zeroPathsLine(v)));
            }
          }
          if (!fullAfter || reducedOnly) {
            clearTimers();
            onChange();
            return;
          }
        }
        await maskStep(id, false, settings, stale);
        if (stale()) {
          return;
        }
        const out = await runner().trace(full, settings, onProgress);
        if (stale()) {
          return;
        }
        session.set_trace_preview(id, out.wire);
        session.set_trace_result(id, out.wire);
        results.current.set(id, settings);
        setStatus(statusText(readSummary(out.wire), out.seconds, full.width * full.height, false, zeroPathsLine(v)));
        clearTimers();
        onChange();
      } catch (error) {
        if (!(error instanceof TraceCancelled) && !stale()) {
          fail(error, pixels);
        }
      }
    },
    [getSession, startTimers, clearTimers, thresholdOf, runner, setValues, onChange, fail, setPercent, maskStep],
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
      const background = backgrounds.current.get(next.shown);
      setValues((v) => ({
        ...v,
        threshold: known ?? null,
        background: v.backgroundAuto ? (background ?? null) : v.background,
      }));
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
          if (v.method === PICK_COLOURS && !backgrounds.current.has(id)) {
            await thresholdOf(session, id);
          }
          const wanted = key(threshold, v, id);
          if (results.current.get(id) === wanted) {
            continue;
          }
          setProgress(index);
          const full = pixelsOf(session, id, false);
          if (!full) {
            setStatus(NO_STATUS);
            setCreating(false);
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
  }, [getSession, creating, stopped, failure, retrace, status.canCreate, startTimers, clearTimers, runner, thresholdOf, showNotice, onChange, returnFocus, fail, setPercent]);

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



  /** A change to the Mask view asks for the mask of the pixels the last preview step traced
   * (`specs/0089` criterion 21); a running preview is not interrupted. */
  const onViewMode = useCallback(
    (mode: string) => {
      const id = viewRef.current.shown;
      const threshold = choiceRef.current.values.threshold;
      const source = maskSource.current;
      if (mode !== "mask" || !id || threshold === null || source?.id !== id) {
        return;
      }
      const mine = generation.current;
      maskStep(id, source.reduced, key(threshold, choiceRef.current.values, id), () => mine !== generation.current).catch(() => undefined);
    },
    [maskStep, maskSource],
  );
  const { viewMode, setViewMode } = useTraceViewMode(active, getSession, onChange, onViewMode);
  const { setPicking, setMethod, setColourTolerance, setBackgroundAuto, removeSwatch } = useTraceMethod({
    active,
    syncRevision,
    getSession,
    onChange,
    apply,
    choiceRef,
    dragging,
    viewRef,
    backgrounds,
  });

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
      // Esc order (UX notes §6): a running trace stops first, then the armed pipette disarms,
      // then the tool is left.
      const session = getSession();
      if (event.key === "Escape" && session && session.trace_pick_mode() !== "") {
        session.set_trace_pick("");
        onChange();
        return true;
      }
      return false;
    },
    [active, create, creating, stop, getSession, onChange],
  );



  // The progress shows only after 400 ms and then holds (customer 2026-10-30); loading counts as
  // a job that has not reported yet.
  const slot: TraceSlot =
    failure !== null
      ? { kind: "failed", text: failure }
      : stopped
        ? { kind: "stopped" }
        : busy
          ? { kind: "running", percent: loading ? null : percent }
          : { kind: "none" };
  const session = active ? getSession() : null;
  const pickMode = session?.trace_pick_mode() ?? "";
  const shownId = view.shown;
  const suggestedBackground = shownId ? (backgrounds.current.get(shownId) ?? null) : null;
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
    status: withSwatchNote(status, values),
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
    setMethod,
    setColourTolerance,
    setBackgroundAuto,
    removeSwatch,
    pickMode,
    setPicking,
    suggestedBackground,
  };
}
