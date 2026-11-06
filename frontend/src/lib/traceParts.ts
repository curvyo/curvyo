/**
 * The parts of a composite trace (`specs/0094-masked-composite-trace` slice 2, UX notes §3 to §7):
 * the chips the session reports, their names, and the parts' jobs traced one by one, each part's
 * result cached under its own key, joined by the session. No React, so node tests can check it
 * with a stand-in worker and session.
 */

import type { TraceOutput, TracePixels } from "./traceJobs.ts";
import type { TraceValues } from "./traceText.ts";

/** At most this many parts (spec, Proposals). */
export const PARTS_MAX = 8;

export const PARTS = "Parts";
export const ADD_PART = "Add part";
export const PARTS_AT_MOST = "8 parts at most";
export const KEEP_ONE_PART = "Keep at least one part";
/** "0 paths: no part has anything to trace." (UX notes §3). */
export const NO_PART_LINE = "0 paths: no part has anything to trace.";

/** One chip as the session reports it. */
export interface TracePartInfo {
  /** Stable across renumbering: the part's settings are kept under it. */
  id: number;
  paths: number;
  outside: boolean;
  /** The first part without mask paths: the whole image. */
  whole: boolean;
  /** A later part without mask paths: it adds nothing. */
  muted: boolean;
  active: boolean;
  /** Its clip keeps some of the image. */
  contributes: boolean;
  /** Its clip for the worker's settings, empty for the whole image or nothing. */
  mask: string;
}

/** The session's chips JSON; one whole-image part when it cannot be read. */
export function readParts(json: string): TracePartInfo[] {
  try {
    const parsed = JSON.parse(json) as TracePartInfo[];
    if (Array.isArray(parsed) && parsed.length > 0) {
      return parsed;
    }
  } catch {
    // An older module or no session: one part.
  }
  return [{ id: 0, paths: 0, outside: false, whole: true, muted: false, active: true, contributes: true, mask: "" }];
}

function pathsText(paths: number): string {
  return paths === 1 ? "1 path" : `${paths} paths`;
}

/** A chip's accessible name (criterion 32): "Part 2 of 3, inside, 2 paths, active". */
export function partName(index: number, count: number, part: TracePartInfo): string {
  const mode = part.whole ? "whole image" : part.paths === 0 ? "no paths" : `${part.outside ? "outside" : "inside"}, ${pathsText(part.paths)}`;
  return [`Part ${index + 1} of ${count}`, mode, ...(part.active ? ["active"] : [])].join(", ");
}

/** A chip's tooltip: its name without "active". */
export function partTooltip(index: number, count: number, part: TracePartInfo): string {
  return partName(index, count, { ...part, active: false });
}

/** The bar's muted label with two or more parts: "Part 2 of 3"; empty with one. */
export function partsLabel(parts: readonly TracePartInfo[]): string {
  const at = parts.findIndex((p) => p.active);
  return parts.length < 2 || at < 0 ? "" : `Part ${at + 1} of ${parts.length}`;
}

/** "3 parts", or empty with one (the status segment and the expander's description). */
export function partsCount(count: number): string {
  return count < 2 ? "" : `${count} parts`;
}

/** The status line's muted segment for an active later part without paths: "Part 3 has no paths". */
export function partEmptyText(parts: readonly TracePartInfo[]): string {
  const at = parts.findIndex((p) => p.active);
  return parts.length < 2 || at < 0 || !parts[at].muted ? "" : `Part ${at + 1} has no paths`;
}

/** The Trace button's tooltip with two or more parts: "Create paths (Enter), 3 parts joined". */
export function joinTooltip(count: number): string {
  return count < 2 ? "" : `Create paths (Enter), ${count} parts joined`;
}

/** The trash's name: what its next press does (UX notes §5). */
export function removePartText(index: number, armed: boolean): string {
  return armed ? `Press again to remove part ${index + 1}` : `Remove part ${index + 1}`;
}

/** One part's job: its settings (the clip inside), whether it reads text, whether it may be
 * traced from the crop of its clip, and its cache key. */
