// `specs/0094` criterion 9 (tester 2026-11-04): a new mask region forgets the image's automatic
// Threshold, background and analysis, so they are read again from the region's pixels; a
// Threshold the maker set stays.

import { strict as assert } from "node:assert";
import { test } from "node:test";

import { readAnalysis, type TraceAnalysis } from "../src/lib/traceAnalysis.ts";
import { applyRegionAnalysis, forgetRegionValues } from "../src/lib/traceMaskCaches.ts";
import { BRIGHTNESS, REMOVE_BACKGROUND, firstChoice, type TraceValues } from "../src/lib/traceText.ts";

const caches = () => ({
  thresholds: new Map([["a", 120], ["b", 90]]),
  backgrounds: new Map([["a", "#FFFFFF"], ["b", "#D8D8D8"]]),
  analyses: new Map<string, unknown>([["a", { threshold: 120 }], ["b", null]]),
});

test("a new mask forgets the analysis, the background and the automatic Threshold of that image only", () => {
  const c = caches();
  forgetRegionValues("a", c, new Set());
  assert.equal(c.analyses.has("a"), false, "the analysis runs again on the region");
  assert.equal(c.backgrounds.has("a"), false);
  assert.equal(c.thresholds.has("a"), false);
  assert.equal(c.analyses.has("b") && c.backgrounds.has("b") && c.thresholds.has("b"), true, "other images keep theirs");
});

test("a Threshold the maker set stays across a new mask", () => {
  const c = caches();
  forgetRegionValues("a", c, new Set(["a"]));
  assert.equal(c.thresholds.get("a"), 120);
  assert.equal(c.analyses.has("a"), false);
});

/** A region's analysis: Brightness at Threshold 90 (the left half of a picture, black on white). */
function brightness(threshold = 90): TraceAnalysis {
  const read = readAnalysis(
    JSON.stringify({ type: "logo", capture: "digital", confidence: 0.9, low: false, method: 0, threshold, background: "", reasons: [], properties: {} }),
  );
  assert.ok(read);
  return read;
}

const start = (): TraceValues => ({
  ...firstChoice().values,
  method: REMOVE_BACKGROUND,
  backgroundAuto: false,
  background: "#D8D8D8",
  backgroundTolerance: 40,
  threshold: 33,
  smooth: 70,
  specks: 1.5,
  corners: 20,
  invert: true,
});

test("a Method the maker chose survives a new mask, with its colour and every other setting", () => {
  const maker = new Set<keyof TraceValues>(["method", "background"]);
  const read = applyRegionAnalysis(start(), brightness(), maker);
  assert.equal(read.method, REMOVE_BACKGROUND);
  assert.equal(read.background, "#D8D8D8");
  assert.equal(read.backgroundAuto, false);
  assert.equal(read.backgroundTolerance, 40);
  assert.deepEqual([read.smooth, read.specks, read.corners, read.invert], [70, 1.5, 20, true]);
  assert.equal(read.threshold, 90, "the Threshold was automatic: it follows the region");
});

test("an automatic Method follows the region's analysis", () => {
  const read = applyRegionAnalysis(start(), brightness(), new Set());
  assert.equal(read.method, BRIGHTNESS);
  assert.equal(read.threshold, 90);
});

test("a Threshold the maker set stays, an automatic Method still follows the region", () => {
  const read = applyRegionAnalysis(start(), brightness(), new Set<keyof TraceValues>(["threshold"]));
  assert.equal(read.threshold, 33);
  assert.equal(read.method, BRIGHTNESS);
});
