// `specs/0089-trace-quality-and-colour` slice F: the analysis in the frontend (criteria 31 to 33,
// UX notes §3): the worker's JSON read, the values applied to a new image, the line per state.

import { strict as assert } from "node:assert";
import { test } from "node:test";

import {
  ANALYSE,
  ANALYSED_FIELDS,
  ANALYSING,
  ANALYSIS_FAILED,
  TYPE_UNCLEAR,
  analysisLine,
  applyAnalysis,
  readAnalysis,
  type TraceAnalysis,
} from "../src/lib/traceAnalysis.ts";
import { BRIGHTNESS, MULTICOLOUR, PICK_COLOURS, REMOVE_BACKGROUND, firstChoice } from "../src/lib/traceText.ts";

const pale = JSON.stringify({
  type: "logo",
  capture: "digital",
  confidence: 0.95,
  low: false,
  method: 1,
  threshold: 211,
  background: "#FFFFFF",
  tolerance: 59,
  reasons: ["pale_colours"],
  properties: { colours: 3, noise: 0.2 },
});

function analysis(json: string): TraceAnalysis {
  const read = readAnalysis(json);
  assert.ok(read);
  return read;
}

test("the worker's JSON is read; empty or broken JSON is a failed analysis", () => {
  const a = analysis(pale);
  assert.equal(a.type, "logo");
  assert.equal(a.method, REMOVE_BACKGROUND);
  assert.equal(a.tolerance, 59);
  assert.deepEqual(a.reasons, ["pale_colours"]);
  assert.equal(a.properties.colours, 3);
  assert.equal(readAnalysis(""), null);
  assert.equal(readAnalysis("{"), null);
  assert.equal(readAnalysis(JSON.stringify({ type: "photo", capture: "digital", threshold: 1 })), null);
});

test("criterion 33: the analysis sets the method, its background and tolerance and the Threshold only", () => {
  const start = { ...firstChoice().values, smooth: 33, specks: 1.5, output: MULTICOLOUR, denoise: 2, invert: true, swatches: ["#123456"] };
  const applied = applyAnalysis(start, analysis(pale));
  assert.equal(applied.method, REMOVE_BACKGROUND);
  assert.equal(applied.threshold, 211);
  assert.equal(applied.backgroundAuto, true);
  assert.equal(applied.background, "#FFFFFF");
  assert.equal(applied.backgroundTolerance, 59);
  for (const key of Object.keys(start) as (keyof typeof start)[]) {
    if (!ANALYSED_FIELDS.includes(key)) {
      assert.deepEqual(applied[key], start[key], key);
    }
  }
  // Back to Brightness for an image that needs no colour method.
  const plain = analysis(JSON.stringify({ ...JSON.parse(pale), method: 0, tolerance: null, background: "", reasons: [] }));
  assert.equal(applyAnalysis(applied, plain).method, BRIGHTNESS);
  // Pick colours keeps the method (the picks are the maker's) and takes the Threshold.
  const picking = applyAnalysis({ ...start, method: PICK_COLOURS }, analysis(pale));
  assert.equal(picking.method, PICK_COLOURS);
  assert.equal(picking.threshold, 211);
});

test("UX notes §3: the line per state and when Analyse again is pressable", () => {
  const a = analysis(pale);
  assert.equal(analysisLine({ kind: "auto", analysis: a }), "Logo · pale colours kept");
  const scan = analysis(JSON.stringify({ ...JSON.parse(pale), type: "line_drawing", capture: "scan", method: 0, reasons: [] }));
  assert.equal(analysisLine({ kind: "auto", analysis: scan }), "Line drawing · scan · automatic Threshold");
  const photo = analysis(JSON.stringify({ ...JSON.parse(pale), type: "graphic", capture: "paper_photo", method: 0, reasons: ["shadow_left_out"] }));
  assert.equal(analysisLine({ kind: "auto", analysis: photo }), "Graphic · photo of paper · shadow left out");
  assert.equal(analysisLine({ kind: "custom", analysis: scan }), "Custom · Line drawing");
  const low = analysis(JSON.stringify({ ...JSON.parse(pale), low: true, reasons: ["low_confidence"] }));
  assert.equal(analysisLine({ kind: "auto", analysis: low }), TYPE_UNCLEAR);
  assert.equal(TYPE_UNCLEAR, "Type unclear · Brightness with automatic Threshold");
  assert.equal(analysisLine({ kind: "analysing" }), ANALYSING);
  assert.equal(analysisLine({ kind: "failed" }), ANALYSIS_FAILED);
  assert.equal(analysisLine({ kind: "none" }), "");
  // The button is always enabled and no longer says "again" (`ux-notes.md` §22.3, §22.5).
  assert.equal(ANALYSE, "Analyse");
});
