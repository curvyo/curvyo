// Run with `npm test`. The typed-width field and the wheel readout of a width point
// (`specs/0114` criteria 6 and 7, UX review 2026-11-12) open below or above the diamond, never
// up and to the right over the stroke, and stay inside the canvas (`src/lib/readoutPlacement.ts`).

import assert from "node:assert/strict";
import { test } from "node:test";

import { placeWidthChip } from "../src/lib/readoutPlacement.ts";

const chip = { width: 140, height: 32 };
const canvas = { width: 340, height: 560 };

test("the field opens below the diamond, starting just left of it", () => {
  assert.deepEqual(placeWidthChip({ x: 100, y: 200 }, chip, canvas, "below"), { left: 86, top: 214 });
});

test("the readout opens above the diamond", () => {
  assert.deepEqual(placeWidthChip({ x: 100, y: 200 }, chip, canvas, "above"), { left: 86, top: 154 });
});

test("without room on the right the chip ends just right of the diamond", () => {
  // The 800 × 600 case: a diamond 20 px from the canvas's right edge.
  assert.deepEqual(placeWidthChip({ x: 320, y: 200 }, chip, canvas, "below"), { left: 194, top: 214 });
});

test("without room below or above the chip goes to the other side", () => {
  assert.deepEqual(placeWidthChip({ x: 100, y: 540 }, chip, canvas, "below"), { left: 86, top: 494 });
  assert.deepEqual(placeWidthChip({ x: 100, y: 20 }, chip, canvas, "above"), { left: 86, top: 34 });
});

test("the chip never leaves the canvas", () => {
  const wide = { width: 400, height: 32 };
  assert.deepEqual(placeWidthChip({ x: 5, y: 200 }, wide, canvas, "below"), { left: 0, top: 214 });
});
