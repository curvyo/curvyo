import { useCallback, useEffect, useMemo, useRef, useState } from "react";

import type { WasmSession } from "@/lib/editorSession";
import { type CentreOptionsWire, CentreLineJobs, createCentreLineJobs } from "@/lib/centreLineJobs";
import {
  CENTRE_LINES_DEFAULTS,
  CENTRE_UPDATING,
  type CentreLinesOptions,
  type CentreLinesResult,
  centreLinesNotice,
  centrePreviewCounts,
} from "@/lib/centreLinesText";

/** The notice of the last press or Apply, keyed so a repeated text is announced again. */
export interface CentreLinesBarNotice {
  id: number;
  text: string;
  alert: boolean;
}

/** The shown preview's counts. */
export interface CentrePreview {
  paths: number;
  nodes: number;
  kept: number;
}

export interface CentreLinesApi {
  /** Whether the options bar is open. */
  open: boolean;
  /** The options shown, remembered for the session. */
  options: CentreLinesOptions;
  /** A value changed (a drag step, an arrow, a typed value, the switch): a new preview. */
  setOptions: (options: CentreLinesOptions) => void;
  /** The press of the button or menu item: opens the bar, or shows the refusal. */
  press: () => void;
  /** Apply: commits the preview's result in one undo step "Find centre lines". */
  apply: () => void;
  /** Cancel or Escape: closes the bar without a write. */
  cancel: () => void;
  /** The x or Escape during Apply: stops it, nothing is written, the bar stays. */
  stop: () => void;
  notice: CentreLinesBarNotice | null;
  /** Drops the notice (an Undo or Redo takes the step it speaks of back). */
  clearNotice: () => void;
  /** Why a press would be refused now (`"text"`, `"image"`, `"not_filled"`, `"too_large"`), or
   * `"none"`; read for the dimmed tooltip. */
  refusalKind: () => string;
  /** The shown preview's counts, `null` before the first. */
  preview: CentrePreview | null;
  /** A run took longer than 400 ms: the old preview is dimmed and Apply shows the running look. */
  running: boolean;
  /** Apply has run longer than the threshold: the running look shows (UX notes §4). */
  slow: boolean;
  /** Apply is waiting or writing: the fields are inert; the share done, or `null` without one. */
  applying: boolean;
  progress: number | null;
  /** The polite announcement (UX notes §4). */
  announcement: string;
  /** Re-reads the session after any change: a press on empty canvas, a tool change or an undo
   * closes the bar; a changed document or selection sends the subjects again. */
  refresh: () => void;
}

/** A run longer than this shows the running look (UX notes §4). */
const RUNNING_AFTER_MS = 400;
/** The running look stays at least this long once shown. */
const RUNNING_HOLD_MS = 300;

function read(raw: {
  kind: string;
  converted: number;
  kept: number;
  skipped: number;
  unlinked: number;
  nodes: number;
  free: () => void;
}): CentreLinesResult {
  const result = {
    kind: raw.kind,
    converted: raw.converted,
    kept: raw.kept,
    skipped: raw.skipped,
    unlinked: raw.unlinked,
    nodes: raw.nodes,
  };
  raw.free();
  return result;
}

/** The options as the worker reads them. */
function wireOf(options: CentreLinesOptions): CentreOptionsWire {
  return [options.overlap / 100, options.thickParts, options.optimise / 100, options.keepInner ? 1 : 0, options.offset];
}

/**
 * Find centre lines in the frontend (`specs/0117-centre-lines` criteria 15 and 15b, ADR 0063
 * amendment 2026-11-18): the bar's open state and remembered options, the worker's jobs, the
 * preview's counts, the sliced Apply and the notices. Every decision is the session's.
 */
