// Run with `npm test`. `placePlacementChip` puts the click-to-create chip on the side of the
// pointer away from A, flips it across A where there is no room, and keeps it in the canvas
// (`specs/0052-click-to-create-shapes/` criteria 11 and 33).

import assert from "node:assert/strict";
import { test } from "node:test";

import { placePlacementChip } from "../src/lib/readoutPlacement.ts";

const chip = { width: 424, height: 70 };
const canvas = { width: 800, height: 600 };

test("the chip sits 12 px from the pointer on the side away from A", () => {
  const a = { x: 200, y: 300 };
  assert.deepEqual(placePlacementChip({ x: 250, y: 340 }, a, chip, canvas), { left: 262, top: 352 });
  assert.deepEqual(placePlacementChip({ x: 250, y: 260 }, a, chip, canvas), { left: 262, top: 178 });
  // No offset counts as right and down.
  assert.deepEqual(placePlacementChip(a, a, chip, canvas), { left: 212, top: 312 });
});

test("without room on one axis the chip clamps when the other axis keeps it off A", () => {
  const a = { x: 700, y: 300 };
  // Right of the pointer it would leave the canvas; below the pointer keeps it off A, so it
  // clamps to the right edge and stays next to the pointer.
  assert.deepEqual(placePlacementChip({ x: 720, y: 320 }, a, chip, canvas), {
    left: 800 - 424,
    top: 332,
  });
});

test("without room on both axes near A it goes to the other side of A", () => {
  const small = { width: 200, height: 70 };
  const a = { x: 700, y: 560 };
  // Right and below both leave the canvas: x clamps (y is not yet clear), then y flips above A.
  const at = placePlacementChip({ x: 705, y: 565 }, a, small, canvas);
  assert.deepEqual(at, { left: 600, top: 560 - 15 - 70 });
});

test("the chip keeps off the active tool's bar and right of the rail", () => {
  const small = { width: 272, height: 70 };
  const bar = { left: 300, top: 8, right: 700, bottom: 44 };
  // Above A near the top the chip would cover the bar: it moves to the pointer's other side.
  const a = { x: 400, y: 140 };
  const at = placePlacementChip({ x: 420, y: 100 }, a, small, canvas, bar, 128);
  assert.deepEqual(at, { left: 432, top: 112 });
  // Left of the pointer near the rail: kept right of the rail when it fits there.
  const left = placePlacementChip({ x: 200, y: 400 }, { x: 260, y: 380 }, small, canvas, bar, 128);
  assert.equal(left.left, 128);
  // In an 800 x 600 window the canvas is 464 px wide: the 424 px rectangle chip cannot clear the
  // rail there, and that overlap is accepted.
  const narrow = { width: 464, height: 600 };
  const wide = placePlacementChip({ x: 200, y: 400 }, { x: 260, y: 380 }, chip, narrow, bar, 128);
  assert.ok(wide.left >= 0 && wide.left < 128 && wide.left + chip.width <= narrow.width);
});

test("an 800 x 600 canvas keeps the chip inside and off A near an edge", () => {
  for (const a of [{ x: 20, y: 20 }, { x: 780, y: 580 }, { x: 20, y: 580 }, { x: 780, y: 20 }]) {
    for (const pointer of [{ x: a.x + 5, y: a.y + 5 }, { x: a.x - 5, y: a.y - 5 }]) {
      const at = placePlacementChip(pointer, a, chip, canvas);
      assert.ok(at.left >= 0 && at.left + chip.width <= canvas.width);
      assert.ok(at.top >= 0 && at.top + chip.height <= canvas.height);
      const coversA =
        a.x >= at.left && a.x <= at.left + chip.width && a.y >= at.top && a.y <= at.top + chip.height;
      assert.ok(!coversA, `covers A at ${JSON.stringify(a)}`);
    }
  }
});
