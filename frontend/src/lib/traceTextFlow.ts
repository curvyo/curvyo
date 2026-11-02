/**
 * The order of a Trace job with text (`specs/0091-trace-text-recognition` criterion 25): the read
 * lines' own traces and the font fit decide the lines first, then the picture is traced once
 * with them; one percent from 0 to 100 runs over the whole job. No React, so node tests can check
 * it with a stand-in worker and session.
 */

import type { FitFace, TraceOutput, TracePixels } from "@/lib/traceJobs";

/** The stages a Trace job can have, in order. */
export type TraceStage = "read" | "fit" | "reduced" | "trace";

/** Each stage's share of the job's percent (Proposal, from the measured times: the reading takes
 * longest, the reduced preview is quick). Absent stages give their share to the others. */
const STAGE_WEIGHT: Record<TraceStage, number> = { read: 45, fit: 25, reduced: 5, trace: 25 };

/** One percent over a job of several stages: each stage's own 0 to 100 maps into its span, and
 * only a higher whole percent is reported, so it never runs backwards. */
export class JobProgress {
  private readonly spans = new Map<TraceStage, [number, number]>();
  private readonly report: (percent: number) => void;
  private last = -1;

  constructor(stages: TraceStage[], report: (percent: number) => void) {
    this.report = report;
    const total = stages.reduce((sum, stage) => sum + STAGE_WEIGHT[stage], 0) || 1;
    let start = 0;
    for (const stage of stages) {
      const width = (100 * STAGE_WEIGHT[stage]) / total;
      this.spans.set(stage, [start, width]);
      start += width;
    }
  }

  /** The reporter of `stage`: hears that stage's own percent. A stage not in the job reports
   * nothing. */
  stage(stage: TraceStage): (percent: number) => void {
    return (percent: number) => {
      const span = this.spans.get(stage);
      if (!span) {
        return;
      }
      // A hair over the exact value: the last stage ends at 100, not 99.999.
      const whole = Math.floor(span[0] + (span[1] * Math.min(100, Math.max(0, percent))) / 100 + 1e-9);
      if (whole > this.last) {
        this.last = whole;
        this.report(whole);
      }
    };
  }
}

/** The session calls the flow needs. */
export interface TextFlowSession {
  trace_fit_request_for(id: string, regions: Float64Array, tolerance: number, favourites: string): string;
  set_trace_fit(request: string, result: string): boolean;
  trace_ocr_lines(id: string): string;
}

/** The worker jobs the flow needs. */
export interface TextFlowRunner {
  regions(pixels: TracePixels, settings: string, lines: string): Promise<Float64Array>;
  fit(request: string, loadFaces: (faces: [string, number][]) => Promise<FitFace[]>, onProgress?: (percent: number) => void): Promise<string>;
  trace(pixels: TracePixels, settings: string, onProgress?: (percent: number) => void, lines?: string): Promise<TraceOutput>;
}

/** What the fit needs from the page: the tolerance, the favourites JSON and the font files. */
export interface FitInputs {
  tolerance: number;
  favourites: string;
  loadFaces: (faces: [string, number][]) => Promise<FitFace[]>;
}

/** Decides the read `lines` of image `id` before its trace (criterion 25): the text pass alone,
 * the fit of its lines (its percent to `onProgress`), and the lines the trace then takes (those
 * the fit set no text for are traced as shapes). The pixels go to the worker and come back.
 * `null` when a newer job took over (`stale`). */
export async function decideTextLines(
  session: TextFlowSession,
  runner: TextFlowRunner,
  id: string,
  pixels: TracePixels,
  settings: string,
  lines: string,
  fit: FitInputs,
  onProgress: (percent: number) => void,
  stale: () => boolean,
): Promise<string | null> {
  if (lines === "") {
    return "";
  }
  const regions = await runner.regions(pixels, settings, lines);
  if (stale()) {
    return null;
  }
  const request = session.trace_fit_request_for(id, regions, fit.tolerance, fit.favourites);
  if (request !== "") {
    const result = await runner.fit(request, fit.loadFaces, onProgress);
    if (stale()) {
      return null;
    }
    session.set_trace_fit(request, result);
  }
  onProgress(100);
  return session.trace_ocr_lines(id);
}

/** The whole job of one picture with read `lines` (criterion 25): decided first, then traced
 * once with the decided lines; the percent over both stages to `progress`. `null` when a newer
 * job took over. */
export async function traceWithText(
  session: TextFlowSession,
  runner: TextFlowRunner,
  id: string,
  pixels: TracePixels,
  settings: string,
  lines: string,
  fit: FitInputs,
  progress: JobProgress,
  stale: () => boolean,
): Promise<{ out: TraceOutput; lines: string } | null> {
  const decided = await decideTextLines(session, runner, id, pixels, settings, lines, fit, progress.stage("fit"), stale);
  if (decided === null) {
    return null;
  }
  const out = await runner.trace(pixels, settings, progress.stage("trace"), decided);
  return stale() ? null : { out, lines: decided };
}
