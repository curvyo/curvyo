import { useCallback, useEffect, useMemo, useRef, useState } from "react";

import type { WasmSession } from "@/lib/editorSession";
import {
  BRIGHTNESS,
  MULTICOLOUR,
  NO_COLOUR_PICKED_LINE,
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
  writeAdvancedOpen,
  writeChoice,
} from "@/lib/traceText";
import {
  readPalette,
} from "@/lib/tracePaletteText";
import {
  ANALYSED_FIELDS,
  type TraceAnalysis,
  type TraceAnalysisState,
  applyAnalysis,
  readAnalysis,
} from "@/lib/traceAnalysis";
import { TraceCancelled, type TraceRunner, createTraceRunner } from "@/lib/traceJobs";
import { trace_auto_threshold, trace_suggest_background } from "@/wasm-bindings/curvyo_editor_wasm.js";
import { NO_VIEW, loadParams, loadPresets, pixelsOf, readView, type ViewState } from "@/lib/traceSource";
import type { TraceApi } from "./traceApi";
import { useTraceMask } from "./useTraceMask";
import { useTracePalette } from "./useTracePalette";
import type { HsvTriple } from "./useStylePanel";
import { useTraceMethod } from "./useTraceMethod";
import { useTraceProgress } from "./useTraceProgress";
import { useTraceViewMode } from "./useTraceViewMode";

export type { TraceApi } from "./traceApi";

/** Debounce of a value change (criterion 10). */
const DEBOUNCE_MS = 150;
/** The longest side of the reduced copy; a picture this small is traced once. */
const REDUCED_SIDE = 1000;
/** "Analysing..." shows after this long, like the trace's progress (UX notes §3). */
const ANALYSING_AFTER_MS = 400;


