// Run with `npm test`. The Measure tool's overlay as Rust hands it over and where its chips go
// (`specs/0070-measure-and-dimension` criteria 1 and 2, `src/lib/measureView.ts`).

import assert from "node:assert/strict";
import { test } from "node:test";

import { chipPlace, EMPTY_MEASURE_VIEW, readMeasureView } from "../src/lib/measureView.ts";

test("reads Rust's overlay and falls back to the empty view", () => {
  const view = readMeasureView(JSON.stringify({
    dimensions: [{ id: "a", text: "12.5 mm", name: "Dimension 12.5 millimetres", x: 1, y: 2, linked: false, selected: false, preview: false }],
    mode: { lines: ["Dimension, aligned"], x: 3, y: 4 },
    readout: null,
    speech: "12.5 millimetres, 0 degrees",
  }));
  assert.equal(view.speech, "12.5 millimetres, 0 degrees");
  assert.equal(view.dimensions[0].text, "12.5 mm");
  assert.equal(view.mode?.lines[0], "Dimension, aligned");
  assert.deepEqual(readMeasureView("not json"), EMPTY_MEASURE_VIEW);
});

test("puts a chip up and right of its point and flips it at the edges", () => {
  const size = { width: 80, height: 30 };
  const canvas = { width: 400, height: 300 };
  assert.deepEqual(chipPlace({ x: 100, y: 100 }, size, canvas), { left: 112, top: 58 });
  assert.deepEqual(chipPlace({ x: 380, y: 10 }, size, canvas), { left: 288, top: 22 });
});
