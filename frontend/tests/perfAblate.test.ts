// Run with `npm test`. The perf build's ablation variants (`src/lib/perfAblate.ts`,
// `tools/perf/measure.py --ablate`): the backing-store ratio each one sizes the canvas with, the
// input its scenario sends, and that a normal build (no `VITE_CURVYO_PERF`) is never ablated.

import assert from "node:assert/strict";
import { test } from "node:test";

import { ablatedPixelRatio, ablationInput, perfPixelRatio, perfSkipDraw } from "../src/lib/perfAblate.ts";

test("dpr1 sizes the backing store at CSS size, half-backing at half the device pixels", () => {
  assert.equal(ablatedPixelRatio("dpr1", 2), 1);
  assert.equal(ablatedPixelRatio("half-backing", 2), 1);
  assert.equal(ablatedPixelRatio("half-backing", 1.5), 0.75);
  assert.equal(ablatedPixelRatio("baseline", 2), 2);
  assert.equal(ablatedPixelRatio("no-rulers", 1.25), 1.25);
});

test("no-view hovers, idle sends nothing, every other variant pans", () => {
  assert.equal(ablationInput("no-view"), "hover");
  assert.equal(ablationInput("idle"), "none");
  assert.equal(ablationInput("no-draw"), "pan");
  assert.equal(ablationInput("baseline"), "pan");
});

test("outside the perf build the window's ratio is kept and every frame is drawn", () => {
  assert.equal(perfPixelRatio(2), 2);
  assert.equal(perfSkipDraw(), false);
});
