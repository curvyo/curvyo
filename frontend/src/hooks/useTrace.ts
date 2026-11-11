import { useCallback, useEffect, useMemo, useRef, useState } from "react";

import type { WasmSession } from "@/lib/editorSession";
import {
  MULTICOLOUR,
  MASK_FAILED,
  MASK_OUTSIDE_IMAGE,
  NO_STATUS,
  TRACE_MIXED,
  nothingToTrace,
  maskText,
  PICK_COLOURS,
  zeroPathsLine,
  failureReason,
  failureText,
  previewIsReduced,
  READING_TEXT,
  readsText,
  traceAnnouncement,
  pickedAnnouncement,
  textLinesText,
  shapesText,
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
  analysisLine,
  applyAnalysis,
  readAnalysis,
} from "@/lib/traceAnalysis";
import { type FitFace, OcrFailed, TraceCancelled, type TracePixels, type TraceRunner, createTraceRunner, ocrWeights } from "@/lib/traceJobs";
import { type FitInputs, JobProgress, type TraceStage, decideTextLines, traceWithText } from "@/lib/traceTextFlow";
import { readTolerance } from "@/lib/recogniseText";
import { favouritesAmong } from "@/hooks/fontLibraryPage";
import { listedFileBytes } from "@/hooks/useText";
import type { FontFace } from "@/lib/textText";
import { trace_auto_threshold, trace_suggest_background, trace_text_failed_notice } from "@/wasm-bindings/curvyo_editor_wasm.js";
import { type RegionRead, forgetRegionValues, partAutoValues, readRegion, regionAnswer, viewValues } from "@/lib/traceMaskCaches";
import { NO_VIEW, joinWires, loadParams, loadPresets, partPixelsOf, pixelsOf, readView, type ViewState } from "@/lib/traceSource";
import { NO_PART_LINE, type PartJob, type TracePartInfo, mayCrop, partEmptyText, partKey, partsCount, readParts, traceParts } from "@/lib/traceParts";
import type { TraceApi } from "./traceApi";
import { useTraceParts } from "./useTraceParts";
import { useTracePalette } from "./useTracePalette";
import { useTraceMethod } from "./useTraceMethod";
import { useTraceProgress } from "./useTraceProgress";

export type { TraceApi } from "./traceApi";

/** Debounce of a value change (criterion 10). */
const DEBOUNCE_MS = 150;
/** The longest side of the reduced copy; a picture this small is traced once. */
const REDUCED_SIDE = 1000;
/** "Analysing..." shows after this long, like the trace's progress (UX notes §3). */
const ANALYSING_AFTER_MS = 400;

/** What text recognition in Trace needs from the editor (`specs/0091`): the listed faces (for the
 * favourites tier) and where the guessed-letters warning goes after Create paths. */
export interface TraceTextOptions {
  faces: readonly FontFace[];
  onNotice: (text: string) => void;
}