export function useTrace(
  getSession: () => WasmSession | null,
  onChange: () => void,
  syncRevision: number,
  tool: string,
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
  /** Each image's analysis (`specs/0089` slice F; `null`: it failed), read once per image. */
  const analyses = useRef(new Map<string, TraceAnalysis | null>());
  /** The images whose analysed values the maker changed since the analysis was applied. */
  const customised = useRef(new Set<string>());
  const [analysisState, setAnalysisState] = useState<TraceAnalysisState>({ kind: "none" });
  const analysingTimer = useRef<number | null>(null);
  /** The images the maker changed a setting of before their analysis answered. */
  const editedEarly = useRef(new Set<string>());
  /** The values each kept full-size result was traced with. */
  const results = useRef(new Map<string, string>());
  const choiceRef = useRef(choice);
  useEffect(() => {
    choiceRef.current = choice;
  }, [choice]);
  /** The view as last read; the sync effect below keeps it and the state together. */
  const viewRef = useRef(view);
  /** Reads the palette of a new preview (`useTracePalette`, set once it exists). */
  const paletteRefresh = useRef<() => void>(() => undefined);

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
      if (known !== undefined && backgrounds.current.has(id) && analyses.current.has(id)) {
        return known;
      }
      const pixels = pixelsOf(session, id, false);
      if (!pixels) {
        return null;
      }
      const fresh = !analyses.current.has(id);
      if (fresh && analysingTimer.current === null) {
        analysingTimer.current = window.setTimeout(() => {
          analysingTimer.current = null;
          if (!analyses.current.has(id) && viewRef.current.shown === id) {
            setAnalysisState({ kind: "analysing" });
          }
        }, ANALYSING_AFTER_MS);
      }
      let defaults: Awaited<ReturnType<TraceRunner["defaults"]>>;
      try {
        defaults = await runner().defaults(pixels);
      } catch (error) {
        if (error instanceof TraceCancelled) {
          // A newer job takes over (it shows "Analysing..." again after its 400 ms); Stop has
          // already set "Analysis stopped", which stays.
          setAnalysisState((state) => (state.kind === "analysing" ? { kind: "none" } : state));
          throw error;
        }
        // The worker died: the analysis failed; the automatic Threshold and background come from
        // the page's own module, so the trace still has its values (UX notes §3).
        defaults = {
          threshold: trace_auto_threshold(pixels.rgba, pixels.width, pixels.height),
          background: trace_suggest_background(pixels.rgba, pixels.width, pixels.height),
          analysis: "",
        };
      } finally {
        if (analysingTimer.current !== null) {
          window.clearTimeout(analysingTimer.current);
          analysingTimer.current = null;
        }
      }
      backgrounds.current.set(id, defaults.background);
      if (fresh) {
        // A new image (Frage 83 B): the analysis applies its values at once, unless the maker
        // changed any setting since the image was chosen (lead 2026-10-31: the answer for this
        // image is then dropped and the line reads Custom).
        const analysis = readAnalysis(defaults.analysis);
        analyses.current.set(id, analysis);
        const edited = known !== undefined || editedEarly.current.has(id);
        if (analysis && edited) {
          customised.current.add(id);
        }
        if (analysis && !edited) {
          thresholds.current.set(id, analysis.threshold);
          const values = applyAnalysis(choiceRef.current.values, analysis);
          choiceRef.current = { ...choiceRef.current, values };
          setChoice(choiceRef.current);
          writeChoice(choiceRef.current);
        }
        if (viewRef.current.shown === id) {
          setAnalysisState(analysis ? { kind: edited ? "custom" : "auto", analysis } : { kind: "failed" });
        }
      }
      const kept = thresholds.current.get(id);
      if (kept !== undefined) {
        return kept;
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
            paletteRefresh.current();
            if (reducedOnly) {
              setStatus(statusText(readSummary(out.wire), out.seconds, reducedPixels, true, zeroPathsLine(v), colourCounts(session)));
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
        paletteRefresh.current();
        results.current.set(id, settings);
        setStatus(statusText(readSummary(out.wire), out.seconds, full.width * full.height, false, zeroPathsLine(v), colourCounts(session)));
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
    const shownAnalysis = next.shown ? analyses.current.get(next.shown) : undefined;
    setAnalysisState(
      !active || !next.shown || shownAnalysis === undefined
        ? { kind: "none" }
        : shownAnalysis === null
          ? { kind: "failed" }
          : { kind: customised.current.has(next.shown) ? "custom" : "auto", analysis: shownAnalysis },
    );
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

  /** Takes a new choice: remembered, the kept results dropped, a preview after the debounce. An
   * edit of an analysed value moves the shown image's analysis line to Custom (UX notes §3). */
  const apply = useCallback(
    (next: TraceChoice) => {
      const session = getSession();
      const id = viewRef.current.shown;
      const analysis = id ? analyses.current.get(id) : undefined;
      if (id && !analyses.current.has(id)) {
        editedEarly.current.add(id);
      }
      if (id && analysis && ANALYSED_FIELDS.some((f) => next.values[f] !== choiceRef.current.values[f])) {
        customised.current.add(id);
        setAnalysisState({ kind: "custom", analysis });
      }
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

  const palette = useTracePalette({ active, values, getSession, onChange, apply, choiceRef });
  // The Recolour picker converts as the Style section does (Rust's rounding).
  const hsvOf = useCallback((rgb: number): HsvTriple => {
    const [hue, saturation, value] = getSession()?.colour_hsv(rgb) ?? [-1, 0, 0];
    return [hue, saturation, value];
  }, [getSession]);
  const rgbOf = useCallback(
    (hue: number, saturation: number, value: number) => getSession()?.hsv_colour(hue, saturation, value) ?? 0,
    [getSession],
  );
  useEffect(() => {
    paletteRefresh.current = palette.refresh;
  }, [palette.refresh]);

  const setValue = useCallback(
    <K extends keyof TraceValues>(name: K, value: TraceValues[K], isDragging = false) => {
      dragging.current = isDragging;
      // Leaving Multicolour in Brightness disarms the palette pipette (UX notes §7).
      const session = getSession();
      if (name === "output" && value !== MULTICOLOUR && choiceRef.current.values.method === BRIGHTNESS && session?.trace_pick_mode() === "colours") {
        session.set_trace_pick("");
      }
      if (name === "threshold" && viewRef.current.shown && typeof value === "number") {
        thresholds.current.set(viewRef.current.shown, value);
      }
      apply(editChoice(choiceRef.current, name, value));
    },
    [apply, getSession],
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
    // The analysis never answered: its line says so instead of "Analysing..." (customer 2026-10-31).
    setAnalysisState((state) => (state.kind === "analysing" ? { kind: "stopped" } : state));
    setCreating(false);
    setLoading(false);
    setStopped(true);
    getSession()?.set_trace_preview_stale(true);
    setStatus((s) => ({ ...s, line: "", muted: "", message: "", canCreate: false }));
    onChange();
  }, [cancelAll, getSession, onChange]);

  /** Analyse (UX notes §3, §22.3, Frage 83 B): the shown image's analysed values back to its
   * analysis in one go, the line back to Auto, no confirmation and no history step; after a
   * failure the analysis runs again. */
  const analyse = useCallback(() => {
    const id = viewRef.current.shown;
    if (!id) {
      return;
    }
    const analysis = analyses.current.get(id);
    customised.current.delete(id);
    editedEarly.current.delete(id);
    if (!analysis) {
      analyses.current.delete(id);
      thresholds.current.delete(id);
      backgrounds.current.delete(id);
      setAnalysisState({ kind: "none" });
    } else {
      thresholds.current.set(id, analysis.threshold);
      const values = applyAnalysis(choiceRef.current.values, analysis);
      const next = { ...choiceRef.current, values };
      choiceRef.current = next;
      setChoice(next);
      writeChoice(next);
      setAnalysisState({ kind: "auto", analysis });
    }
    results.current.clear();
    getSession()?.clear_trace_results();
    dragging.current = false;
    setStopped(false);
    setFailure(null);
    schedule(0);
  }, [getSession, schedule]);

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
        // No notice after creating paths: the new group being selected is the feedback (decided by
        // the customer, 2026-10-31).
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
  }, [getSession, creating, stopped, failure, retrace, status.canCreate, startTimers, clearTimers, runner, thresholdOf, onChange, returnFocus, fail, setPercent]);

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
  const { setPicking, setMethod, setColourTolerance, setBackgroundAuto } = useTraceMethod({
    active,
    syncRevision,
    getSession,
    onChange,
    apply,
    choiceRef,
    dragging,
    viewRef,
    backgrounds,
    palettePick: palette.pick,
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
      // then the tool is left; a Merge into waiting for its target is cancelled before the tool.
      const session = getSession();
      if (event.key === "Escape" && palette.merging) {
        palette.cancelMerge();
        return true;
      }
      if (event.key === "Escape" && session && session.trace_pick_mode() !== "") {
        session.set_trace_pick("");
        onChange();
        return true;
      }
      return false;
    },
    [active, create, creating, stop, getSession, onChange, palette],
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
  const advancedStatus = advancedStatusOf(status, palette.note);
  // A disabled Trace button states why (`ux-notes.md` §22.2): nothing picked, or a result that
  // has no paths.
  const traceBlocked =
    slot.kind !== "none"
      ? ""
      : values.method === PICK_COLOURS && values.swatches.length === 0
        ? NO_COLOUR_PICKED_LINE
        : status.canCreate
          ? ""
          : status.line || status.message;
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
    setMethod,
    setColourTolerance,
    setBackgroundAuto,
    pickMode,
    setPicking,
    suggestedBackground,
    analysis: analysisState,
    analyse,
    advancedStatus,
    traceBlocked,
    palette,
    hsvOf,
    rgbOf,
  };
}

/** The colours and joined regions of the shown multicolour preview, `null` for one colour. */
function colourCounts(session: WasmSession): { colours: number; joined: number } | null {
  const info = readPalette(session.trace_palette());
  return info ? { colours: info.layers.length, joined: info.joined } : null;
}

/** The Advanced status line (`ux-notes.md` §22.2): the result counts or the message that replaces
 * them, and in the muted line the specks, the joined regions and what a palette action did. */
function advancedStatusOf(status: TraceStatusText, note: string): { text: string; muted: string } {
  return { text: status.message || status.counts, muted: [status.countsMuted, note].filter((t) => t !== "").join(", ") };
}
