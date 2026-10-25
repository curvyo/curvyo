// Run with `npm test`. `placeTextPathEntryChip` puts the entry box of a text-on-path handle on
// the side of the carrier away from the text, outside the text's box, flipped inside the canvas
// when there is no room (`src/lib/readoutPlacement.ts`).

import assert from "node:assert/strict";
import { test } from "node:test";

import { type Rect, placeTextPathEntryChip } from "../src/lib/readoutPlacement.ts";

const chip = { width: 300, height: 40 };
const canvas = { width: 1000, height: 700 };

function overlaps(at: { left: number; top: number }, box: Rect): boolean {
  return at.left < box.right && at.left + chip.width > box.left && at.top < box.bottom && at.top + chip.height > box.top;
}

test("text above the handle (top of a ring): the box goes below, centred on the handle", () => {
  const box = { left: 400, top: 200, right: 600, bottom: 305 };
  const at = placeTextPathEntryChip({ x: 500, y: 300 }, box, chip, canvas, 0);
  assert.deepEqual(at, { left: 350, top: 321 });
  assert.ok(!overlaps(at, box));
});

test("text below the handle (inside a ring at the top): the box goes above", () => {
  const box = { left: 400, top: 300, right: 600, bottom: 380 };
  const at = placeTextPathEntryChip({ x: 500, y: 300 }, box, chip, canvas, 0);
  assert.deepEqual(at, { left: 350, top: 244 });
  assert.ok(!overlaps(at, box));
});

test("text left of the handle (right side of a ring): the box goes right", () => {
  const box = { left: 500, top: 200, right: 610, bottom: 400 };
  const at = placeTextPathEntryChip({ x: 600, y: 300 }, box, chip, canvas, 0);
  assert.deepEqual(at, { left: 626, top: 280 });
  assert.ok(!overlaps(at, box));
});

test("no room below: flipped above the text's box, never over the text", () => {
  const box = { left: 400, top: 560, right: 600, bottom: 660 };
  const at = placeTextPathEntryChip({ x: 500, y: 655 }, box, chip, canvas, 0);
  assert.deepEqual(at, { left: 350, top: 504 });
  assert.ok(!overlaps(at, box));
});

test("no room on the right: flipped to the left of the text's box", () => {
  const box = { left: 700, top: 200, right: 800, bottom: 400 };
  const at = placeTextPathEntryChip({ x: 795, y: 300 }, { ...box }, { width: 300, height: 40 }, { width: 900, height: 700 }, 0);
  assert.equal(at.left, 700 - 16 - 300);
  assert.ok(!overlaps(at, box));
});

test("the left inset (tool rail) counts as no room", () => {
  const box = { left: 200, top: 200, right: 300, bottom: 400 };
  const at = placeTextPathEntryChip({ x: 205, y: 300 }, box, chip, canvas, 128);
  assert.equal(at.left, 316);
  assert.ok(!overlaps(at, box));
});

test("a box too wide for the canvas is clamped inside it", () => {
  const at = placeTextPathEntryChip({ x: 10, y: 10 }, { left: 0, top: 0, right: 20, bottom: 5 }, chip, canvas, 0);
  assert.ok(at.left >= 0 && at.top >= 0);
});