export function useCentreLines(
  getSession: () => WasmSession | null,
  onChange: () => void,
  createJobs: (listener: ConstructorParameters<typeof CentreLineJobs>[2]) => CentreLineJobs = createCentreLineJobs,
): CentreLinesApi {
  const [open, setOpen] = useState(false);
  const [options, setOptionsState] = useState<CentreLinesOptions>(CENTRE_LINES_DEFAULTS);
  const [notice, setNotice] = useState<CentreLinesBarNotice | null>(null);
  const [preview, setPreview] = useState<CentrePreview | null>(null);
  const [running, setRunning] = useState(false);
  const [applying, setApplying] = useState(false);
  const [slow, setSlow] = useState(false);
  const slowTimer = useRef<number | null>(null);
  const previewRef = useRef<CentrePreview | null>(null);
  const [progress, setProgress] = useState<number | null>(null);
  const [announcement, setAnnouncement] = useState("");
  const counter = useRef(0);
  const jobs = useRef<CentreLineJobs | null>(null);
  const openRef = useRef(false);
  const optionsRef = useRef(options);
  const runningTimer = useRef<number | null>(null);
  const runningSince = useRef<number | null>(null);
  /** When each generation was asked for, for the measured latency. */
  const asked = useRef(new Map<number, number>());
  /** Bumped by Cancel: a running Apply stops at its next slice. */
  const applyEpoch = useRef(0);

  useEffect(() => {
    if (!notice) {
      return undefined;
    }
    const timer = window.setTimeout(() => setNotice(null), notice.alert ? 8000 : 5000);
    return () => window.clearTimeout(timer);
  }, [notice]);

  const endSlow = useCallback(() => {
    if (slowTimer.current !== null) {
      window.clearTimeout(slowTimer.current);
      slowTimer.current = null;
    }
    setSlow(false);
  }, []);

  const show = useCallback((result: CentreLinesResult, paths?: number) => {
    const next = centreLinesNotice(result, paths);
    if (next) {
      counter.current += 1;
      setNotice({ ...next, id: counter.current });
    }
  }, []);

  const stopRunning = useCallback(() => {
    if (runningTimer.current !== null) {
      window.clearTimeout(runningTimer.current);
      runningTimer.current = null;
    }
    const since = runningSince.current;
    const end = () => {
      runningSince.current = null;
      setRunning(false);
      getSession()?.centre_lines_set_stale(false);
    };
    if (since === null) {
      end();
    } else {
      window.setTimeout(end, Math.max(0, RUNNING_HOLD_MS - (performance.now() - since)));
    }
  }, [getSession]);

  /** Ends the bar: the worker goes, the session shows the document again. */
  const close = useCallback(() => {
    openRef.current = false;
    applyEpoch.current += 1;
    jobs.current?.close();
    jobs.current = null;
    getSession()?.centre_lines_close();
    if (runningTimer.current !== null) {
      window.clearTimeout(runningTimer.current);
      runningTimer.current = null;
    }
    runningSince.current = null;
    setRunning(false);
    setApplying(false);
    endSlow();
    setProgress(null);
    previewRef.current = null;
    setPreview(null);
    setOpen(false);
  }, [endSlow, getSession]);

  /** Asks the worker for the preview of `next`, with the subjects when given. */
  const ask = useCallback(
    (next: CentreLinesOptions, subjects: Float64Array | null) => {
      const runner = jobs.current;
      if (!runner) {
        return;
      }
      const gen = runner.request(wireOf(next), subjects);
      asked.current.set(gen, performance.now());
      setAnnouncement(CENTRE_UPDATING);
      if (runningTimer.current === null && runningSince.current === null) {
        runningTimer.current = window.setTimeout(() => {
          runningTimer.current = null;
          if (jobs.current?.busy) {
            runningSince.current = performance.now();
            setRunning(true);
            getSession()?.centre_lines_set_stale(true);
          }
        }, RUNNING_AFTER_MS);
      }
    },
    [getSession],
  );

  /** Opens (or opens again) the session's bar and sends its subjects; the refusal otherwise. */
  const start = useCallback((): boolean => {
    const session = getSession();
    if (!session) {
      return false;
    }
    const opened = session.centre_lines_open();
    const { kind, nodes } = opened;
    const wire = new Float64Array(opened.wire);
    opened.free();
    if (kind !== "open") {
      close();
      show({ kind, nodes, converted: 0, kept: 0, skipped: 0, unlinked: 0 });
      return false;
    }
    jobs.current ??= createJobs({
      preview: (gen, previewWire, ms) => {
        const current = getSession();
        if (!current || !openRef.current) {
          return;
        }
        const view = current.centre_lines_show(previewWire);
        const counts = { paths: view.paths, nodes: view.nodes, kept: view.kept };
        const shown = view.shown;
        view.free();
        const since = asked.current.get(gen);
        asked.current.clear();
        if (since !== undefined) {
          // The latency of a value change, read by the browser checks (`0117` criterion 18).
          performance.measure("centre-preview", { start: since, end: performance.now(), detail: { worker: ms } });
        }
        if (shown) {
          previewRef.current = counts;
          setPreview(counts);
          setAnnouncement(centrePreviewCounts(counts.paths, counts.nodes, counts.kept));
        }
        stopRunning();
      },
      progress: (percent) => setProgress(percent / 100),
      failed: () => {
        close();
        // The worker failed: say so, not a refusal of the selection.
        show({ kind: "failed", nodes: 0, converted: 0, kept: 0, skipped: 0, unlinked: 0 });
      },
    });
    openRef.current = true;
    setOpen(true);
    ask(optionsRef.current, wire);
    return true;
  }, [ask, close, createJobs, getSession, show, stopRunning]);

  const press = useCallback(() => {
    const session = getSession();
    if (!session) {
      return;
    }
    const refusal = read(session.centre_lines_refusal());
    if (refusal.kind !== "none") {
      show(refusal);
      return;
    }
    // The Select tool's menu item: the bar lives in the Node tool (UX notes §1).
    if (session.tool() !== "node") {
      session.set_tool("node");
    }
    setNotice(null);
    start();
    onChange();
  }, [getSession, onChange, show, start]);

  const setOptions = useCallback(
    (next: CentreLinesOptions) => {
      optionsRef.current = next;
      setOptionsState(next);
      if (openRef.current) {
        ask(next, null);
      }
    },
    [ask],
  );

  const apply = useCallback(() => {
    const runner = jobs.current;
    if (!runner || !openRef.current) {
      return;
    }
    const epoch = applyEpoch.current;
    // The paths the preview showed are the ones Apply writes (the notice and the tooltip agree).
    const paths = previewRef.current?.paths;
    setApplying(true);
    setProgress(null);
    endSlow();
    slowTimer.current = window.setTimeout(() => {
      slowTimer.current = null;
      setSlow(true);
    }, RUNNING_AFTER_MS);
    const finish = (result: CentreLinesResult | null) => {
      if (applyEpoch.current !== epoch) {
        return;
      }
      close();
      if (result) {
        show(result, paths);
      }
      onChange();
    };
    void runner.result().then((wire) => {
      const session = getSession();
      if (applyEpoch.current !== epoch || !session) {
        return;
      }
      if (!wire) {
        finish(null);
        return;
      }
      // The document or the selection changed (at the start or between two slices): nothing
      // was written, the subjects go again and the bar stays.
      const again = () => {
        endSlow();
        setApplying(false);
        setProgress(null);
        start();
      };
      const begin = session.centre_lines_apply_begin(wire);
      const kind = begin.kind;
      const outcome = read(begin.outcome);
      begin.free();
      if (kind === "stale") {
        again();
        return;
      }
      if (kind !== "pending") {
        finish(outcome);
        return;
      }
      // One slice per task, so the canvas keeps drawing between them (criterion 18).
      const step = () => {
        const current = getSession();
        if (applyEpoch.current !== epoch || !current) {
          return;
        }
        const view = current.centre_lines_apply_step();
        const stepKind = view.kind;
        const share = view.progress;
        const done = read(view.outcome);
        view.free();
        if (stepKind === "pending") {
          setProgress(share);
          window.setTimeout(step, 0);
          return;
        }
        if (stepKind === "stale") {
          again();
          return;
        }
        finish(done);
      };
      window.setTimeout(step, 0);
    });
  }, [close, endSlow, getSession, onChange, show, start]);

  const cancel = useCallback(() => {
    close();
    onChange();
  }, [close, onChange]);

  const stop = useCallback(() => {
    applyEpoch.current += 1;
    setApplying(false);
    endSlow();
    setProgress(null);
  }, [endSlow]);

  const clearNotice = useCallback(() => setNotice(null), []);

  const refresh = useCallback(() => {
    const session = getSession();
    if (!openRef.current || !session) {
      return;
    }
    if (!session.centre_lines_is_open()) {
      close();
      return;
    }
    if (!applying && !session.centre_lines_current()) {
      start();
    }
  }, [applying, close, getSession, start]);

  useEffect(() => () => jobs.current?.close(), []);

  const refusalKind = useCallback(() => {
    const session = getSession();
    return session ? read(session.centre_lines_refusal()).kind : "none";
  }, [getSession]);

  return useMemo(
    () => ({
      open,
      options,
      setOptions,
      press,
      apply,
      cancel,
      stop,
      notice,
      clearNotice,
      refusalKind,
      preview,
      running,
      slow,
      applying,
      progress,
      announcement,
      refresh,
    }),
    [open, options, setOptions, press, apply, cancel, stop, notice, clearNotice, refusalKind, preview, running, slow, applying, progress, announcement, refresh],
  );
}
