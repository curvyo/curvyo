// `specs/0025-raster-trace`: the Trace tool's words and plain rules (criteria 1, 3, 5, 6, 9, 11,
// 13, 15, 16, 46) and criterion 35: every notice of this slice is the same string in the code and
// in the user page `docs/tracing-images.md`.

import { strict as assert } from "node:assert";
import { readFileSync } from "node:fs";
import path from "node:path";
import { test } from "node:test";

import {
  CREATE_PATHS_TOOLTIP,
  LOADING_IMAGE,
  PREVIEW_OUT_OF_DATE,
  TOO_DETAILED,
  TRACE_EMPTY,
  TRACE_IMPORT,
  TRACE_NOT_IMAGE,
  TRACE_PARAMS_FALLBACK,
  TRACE_TOOLTIP,
  TRACING,
  clampValue,
  creatingLabel,
  imagesLabel,
  parseTyped,
  readSummary,
  readValues,
  specksHint,
  statusText,
  stepValue,
  tracedNotice,
  writeValues,
} from "../src/lib/traceText.ts";

const [threshold, smooth, specks] = TRACE_PARAMS_FALLBACK;
const keyOf = (key: string, mods: Partial<{ shiftKey: boolean; ctrlKey: boolean }> = {}) => ({
  key,
  shiftKey: false,
  ctrlKey: false,
  metaKey: false,
  ...mods,
});

test("criteria 5 and 6: steps, limits and clamping from the schema", () => {
  assert.equal(stepValue(threshold, 100, keyOf("ArrowUp")), 101);
  assert.equal(stepValue(threshold, 100, keyOf("ArrowDown", { shiftKey: true })), 90);
  assert.equal(stepValue(smooth, 95, keyOf("ArrowUp", { shiftKey: true })), 100);
  assert.equal(stepValue(specks, 0.5, keyOf("ArrowUp")), 0.6);
  assert.equal(stepValue(specks, 0.5, keyOf("ArrowUp", { shiftKey: true })), 1.5);
  assert.equal(stepValue(specks, 0.5, keyOf("ArrowDown", { ctrlKey: true })), 0.49);
  assert.equal(stepValue(threshold, 7, keyOf("Home")), 0);
  assert.equal(stepValue(threshold, 7, keyOf("End")), 255);
  assert.equal(stepValue(threshold, 7, keyOf("a")), null);
  assert.equal(parseTyped(threshold, "300"), 255);
  assert.equal(parseTyped(specks, "-2"), 0);
  assert.equal(parseTyped(specks, "0,25 mm"), 0.25);
  assert.equal(parseTyped(smooth, "x"), null);
  assert.equal(clampValue(smooth, 33.4), 33);
  assert.deepEqual(
    TRACE_PARAMS_FALLBACK.map((p) => [p.name, p.min, p.max, p.default]),
    [
      ["threshold", 0, 255, 128],
      ["smooth", 0, 100, 80],
      ["specks", 0, 10, 0.5],
    ],
  );
});

test("criterion 5: Smooth and Specks are an app setting; Threshold starts automatic", () => {
  const store = new Map<string, string>();
  const storage = { getItem: (k: string) => store.get(k) ?? null, setItem: (k: string, v: string) => void store.set(k, v) };
  assert.deepEqual(readValues(TRACE_PARAMS_FALLBACK, storage), { threshold: null, smooth: 80, specks: 0.5 });
  writeValues({ threshold: 90, smooth: 40, specks: 1.25 }, storage);
  assert.deepEqual(readValues(TRACE_PARAMS_FALLBACK, storage), { threshold: null, smooth: 40, specks: 1.25 });
  store.set("curvyo.trace-values", "{broken");
  assert.deepEqual(readValues(TRACE_PARAMS_FALLBACK, storage), { threshold: null, smooth: 80, specks: 0.5 });
});

test("criteria 11, 15, 16: the status line", () => {
  const wire = [0, 7, 412, 38, 5000, 0, 7];
  assert.deepEqual(statusText(readSummary(wire), 0.42, 100_000), {
    line: "7 paths, 412 nodes, 0.4 s",
    muted: "38 specks ignored",
    canCreate: true,
  });
  assert.equal(statusText(readSummary([0, 1, 4, 1, 10, 0, 1]), 0.1, 100).muted, "1 speck ignored");
  assert.deepEqual(statusText(readSummary([0, 0, 0, 0, 0, 0, 0]), 0.1, 100), {
    line: "0 paths at this Threshold: too low.",
    muted: "",
    canCreate: false,
  });
  assert.equal(statusText(readSummary([0, 0, 0, 5, 90, 0, 0]), 0.1, 100).line, "0 paths at this Threshold: too high.");
  assert.deepEqual(statusText(readSummary([0, 1, 4, 0, 100, 1, 1]), 0.1, 100), {
    line: "1 path covering the whole image.",
    muted: "",
    canCreate: true,
  });
  assert.deepEqual(statusText(readSummary([1, 0, 0, 0, 0, 0, 0]), 0.1, 100), {
    line: TOO_DETAILED,
    muted: "",
    canCreate: false,
  });
  assert.equal(statusText(readSummary([0, 1, 1204, 0, 10, 0, 1]), 2, 100).line, "1 path, 1,204 nodes, 2.0 s");
});

test("criteria 13 and 46: notices and labels", () => {
  assert.equal(tracedNotice(1, 7, 412), "Traced: 7 paths, 412 nodes.");
  assert.equal(tracedNotice(3, 21, 1204), "Traced 3 images: 21 paths, 1,204 nodes.");
  assert.equal(tracedNotice(1, 1, 1), "Traced: 1 path, 1 node.");
  assert.equal(imagesLabel(0, 3), "Tracing 1 of 3 images");
  assert.equal(imagesLabel(0, 1), "");
  assert.equal(creatingLabel(1, 3), "Tracing 2 of 3...");
  assert.equal(creatingLabel(0, 1), TRACING);
  assert.equal(specksHint(0.5, 3.78), "0.50 mm  ~ 2 px");
  assert.equal(TRACE_TOOLTIP, "Trace tool (I)");
  assert.equal(CREATE_PATHS_TOOLTIP, "Create paths (Enter)");
});

test("criterion 35: the notices are the same strings in the user page", () => {
  const page = readFileSync(path.resolve(import.meta.dirname, "..", "..", "docs", "tracing-images.md"), "utf8").replace(/\s+/g, " ");
  const quoted = [
    TRACE_EMPTY,
    TRACE_IMPORT,
    TRACE_NOT_IMAGE,
    TRACING,
    PREVIEW_OUT_OF_DATE,
    TOO_DETAILED,
    "Stop",
    statusText(readSummary([0, 7, 412, 38, 5000, 0, 7]), 0.4, 100_000).line,
    statusText(readSummary([0, 7, 412, 38, 5000, 0, 7]), 0.4, 100_000).muted,
    statusText(readSummary([0, 0, 0, 0, 0, 0, 0]), 0.1, 100).line,
    statusText(readSummary([0, 1, 4, 0, 100, 1, 1]), 0.1, 100).line,
    tracedNotice(1, 7, 412),
    tracedNotice(3, 21, 1204),
    imagesLabel(0, 3),
  ];
  for (const text of quoted) {
    assert.ok(page.includes(text), `the page quotes "${text}"`);
  }
  assert.ok(LOADING_IMAGE.length > 0);
});