export function useTrace(
  getSession: () => WasmSession | null,
  onChange: () => void,
  syncRevision: number,
  tool: string,
  returnFocus: () => void,
  textOptions?: TraceTextOptions,
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
  const runnerRef = useRef<TraceRunner | null>(null);
  /** A reduced preview is shown and its full-size trace follows (`ux-notes.md` §24.5). */
  const fullFollows = useRef(false);
  const generation = useRef(0);
  const debounce = useRef<number | null>(null);
  const dragging = useRef(false);
  /** Each image's Threshold: its automatic value until the maker sets one (criterion 9). */
  const thresholds = useRef(new Map<string, number>());
  /** Each image's suggested background (`specs/0089` criterion 16; empty: transparent). */
  const backgrounds = useRef(new Map<string, string>());
  /** Each image's analysis (`specs/0089` slice F; `null`: it failed), read once per image. */
  const analyses = useRef(new Map<string, TraceAnalysis | null>());
  /** The images whose Threshold the maker set: a new mask keeps it (`specs/0094`). */
  const makerThreshold = useRef(new Set<string>());
  /** Per image the analysed values the maker set since its analysis (`specs/0094`): a new mask's
   * analysis keeps them. */
  const makerFields = useRef(new Map<string, Set<keyof TraceValues>>());
  /** The images whose analysis is read again for a new mask region. */
  const regionReread = useRef(new Set<string>());
  /** Each region's automatic values, by image and mask, read once while the tool is used
   * (`ux-notes.md` §25.2). */
  const regionReads = useRef(new Map<string, RegionRead>());
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
  /** Another part's settings into the bar (`specs/0094` slice 2): its Threshold is the image's
   * kept one from now on, so the sync below does not put the last part's back. */
  const loadPart = useCallback((next: TraceChoice) => {
    choiceRef.current = next;
    setChoice(next);
    const id = viewRef.current.shown;
    if (id && next.values.threshold !== null) {
      thresholds.current.set(id, next.values.threshold);
      makerThreshold.current.add(id);
    }
  }, []);
  const partsApi = useTraceParts(getSession, onChange, view.parts, choiceRef, loadPart);
  const partsRef = useRef(partsApi);
  /** Each part's automatic background, read from the region it was last read for (`specs/0094`
   * slice 2, region-only analysis per part), by part id. */
  const partAuto = useRef(new Map<number, { region: string; background: string }>());
  useEffect(() => {
    partsRef.current = partsApi;
  }, [partsApi]);
  /** Reads the palette of a new preview (`useTracePalette`, set once it exists). */
  const paletteRefresh = useRef<() => void>(() => undefined);
  /** The text is being read (`specs/0091` UX notes §3: "Reading text"). */
  const [reading, setReading] = useState(false);
  /** Counts the reads, so a stopped one does not clear the label of the next. */
  const readingCall = useRef(0);
  /** The images whose text could not be read: traced without text until the tool is left. */
  const ocrFailed = useRef(new Set<string>());
  /** The text lines of the shown preview: set and found (UX notes §4). */
  const [textCounts, setTextCounts] = useState<[number, number]>([0, 0]);
  const [shapeCount, setShapeCount] = useState(0);
  /** "1 shape crosses the mask edge unclipped." for the shown preview (`specs/0094`), or empty. */
  const [unclipped, setUnclipped] = useState("");
  /** The colour last taken out of the picked colours, and the set it left, for the hidden region
   * ("Removed #RRGGBB", `ux-notes.md` §25.4); it is said until the set changes again. */
  const [removed, setRemoved] = useState<{ hex: string; left: string } | null>(null);
  const textOptionsRef = useRef(textOptions);
  useEffect(() => {
    textOptionsRef.current = textOptions;
  }, [textOptions]);

  const { busy, percent, setPercent, startTimers, clearTimers } = useTraceProgress(getSession, onChange);

  const runner = useCallback((): TraceRunner => {
    runnerRef.current ??= createTraceRunner();
    return runnerRef.current;
  }, []);
  useEffect(() => () => runnerRef.current?.dispose(), []);


  /** Ends whatever runs: the debounce, the jobs and their timers. */
  const cancelAll = useCallback(() => {
    generation.current += 1;
    fullFollows.current = false;
    if (debounce.current !== null) {
      window.clearTimeout(debounce.current);
      debounce.current = null;
    }
    runnerRef.current?.cancel();
    clearTimers();
  }, [clearTimers]);

  // The settings, and whether the text is read (`specs/0091`): a kept result is reused only for
  // both.
  const key = (threshold: number, v: TraceValues, id: string) =>
    settingsJson(v, threshold, backgrounds.current.get(id) ?? "", viewRef.current.mask);
  const resultKey = (settings: string, v: TraceValues) => `${settings}|${readsText(v) ? "text" : ""}`;

  /** The OCR lines JSON of image `id` (`specs/0091` decision 1): kept by the session, or read
   * once in the worker; empty when the text is not read or the reader failed (then the bar says
   * so and the trace runs without text, UX notes §3). A stop (`stale`) while the weights load or
   * the lines are read keeps nothing: the stop may come before the worker has a job to cancel, or
   * after it answered (`specs/0091` criterion 19). */
  const textLines = useCallback(
    async (session: WasmSession, id: string, v: TraceValues, onProgress: (p: number) => void, pixels: TracePixels, stale: () => boolean): Promise<string> => {
      if (!readsText(v) || ocrFailed.current.has(id)) {
        return "";
      }
      const kept = session.trace_ocr_lines(id);
      if (kept !== "") {
        return kept;
      }
      // A stopped read ends after a newer one started (Retry): only the newest clears the label.
      const read = ++readingCall.current;
      setReading(true);
      try {
        // `pixels` (the full picture the trace takes next) go to the worker and come back.
        const weights = await ocrWeights();
        if (stale()) {
          throw new TraceCancelled();
        }
        const found = await runner().ocr(pixels, weights, onProgress);
        pixels.rgba = found.rgba;
        if (stale()) {
          throw new TraceCancelled();
        }
        if (found.error !== "") {
          ocrFailed.current.add(id);
          return "";
        }
        session.set_trace_ocr_lines(id, found.lines);
        return session.trace_ocr_lines(id);
      } catch (error) {
        // Only the reader failing (its weights not fetched, or not read) traces without text; a
        // stop, or a worker that died, ends the trace itself (Failed, Retry reads again; `0025`
        // criterion 54).
        if (!(error instanceof OcrFailed || error instanceof TypeError)) {
          throw error;
        }
        ocrFailed.current.add(id);
        return "";
      } finally {
        if (read === readingCall.current) {
          setReading(false);
        }
      }
    },
    [runner],
  );

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
      let defaults: RegionRead;
      const mask = viewRef.current.mask;
      try {
        defaults = await readRegion(regionReads.current, id, mask, () => runner().defaults(pixels, mask));
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
        if (regionReread.current.delete(id)) {
          // `specs/0094`: a new mask region; what the analysis filled in follows the region, what
          // the maker set stays. The bar showed the old values until now and changes once; a
          // failed analysis keeps them (`ux-notes.md` §25.2).
          const maker = makerFields.current.get(id) ?? new Set<keyof TraceValues>();
          const shown = choiceRef.current.values.threshold;
          if (!makerThreshold.current.has(id)) {
            const next = analysis?.threshold ?? shown;
            if (next !== null) {
              thresholds.current.set(id, next);
            }
          }
          const values = regionAnswer(choiceRef.current.values, analysis, maker);
          if (values !== choiceRef.current.values) {
            choiceRef.current = { ...choiceRef.current, values };
            setChoice(choiceRef.current);
            writeChoice(choiceRef.current);
          }
          if (viewRef.current.shown === id) {
            setAnalysisState(analysis ? { kind: maker.size > 0 ? "custom" : "auto", analysis } : { kind: "failed" });
          }
          const kept = thresholds.current.get(id);
          if (kept !== undefined) {
            return kept;
          }
        }
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

  /** What the font fit needs from the page (`specs/0091` decision 4): the Recognise text
   * tolerance, the favourites and the font files (the main thread only reads them). */
  const fitInputs = useCallback((): FitInputs => {
    const faces = textOptionsRef.current?.faces ?? [];
    return {
      tolerance: readTolerance(typeof localStorage === "undefined" ? null : localStorage),
      favourites: JSON.stringify([...favouritesAmong([...new Set(faces.map((f) => f.family))])]),
      loadFaces: async (wanted: [string, number][]): Promise<FitFace[]> => {
        const read = await Promise.allSettled(wanted.map(([file, index]) => listedFileBytes(file).then((bytes) => ({ file, index, bytes: new Uint8Array(bytes) }))));
        return read.flatMap((r) => (r.status === "fulfilled" ? [r.value] : []));
      },
    };
  }, []);

  /** Fits the kept results' text lines to the faces in the trace worker (several pictures traced
   * one by one, each with its own fit; Create paths fits them together), the percent in the Trace
   * button; the session then shows the texts. `false` when a newer job took over (`stale`). */
  const fitTexts = useCallback(
    async (session: WasmSession, stale: () => boolean, report?: (p: number) => void): Promise<boolean> => {
      const fit = fitInputs();
      const request = session.trace_fit_request(fit.tolerance, fit.favourites);
      if (request === "") {
        return true;
      }
      const result = await runner().fit(request, fit.loadFaces, (p) => {
        if (!stale()) {
          (report ?? setPercent)(p);
        }
      });
      if (stale()) {
        return false;
      }
      getSession()?.set_trace_fit(request, result);
      return true;
    },
    [getSession, runner, setPercent, fitInputs],
  );

  /** Part `part`'s values with Remove background's Auto taking the part's own region's background. */
  const partValues = (part: TracePartInfo): TraceValues => {
    const v = partsRef.current.choiceOf(part).values;
    const auto = partAuto.current.get(part.id);
    return v.backgroundAuto && auto ? { ...v, background: auto.background } : v;
  };

  /** Reads each contributing part's automatic Threshold, background and analysis from its own
   * clip once per clip (region-only analysis per part, `specs/0094` slice 2); what the maker set
   * for a part stays. `false` when a newer job took over. */
  const resolveParts = useCallback(
    async (session: WasmSession, id: string, stale: () => boolean): Promise<boolean> => {
      for (const part of readParts(session.trace_parts())) {
        if (!part.contributes || partAuto.current.get(part.id)?.region === part.mask) {
          continue;
        }
        const pixels = pixelsOf(session, id, false);
        if (!pixels) {
          return false;
        }
        let read: { threshold: number; background: string; analysis: string };
        let answered = true;
        try {
          read = await runner().defaults(pixels, part.mask);
        } catch (error) {
          if (error instanceof TraceCancelled) {
            throw error;
          }
          answered = false;
          read = { threshold: trace_auto_threshold(pixels.rgba, pixels.width, pixels.height), background: trace_suggest_background(pixels.rgba, pixels.width, pixels.height), analysis: "" };
        }
        // A failed read is not kept: the next preview (Analyse, Retry) asks the worker again.
        if (answered) {
          partAuto.current.set(part.id, { region: part.mask, background: read.background });
        }
        const analysis = readAnalysis(read.analysis);
        const maker = partsRef.current.makerOf(part.id);
        const choice = partsRef.current.choiceOf(part);
        partsRef.current.setChoiceOf(part.id, { ...choice, values: partAutoValues(choice.values, { threshold: read.threshold, background: read.background, analysis }, maker) });
        if (part.active && analysis) {
          setAnalysisState({ kind: maker.size > 0 ? "custom" : "auto", analysis });
        }
        if (stale()) {
          return false;
        }
      }
      return true;
    },
    [runner],
  );

  /** The parts' jobs of image `id` (`specs/0094` slice 2): each contributing part's settings with
   * its clip, traced one by one and kept per part (criterion 25), joined by the session into the
   * preview, with `keep` also into the result Create paths writes; `null` when a newer job took
   * over. */
  const joinedJob = useCallback(
    async (session: WasmSession, id: string, reduced: boolean, keep: boolean, lines: string, onProgress: (p: number) => void, stale: () => boolean) => {
      const background = backgrounds.current.get(id) ?? "";
      const auto = thresholds.current.get(id) ?? null;
      const values = readParts(session.trace_parts()).map((part) => ({ part, v: partValues(part) }));
      const jobs: (PartJob | null)[] = values.map(({ part, v }) => {
        const threshold = v.threshold ?? auto;
        if (!part.contributes || threshold === null || nothingToTrace(v) !== "") {
          return null;
        }
        const settings = settingsJson(v, threshold, partAuto.current.get(part.id)?.background ?? background, part.mask);
        const text = lines !== "" && readsText(v);
        return { settings, text, key: partKey(settings, text, reduced), crop: mayCrop(v, text) };
      });
      const out = await traceParts(
        {
          pixels: (index, crop) =>
            crop ? partPixelsOf(session, id, index, reduced, values[index].v.specks, values[index].v.upscale === 0) : pixelsOf(session, id, reduced),
          join: (wires) => joinWires(session, id, wires, keep),
        },
        runner(),
        jobs,
        partsRef.current.cache.current,
        lines,
        onProgress,
        stale,
      );
      return out === null ? null : { ...out, any: jobs.some((j) => j !== null) };
    },
    [runner],
  );

  /** The preview with two or more parts (`specs/0094` slice 2): as the single trace, the reduced
   * copy first and the full size after, each a join of the parts; the parts that read text take
   * the read lines and the fit runs once over the joined result. The picture's pixel count. */
  const runJoinedPreview = useCallback(
    async (session: WasmSession, id: string, fullAfter: boolean, stale: () => boolean, onProgress: (p: number) => void): Promise<number> => {
      const threshold = await thresholdOf(session, id);
      if (stale()) {
        return 0;
      }
      const full = threshold === null ? null : pixelsOf(session, id, false);
      if (threshold === null || !full) {
        setStatus(NO_STATUS);
        setLoading(true);
        return 0;
      }
      if (!(await resolveParts(session, id, stale))) {
        return 0;
      }
      // The bar's part takes the image's automatic Threshold until one is set.
      if (choiceRef.current.values.threshold === null) {
        choiceRef.current = { ...choiceRef.current, values: { ...choiceRef.current.values, threshold } };
        setChoice(choiceRef.current);
      }
      const infos = readParts(session.trace_parts());
      const textValues = infos.filter((p) => p.contributes).map((p) => partsRef.current.choiceOf(p).values).find((v) => readsText(v));
      const text = textValues !== undefined && !ocrFailed.current.has(id);
      const small = full.width <= REDUCED_SIDE && full.height <= REDUCED_SIDE;
      const reducedOnly = previewIsReduced(full.width, full.height);
      const fullTrace = small || (fullAfter && !reducedOnly);
      const stages: TraceStage[] = [
        ...(text && session.trace_ocr_lines(id) === "" ? (["read"] as const) : []),
        ...(small ? [] : (["reduced"] as const)),
        ...(fullTrace ? (["trace"] as const) : []),
        ...(text && fullTrace ? (["fit"] as const) : []),
      ];
      const progress = new JobProgress(stages, onProgress);
      const pixels = full.width * full.height;
      const lines = text && textValues ? await textLines(session, id, textValues, progress.stage("read"), full, stale) : "";
      if (stale()) {
        return pixels;
      }
      if (reducedOnly) {
        startTimers(true);
      }
      const bar = choiceRef.current.values;
      const show = (out: { header: Float64Array; seconds: number; any: boolean }, reduced: boolean) => {
        paletteRefresh.current();
        const [set, found] = session.trace_text_counts();
        setTextCounts([set, found]);
        setShapeCount(session.trace_shape_count());
        setUnclipped(session.trace_unclipped_notice());
        if (!out.any) {
          setStatus({ ...NO_STATUS, line: NO_PART_LINE });
          return;
        }
        const s = statusText(readSummary(out.header), out.seconds, pixels, reduced, zeroPathsLine(bar), colourCounts(session));
        // A picture of text only traces to no path and is still something to create (`specs/0091`).
        setStatus(set > 0 && !s.canCreate && s.message === "" ? { ...s, line: "", canCreate: true } : s);
      };
      if (!small) {
        const out = await joinedJob(session, id, true, false, lines, progress.stage("reduced"), stale);
        if (out === null || stale()) {
          return pixels;
        }
        if (out.missing) {
          setLoading(true);
          return pixels;
        }
        // The full-size result follows: the hidden region waits for it (`ux-notes.md` §24.5).
        fullFollows.current = fullAfter && !reducedOnly;
        show(out, true);
        if (!fullAfter || reducedOnly) {
          clearTimers();
          onChange();
          return pixels;
        }
      }
      const out = await joinedJob(session, id, false, true, lines, progress.stage("trace"), stale);
      if (out === null || stale()) {
        return pixels;
      }
      if (out.missing) {
        setLoading(true);
        return pixels;
      }
      if (text && !(await fitTexts(session, stale, progress.stage("fit")))) {
        return pixels;
      }
      results.current.set(id, "parts");
      fullFollows.current = false;
      show(out, false);
      clearTimers();
      onChange();
      return pixels;
    },
    [thresholdOf, resolveParts, textLines, startTimers, joinedJob, fitTexts, clearTimers, onChange],
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
      if (viewRef.current.maskFailed) {
        setUnclipped("");
        setStatus({ ...NO_STATUS, line: MASK_FAILED });
        onChange();
        return;
      }
      if (readParts(viewRef.current.parts).length > 1) {
        // `specs/0094` slice 2: the parts' join; a part whose mask misses the image adds nothing.
        startTimers();
        let pixels = 0;
        try {
          pixels = await runJoinedPreview(session, id, fullAfter, stale, (p) => {
            if (!stale()) {
              setPercent(p);
            }
          });
        } catch (error) {
          if (!(error instanceof TraceCancelled) && !stale()) {
            fail(error, pixels);
          }
        }
        return;
      }
      if (viewRef.current.maskOutsideImage || viewRef.current.maskFailed) {
        // `specs/0094` criterion 7: nothing to trace, no job; the preview is empty.
        session.set_trace_preview(id, new Float64Array(7));
        setUnclipped("");
        setStatus({ ...NO_STATUS, line: viewRef.current.maskFailed ? MASK_FAILED : MASK_OUTSIDE_IMAGE });
        onChange();
        return;
      }
      if (nothingToTrace(choiceRef.current.values) !== "") {
        // Pick colours with nothing picked, or a palette with every colour deleted, traces
        // nothing: no "0 paths" outcome, the outline shows nothing until the first colour
        // (`ux-notes.md` §25.3, §27).
        session.set_trace_preview(id, new Float64Array(7));
        setUnclipped("");
        setStatus(NO_STATUS);
        paletteRefresh.current();
        onChange();
        return;
      }
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
        // `specs/0091` criterion 25: one percent over the whole job; the lines are decided before
        // the trace, which then runs once.
        const small = full.width <= REDUCED_SIDE && full.height <= REDUCED_SIDE;
        const reducedOnly = previewIsReduced(full.width, full.height);
        const text = readsText(v) && !ocrFailed.current.has(id);
        // The fit decides the lines for the full-size trace only: a reduced preview while a value
        // is dragged shows the read lines' own shapes, as before.
        const fullTrace = small || (fullAfter && !reducedOnly);
        const stages: TraceStage[] = [
          ...(text && session.trace_ocr_lines(id) === "" ? (["read"] as const) : []),
          ...(text && fullTrace ? (["fit"] as const) : []),
          ...(small ? [] : (["reduced"] as const)),
          ...(fullTrace ? (["trace"] as const) : []),
        ];
        const progress = new JobProgress(stages, onProgress);
        const read = await textLines(session, id, v, progress.stage("read"), full, stale);
        if (stale()) {
          return;
        }
        const lines = fullTrace ? await decideTextLines(session, runner(), id, full, settings, read, fitInputs(), progress.stage("fit"), stale) : read;
        if (lines === null || stale()) {
          return;
        }
        const failedText = readsText(v) && ocrFailed.current.has(id);
        let textSet = 0;
        const counts = () => {
          const [set, found] = session.trace_text_counts();
          textSet = set;
          setTextCounts([set, found]);
          setShapeCount(session.trace_shape_count());
          setUnclipped(session.trace_unclipped_notice());
        };
        // A picture of text only traces to no path and is still something to create
        // (`specs/0091`); a failed read says so in the bar (UX notes §3).
        const withText = (s: TraceStatusText): TraceStatusText => {
          const created = textSet > 0 && !s.canCreate && s.message === "" ? { ...s, line: "", canCreate: true } : s;
          return failedText && created.line === "" ? { ...created, line: trace_text_failed_notice() } : created;
        };
        pixels = full.width * full.height;
        // Over 4 Mpx the preview is the reduced copy only; Create paths traces at full size
        // (criterion 52).
        if (reducedOnly) {
          startTimers(true);
        }
        if (!small) {
          const reduced = pixelsOf(session, id, true);
          if (reduced) {
            const reducedPixels = reduced.width * reduced.height;
            const out = await runner().trace(reduced, settings, progress.stage("reduced"), lines);
            if (stale()) {
              return;
            }
            session.set_trace_preview(id, out.wire);
            paletteRefresh.current();
            counts();
            if (reducedOnly) {
              setStatus(withText(statusText(readSummary(out.wire), out.seconds, reducedPixels, true, zeroPathsLine(v), colourCounts(session))));
            }
          }
          if (!fullAfter || reducedOnly) {
            clearTimers();
            onChange();
            return;
          }
        }
        // UX notes §2: the Trace view shows the texts in their faces; the fit decided before the
        // trace stays with its result.
        const out = await runner().trace(full, settings, progress.stage("trace"), lines);
        if (stale()) {
          return;
        }
        session.set_trace_preview(id, out.wire);
        session.set_trace_result(id, out.wire);
        paletteRefresh.current();
        counts();
        results.current.set(id, resultKey(settings, v));
        setStatus(withText(statusText(readSummary(out.wire), out.seconds, full.width * full.height, false, zeroPathsLine(v), colourCounts(session))));
        clearTimers();
        onChange();
      } catch (error) {
        if (!(error instanceof TraceCancelled) && !stale()) {
          fail(error, pixels);
        }
      }
    },
    [getSession, startTimers, clearTimers, thresholdOf, runner, setValues, onChange, fail, setPercent, textLines, fitInputs, runJoinedPreview],
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
      next.images.join() === previous.images.join() &&
      next.mask === previous.mask &&
      next.maskPaths === previous.maskPaths &&
      next.maskOutsideImage === previous.maskOutsideImage &&
      next.maskFailed === previous.maskFailed &&
      next.parts === previous.parts
    ) {
      return;
    }
    if (next.shown !== previous.shown) {
      partsRef.current.cache.current.clear();
      partAuto.current.clear();
    }
    // `specs/0094` slice 2: with two or more parts a part's values are its own; a chip pick or a
    // part's new paths do not read the analysis again.
    const severalParts = readParts(next.parts).length > 1 || readParts(previous.parts).length > 1;
    if (next.mask !== previous.mask && next.shown && !severalParts) {
      // `specs/0094` criterion 9: the automatic values and the analysis of the new region; a
      // Threshold the maker set stays.
      regionReread.current.add(next.shown);
      forgetRegionValues(next.shown, { thresholds: thresholds.current, backgrounds: backgrounds.current, analyses: analyses.current }, makerThreshold.current);
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
      // A new region of the same image keeps the shown values until its analysis answers
      // (`ux-notes.md` §25.2).
      const sameImage = next.shown === previous.shown;
      const known = thresholds.current.get(next.shown);
      const background = backgrounds.current.get(next.shown);
      setValues((v) => viewValues(v, sameImage, known, background));
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
      if (id) {
        const changed = ANALYSED_FIELDS.filter((f) => next.values[f] !== choiceRef.current.values[f]);
        // `specs/0094` slice 2: what the maker set stays with the active part.
        partsRef.current.markMaker(changed);
        if (changed.length > 0) {
          const fields = makerFields.current.get(id) ?? new Set<keyof TraceValues>();
          changed.forEach((f) => fields.add(f));
          makerFields.current.set(id, fields);
        }
      }
      if (id && analysis && ANALYSED_FIELDS.some((f) => next.values[f] !== choiceRef.current.values[f])) {
        customised.current.add(id);
        setAnalysisState({ kind: "custom", analysis });
      }
      choiceRef.current = next;
      setChoice(next);
      // Only the first part's values are remembered (`specs/0094` criterion 29).
      if (readParts(viewRef.current.parts).findIndex((p) => p.active) <= 0) {
        writeChoice(next);
      }
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

  const palette = useTracePalette({ active, traced: partsApi.parts.find((part) => part.active)?.contributes ?? true, values, getSession, apply, choiceRef });
  useEffect(() => {
    paletteRefresh.current = palette.refresh;
  }, [palette.refresh]);
  // Another part became active: the session's palette is the one of the part before until the parts
  // are joined again (kept results, so quick), which reads the new active part's palette.
  const activePartId = partsApi.parts.find((part) => part.active)?.id;
  const shownPartId = useRef(activePartId);
  useEffect(() => {
    if (shownPartId.current === activePartId) {
      return;
    }
    shownPartId.current = activePartId;
    if (active && partsApi.parts.length > 1) {
      schedule(0);
    }
  }, [activePartId, active, partsApi.parts.length, schedule]);

  const setValue = useCallback(
    <K extends keyof TraceValues>(name: K, value: TraceValues[K], isDragging = false) => {
      dragging.current = isDragging;
      // Leaving Multicolour disarms the palette pipette (UX notes §7, §27).
      const session = getSession();
      if (name === "output" && value !== MULTICOLOUR && choiceRef.current.values.method !== PICK_COLOURS && session?.trace_pick_mode() === "colours") {
        session.set_trace_pick("");
      }
      if (name === "threshold" && viewRef.current.shown && typeof value === "number") {
        thresholds.current.set(viewRef.current.shown, value);
        makerThreshold.current.add(viewRef.current.shown);
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
    makerThreshold.current.delete(id);
    makerFields.current.delete(id);
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
    const severalParts = readParts(current.parts).length > 1;
    if (!session || creating || current.images.length === 0 || (current.maskFailed || (!severalParts && current.maskOutsideImage))) {
      return;
    }
    const threshold = choiceRef.current.values.threshold;
    // Nothing to trace: Enter and Ctrl+Enter do nothing, silently (`ux-notes.md` §25.3, §27).
    if (threshold === null || nothingToTrace(choiceRef.current.values) !== "" || (!status.canCreate && results.current.size > 0)) {
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
        if (readParts(current.parts).length > 1) {
          // `specs/0094` slice 2: the parts joined at full size (kept parts from the preview), one
          // percent over the job, then one Create paths.
          const id = current.images[0];
          const stale = () => mine !== generation.current;
          const progress = (p: number) => {
            if (!stale()) {
              setPercent(p);
            }
          };
          const full = pixelsOf(session, id, false);
          if (!full) {
            setStatus(NO_STATUS);
            setCreating(false);
            setLoading(true);
            return;
          }
          pixels = full.width * full.height;
          if (previewIsReduced(full.width, full.height)) {
            startTimers(true);
          }
          if (!(await resolveParts(session, id, stale))) {
            return;
          }
          const infos = readParts(session.trace_parts());
          const textValues = infos.filter((p) => p.contributes).map((p) => partsRef.current.choiceOf(p).values).find((pv) => readsText(pv));
          const text = textValues !== undefined && !ocrFailed.current.has(id);
          const job = new JobProgress([...(text && session.trace_ocr_lines(id) === "" ? (["read"] as const) : []), "trace", ...(text ? (["fit"] as const) : [])], progress);
          const lines = text && textValues ? await textLines(session, id, textValues, job.stage("read"), full, stale) : "";
          if (stale()) {
            return;
          }
          const out = await joinedJob(session, id, false, true, lines, job.stage("trace"), stale);
          if (out === null || stale()) {
            return;
          }
          if (text && !session.trace_text_fitted() && !(await fitTexts(session, stale, job.stage("fit")))) {
            return;
          }
          session.create_traced_paths().free();
          const warning = session.trace_created_warning();
          if (warning !== "") {
            textOptionsRef.current?.onNotice(warning);
          }
          clearTimers();
          setCreating(false);
          onChange();
          returnFocus();
          return;
        }
        for (const [index, id] of current.images.entries()) {
          if (v.method === PICK_COLOURS && !backgrounds.current.has(id)) {
            await thresholdOf(session, id);
          }
          const settings = key(threshold, v, id);
          const wanted = resultKey(settings, v);
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
          const progress = (p: number) => {
            if (mine === generation.current) {
              setPercent(p);
            }
          };
          // `specs/0091` criterion 25: one percent per picture; the lines decided, then one trace.
          const stale = () => mine !== generation.current;
          const text = readsText(v) && !ocrFailed.current.has(id);
          const job = new JobProgress([...(text && session.trace_ocr_lines(id) === "" ? (["read"] as const) : []), ...(text ? (["fit"] as const) : []), "trace"], progress);
          const read = await textLines(session, id, v, job.stage("read"), full, stale);
          if (stale()) {
            return;
          }
          const done = await traceWithText(session, runner(), id, full, settings, read, fitInputs(), job, stale);
          if (done === null) {
            return;
          }
          session.set_trace_result(id, done.out.wire);
          results.current.set(id, wanted);
        }
        // `specs/0091` decision 4: the read lines fitted to the faces (done by the preview
        // already, unless a value changed since); then the texts go with the paths in one step.
        if (readsText(v) && !session.trace_text_fitted() && !(await fitTexts(session, () => mine !== generation.current))) {
          return;
        }
        const outcome = session.create_traced_paths();
        // No notice after creating paths: the new group being selected is the feedback (decided by
        // the customer, 2026-10-31).
        outcome.free();
        // Only a warning when letters are guessed; none when all are sure (`specs/0091`
        // criterion 5).
        const warning = session.trace_created_warning();
        if (warning !== "") {
          textOptionsRef.current?.onNotice(warning);
        }
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
  }, [getSession, creating, stopped, failure, retrace, status.canCreate, startTimers, clearTimers, thresholdOf, onChange, returnFocus, fail, setPercent, textLines, fitTexts, fitInputs, runner, joinedJob, resolveParts]);

  // Leaving the tool ends every job.
  useEffect(() => {
    if (!active) {
      cancelAll();
      setCreating(false);
      setStopped(false);
      setFailure(null);
      setLoading(false);
      setReading(false);
      ocrFailed.current.clear();
      partAuto.current.clear();
      regionReads.current.clear();
      setRemoved(null);
    }
  }, [active, cancelAll]);



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

  /** Takes `hex` out of the picked colours: a settings change, not a history step; the preview
   * traces again after the debounce (`ux-notes.md` §25.4). */
  const removePicked = useCallback(
    (hex: string) => {
      const current = choiceRef.current;
      const swatches = current.values.swatches.filter((s) => s !== hex);
      if (swatches.length === current.values.swatches.length) {
        return;
      }
      dragging.current = false;
      setRemoved({ hex, left: swatches.join() });
      apply({ ...current, values: { ...current.values, swatches } });
    },
    [apply],
  );

  /** The Mask item (`specs/0094` UX notes §1): the session's mode; the view's region changes, so
   * the sync above traces again. */
  const setMaskOutside = useCallback(
    (outside: boolean) => {
      getSession()?.set_trace_mask_outside(outside);
      onChange();
    },
    [getSession, onChange],
  );

  /** Pick mask paths (`specs/0094` slice 1b): the session's mode; a pick changes the selection,
   * so the sync above traces again. */
  const setMaskPick = useCallback(
    (armed: boolean) => {
      getSession()?.set_trace_mask_pick(armed);
      onChange();
    },
    [getSession, onChange],
  );
  const clearMask = useCallback(() => {
    getSession()?.clear_trace_mask();
    onChange();
  }, [getSession, onChange]);

  /** Disarms the armed pipette, Escape's last step and a right press on the canvas (`ux-notes.md`
   * §25.4); `false` when it was not armed. */
  const disarmPick = useCallback((): boolean => {
    const session = active ? getSession() : null;
    if (!session || session.trace_pick_mode() === "") {
      return false;
    }
    session.set_trace_pick("");
    onChange();
    return true;
  }, [active, getSession, onChange]);

  const keyDown = useCallback(
    (event: KeyboardEvent): boolean => {
      if (!active) {
        return false;
      }
      // Armed Pick mask paths ends first, before a running trace stops (`specs/0094` criterion 39).
      const session = getSession();
      if (event.key === "Escape" && session?.trace_mask_pick()) {
        session.set_trace_mask_pick(false);
        onChange();
        return true;
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
      if (event.key === "Escape") {
        return disarmPick();
      }
      return false;
    },
    [active, create, creating, stop, getSession, onChange, disarmPick],
  );



  // The progress shows only after 400 ms and then holds (customer 2026-10-30); loading counts as
  // a job that has not reported yet.
  const slot: TraceSlot =
    failure !== null
      ? { kind: "failed", text: failure }
      : stopped
        ? { kind: "stopped" }
        : busy
          ? { kind: "running", percent: loading ? null : percent, label: reading ? READING_TEXT : undefined }
          : { kind: "none" };
  const recognised = reading
    ? READING_TEXT
    : [textLinesText(...textCounts), shapesText(shapeCount)].filter((t) => t !== "").join(" · ");
  const session = active ? getSession() : null;
  // The Mask group is always drawn (`ux-notes.md` §24.4): the mode with or without mask paths.
  const maskOutside = session?.trace_mask_outside() ?? false;
  const mask = { paths: view.selection === "images" ? view.maskPaths : 0, outside: maskOutside };
  const parts = partsApi.parts;
  // `specs/0094` UX notes §7: with two or more parts "3 parts" replaces the mask segment, and an
  // active later part without paths says so last.
  const pickMode = session?.trace_pick_mode() ?? "";
  const maskPick = session?.trace_mask_pick() ?? false;
  const maskRefused = session?.trace_mask_pick_refused() ?? false;
  // The bar draws no notice (`ux-notes.md` §27): the hidden region says them as they were worded.
  const notices = [
    imagesLabel(progress, view.images.length),
    status.line,
    status.message,
    [status.muted, unclipped].filter((t) => t !== "").join(" "),
    palette.sameNote,
    maskRefused ? TRACE_MIXED : "",
    // Pick colours says its own reason last (`picked`).
    active && values.method !== PICK_COLOURS ? nothingToTrace(values) : "",
  ];
  const nextAnnouncement = traceAnnouncement({
    analysis: analysisLine(analysisState),
    mask: parts.length > 1 ? partsCount(parts.length) : maskText(mask.outside, mask.paths),
    counts: status.counts,
    countsMuted: status.countsMuted,
    notices,
    recognised,
    palette: palette.note,
    emptyPart: active ? partEmptyText(parts) : "",
    picked: active && values.method === PICK_COLOURS ? pickedAnnouncement(removed !== null && removed.left === values.swatches.join() ? removed.hex : null, nothingToTrace(values) !== "") : "",
  });
  // Announced once the result settles (`ux-notes.md` §24.5): not while a value is dragged, and not
  // for a reduced preview whose full-size result follows; each new result renders again.
  const [announcement, setAnnouncement] = useState("");
  useEffect(() => {
    if (!dragging.current && !fullFollows.current) {
      setAnnouncement(nextAnnouncement);
    }
  }, [nextAnnouncement, status, busy]);
  // A disabled Trace button states why (`ux-notes.md` §22.2): nothing to trace, or a result that
  // has no paths.
  const traceBlocked = slot.kind !== "none" ? "" : nothingToTrace(values) || (status.canCreate ? "" : status.line || status.message);
  const shownId = view.shown;
  // With two or more parts the active part's own background (region-only analysis per part).
  const activePart = parts.length > 1 ? parts.find((p) => p.active) : undefined;
  const partBackground = activePart ? partAuto.current.get(activePart.id)?.background : undefined;
  const suggestedBackground = partBackground ?? (shownId ? (backgrounds.current.get(shownId) ?? null) : null);
  return {
    active,
    available,
    selection: view.selection,
    mask,
    setMaskOutside,
    maskPick,
    setMaskPick,
    clearMask,
    parts,
    selectPart: partsApi.selectPart,
    addPart: partsApi.addPart,
    removePart: partsApi.removePart,
    params,
    presets,
    values,
    choice,
    presetLabel: presetLabel(choice, presets),
    advancedOpen,
    countsStale: busy || stopped || creating,
    slot,
    retrace,
    creating,
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
    removePicked,
    disarmPick,
    suggestedBackground,
    analyse,
    announcement,
    traceBlocked,
    palette,
  };
}

/** The colours and joined regions of the shown multicolour preview, `null` for one colour. */
function colourCounts(session: WasmSession): { colours: number; joined: number } | null {
  const info = readPalette(session.trace_palette());
  return info ? { colours: info.layers.length, joined: info.joined } : null;
}
