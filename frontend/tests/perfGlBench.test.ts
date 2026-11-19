// Run with `npm test`. The sizes of the perf build's GL bench (`src/lib/perfGlBench.ts`,
// `tools/perf/measure.py --gl-bench`): 1:1 at full, half and quarter size per side, plus the half
// and quarter backing store stretched over the full CSS box.

import assert from "node:assert/strict";
import { test } from "node:test";

import { benchSizes } from "../src/lib/perfGlBench.ts";

test("1:1 sizes shrink the backing store and the CSS box together", () => {
  const sizes = benchSizes([3104, 1397], [3104, 1397], false);
  assert.deepEqual(sizes.map((s) => s.label), ["1", "1/2", "1/4"]);
  assert.deepEqual(sizes[1].backing, [1552, 699]);
  assert.deepEqual(sizes[1].css, [1552, 698.5]);
  assert.deepEqual(sizes[2].backing, [776, 349]);
});

test("stretched sizes keep the full CSS box", () => {
  const sizes = benchSizes([2000, 1000], [1000, 500], true);
  assert.deepEqual(sizes.map((s) => s.label), ["1", "1/2", "1/4", "1/2 str", "1/4 str"]);
  assert.deepEqual(sizes[3].backing, [1000, 500]);
  assert.deepEqual(sizes[3].css, [1000, 500]);
  assert.deepEqual(sizes[4].css, [1000, 500]);
});
