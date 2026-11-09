/** What a new mask region makes stale of an image's automatic values and how its analysis applies
 * (`specs/0094` criterion 9). */

import { type TraceAnalysis, applyAnalysis } from "./traceAnalysis.ts";
import type { TraceValues } from "./traceText.ts";

/** The per-image caches of the Trace tool that hold automatic values. */
export interface AutomaticValues {
  thresholds: Map<string, number>;
  backgrounds: Map<string, string>;
  analyses: Map<string, unknown>;
}

/** Forgets the automatic values of image `id` after its mask region changed, so the next trace
 * reads the threshold, the background and the analysis of the region's pixels only; a Threshold
 * the maker set (`makerThreshold`) stays. */
export function forgetRegionValues(id: string, caches: AutomaticValues, makerThreshold: ReadonlySet<string>): void {
  caches.backgrounds.delete(id);
  caches.analyses.delete(id);
  if (!makerThreshold.has(id)) {
    caches.thresholds.delete(id);
  }
}

/** The values one choice of Method sets together: the Method and Remove background's colour. */
const METHOD_FIELDS = ["method", "backgroundAuto", "background", "backgroundTolerance"] as const;

/** The analysis of a new mask region applied to `values` (`specs/0094` criterion 9, tester
 * 2026-11-05): what the analysis filled in follows the region, and every analysed value the maker
 * set (`maker`: the Threshold, or the Method with its colour) stays as the maker set it. */
export function applyRegionAnalysis(values: TraceValues, analysis: TraceAnalysis, maker: ReadonlySet<keyof TraceValues>): TraceValues {
  const read = applyAnalysis(values, analysis);
  const kept: Partial<TraceValues> = {};
  if (maker.has("threshold")) {
    kept.threshold = values.threshold;
  }
  if (METHOD_FIELDS.some((f) => maker.has(f))) {
    Object.assign(kept, Object.fromEntries(METHOD_FIELDS.map((f) => [f, values[f]])));
  }
  return { ...read, ...kept };
}

/** The automatic values read from one part's region (`specs/0094` slice 2). */
export interface RegionDefaults {
  threshold: number;
  /** `#RRGGBB`, empty for a transparent background. */
  background: string;
  analysis: TraceAnalysis | null;
}

/** A part's values after its region's automatic values were read (`specs/0094` slice 2, coordinator
 * 2026-11-06): the analysis applies as for a new mask, the automatic Threshold unless the maker set
 * one for the part, and Remove background's Auto takes the region's background. */
export function partAutoValues(values: TraceValues, defaults: RegionDefaults, maker: ReadonlySet<keyof TraceValues>): TraceValues {
  let next = defaults.analysis ? applyRegionAnalysis(values, defaults.analysis, maker) : values;
  if (!maker.has("threshold")) {
    next = { ...next, threshold: defaults.analysis?.threshold ?? defaults.threshold };
  }
  return next.backgroundAuto ? { ...next, background: defaults.background } : next;
}

/** What the worker reads from one region of an image: the automatic Threshold, the background
 * (`#RRGGBB`, empty for transparent) and the analysis JSON. */
export interface RegionRead {
  threshold: number;
  background: string;
  analysis: string;
}

/** The automatic values of image `id` in mask region `mask` (`specs/0089` UX notes §25.2): a
 * region read before in this session answers from `cache` without a new read; a read that fails
 * or is cancelled is not kept. */
export async function readRegion(cache: Map<string, RegionRead>, id: string, mask: string, read: () => Promise<RegionRead>): Promise<RegionRead> {
  const key = `${id}\n${mask}`;
  const kept = cache.get(key);
  if (kept) {
    return kept;
  }
  const fresh = await read();
  cache.set(key, fresh);
  return fresh;
}

/** The values the bar shows right after the view changed (`specs/0089` UX notes §25.2): a new
 * region of the same image keeps them until its analysis answers; a new image starts from its kept
 * Threshold (`known`, or none yet) and its suggested background. */
export function viewValues(values: TraceValues, sameImage: boolean, known: number | undefined, background: string | undefined): TraceValues {
  if (sameImage) {
    return values;
  }
  return { ...values, threshold: known ?? null, background: values.backgroundAuto ? (background ?? null) : values.background };
}

/** The values once a new region's analysis answered (`specs/0089` UX notes §25.2): it applies
 * once, keeping what the maker set; a failed analysis (`null`) changes nothing. */
export function regionAnswer(values: TraceValues, analysis: TraceAnalysis | null, maker: ReadonlySet<keyof TraceValues>): TraceValues {
  return analysis ? applyRegionAnalysis(values, analysis, maker) : values;
}