export interface PartJob {
  settings: string;
  text: boolean;
  key: string;
  /** No text and no Drop border shapes: both read the whole picture (criterion 31). */
  crop: boolean;
}

/** Whether a part with `values` may be traced from the cut-out of its clip (criterion 31): not
 * with Recognise text (the lines are in the picture's pixels), Drop border shapes (the cut's edge
 * is no border) or Denoise (a shape across the cut is fitted otherwise, tester 2026-11-06). */
export function mayCrop(values: TraceValues, text: boolean): boolean {
  return !text && !values.ignoreBackground && values.denoise === 0;
}

/** The cache key of a part's job: the settings, whether text is read, and the picture size. */
export function partKey(settings: string, text: boolean, reduced: boolean): string {
  return `${reduced ? "reduced" : "full"}|${text ? "text" : ""}|${settings}`;
}

/** The session calls the parts' jobs need. */
export interface PartsSession {
  /** Part `index`'s pixels; cut to its clip when `crop`. */
  pixels(index: number, crop: boolean): TracePixels | null;
  /** The parts' wires joined (null: the part adds nothing); the joined header numbers. */
  join(wires: (Float64Array | null)[]): Float64Array;
}

/** The worker job the parts need. */
export interface PartsRunner {
  trace(pixels: TracePixels, settings: string, onProgress?: (percent: number) => void, lines?: string): Promise<TraceOutput>;
}

/** The kept results of the parts, by key; bounded so a long session does not hold every one. */
export class PartCache {
  private readonly wires = new Map<string, Float64Array>();
  private readonly limit: number;

  constructor(limit = 4 * PARTS_MAX) {
    this.limit = limit;
  }

  get(key: string): Float64Array | undefined {
    return this.wires.get(key);
  }

  set(key: string, wire: Float64Array): void {
    this.wires.delete(key);
    this.wires.set(key, wire);
    while (this.wires.size > this.limit) {
      const oldest = this.wires.keys().next().value;
      if (oldest === undefined) {
        break;
      }
      this.wires.delete(oldest);
    }
  }

  clear(): void {
    this.wires.clear();
  }
}

/**
 * The parts' jobs (`null`: the part adds nothing) traced one by one on the one worker, a part
 * whose key is kept taken from `cache` (criterion 25); one percent over the parts that are traced,
 * never running backwards (criterion 26); `lines` (the read OCR lines) go to the parts that read
 * text; a job with `crop` is traced from the crop of its clip (criterion 31). The
 * joined header and the seconds the traces took, or `null` when a newer job took over.
 */
export async function traceParts(
  session: PartsSession,
  runner: PartsRunner,
  jobs: readonly (PartJob | null)[],
  cache: PartCache,
  lines: string,
  onProgress: (percent: number) => void,
  stale: () => boolean,
): Promise<{ header: Float64Array; seconds: number; missing: boolean } | null> {
  const todo = jobs.filter((job) => job !== null && cache.get(job.key) === undefined).length;
  let done = 0;
  let last = -1;
  const report = (percent: number) => {
    const whole = Math.min(100, Math.floor((100 * done + Math.min(100, Math.max(0, percent))) / Math.max(1, todo) + 1e-9));
    if (whole > last) {
      last = whole;
      onProgress(whole);
    }
  };
  const wires: (Float64Array | null)[] = [];
  let seconds = 0;
  for (const [index, job] of jobs.entries()) {
    if (job === null) {
      wires.push(null);
      continue;
    }
    const kept = cache.get(job.key);
    if (kept !== undefined) {
      wires.push(kept);
      continue;
    }
    const pixels = session.pixels(index, job.crop);
    if (!pixels) {
      return { header: new Float64Array(0), seconds, missing: true };
    }
    const out = await runner.trace(pixels, job.settings, report, job.text ? lines : "");
    if (stale()) {
      return null;
    }
    cache.set(job.key, out.wire);
    wires.push(out.wire);
    seconds += out.seconds;
    done += 1;
    report(0);
  }
  report(100);
  return { header: session.join(wires), seconds, missing: false };
}
