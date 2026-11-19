// Run with `npm test`. The perf build's GPU-path variants (`src/lib/perfGpuVariants.ts`,
// `tools/perf/measure.py --gpu-variants`): which part goes to the wasm, the pre-created context,
// the draw timing or the page, and how a context attribute that did not take is named.

import assert from "node:assert/strict";
import { test } from "node:test";

import { attributesNotTaken, planGpuVariant } from "../src/lib/perfGpuVariants.ts";

test("baseline is the production path everywhere", () => {
  const plan = planGpuVariant("baseline");
  assert.deepEqual(plan, { wasm: "", context: null, page: "baseline", drawAfterInput: false, input: "pan", unknown: [] });
});

test("wasm parts are joined for the wasm, context parts keep wgpu's antialias: false", () => {
  const plan = planGpuVariant("samples-1+ctx-no-depth+no-fences+ctx-opaque");
  assert.equal(plan.wasm, "samples-1+no-fences");
  assert.deepEqual(plan.context, { antialias: false, depth: false, alpha: false });
});

test("draw timing, the static redraw and page ablations are the page's", () => {
  assert.equal(planGpuVariant("draw-after-input").drawAfterInput, true);
  assert.equal(planGpuVariant("redraw-static").input, "redraw");
  assert.equal(planGpuVariant("page-no-draw").page, "no-draw");
  assert.equal(planGpuVariant("page-canvas-only").page, "canvas-only");
});

test("unknown parts are reported, not dropped", () => {
  assert.deepEqual(planGpuVariant("samples-3+page-bogus+fast").unknown, ["samples-3", "page-bogus", "fast"]);
});

test("an attribute the context did not take is named with what it got", () => {
  assert.deepEqual(attributesNotTaken({ depth: false, desynchronized: true }, { depth: false, desynchronized: false } as WebGLContextAttributes), ["desynchronized=true->false"]);
  assert.deepEqual(attributesNotTaken(null, null), []);
  assert.deepEqual(attributesNotTaken({ alpha: false }, null), ["no context"]);
});
