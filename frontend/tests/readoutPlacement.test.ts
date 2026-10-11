// Run with `npm test`. `placeAway` puts the Pen's hint chip on the side of the pointer away from
// the path in progress (`src/lib/readoutPlacement.ts`).

import assert from "node:assert/strict";
import { test } from "node:test";

import { placeAway, placeMoveEntryChip } from "../src/lib/readoutPlacement.ts";

const chip = { width: 100, height: 40 };
const canvas = { width: 800, height: 600 };
const at = { x: 400, y: 300 };

test("the chip sits on the side the direction points to", () => {
  assert.deepEqual(placeAway(at, chip, canvas, 12, { x: 1, y: 1 }), { left: 412, top: 312 });
  assert.deepEqual(placeAway(at, chip, canvas, 12, { x: -1, y: 1 }), { left: 288, top: 312 });
  assert.deepEqual(placeAway(at, chip, canvas, 12, { x: 1, y: -1 }), { left: 412, top: 248 });
  assert.deepEqual(placeAway(at, chip, canvas, 12, { x: -1, y: -1 }), { left: 288, top: 248 });
});

test("the chip stays inside the canvas", () => {
  const edge = placeAway({ x: 10, y: 10 }, chip, canvas, 12, { x: -1, y: -1 });
  assert.deepEqual(edge, { left: 0, top: 0 });
});

test("the move chip sits 16 px below the box, and no higher than 16 px below the centre", () => {
  const anchor = { center: { x: 400, y: 200 }, boxBottomY: 260 };
  assert.deepEqual(placeMoveEntryChip(anchor, chip, canvas, 40), { left: 416, top: 276 });
  const small = { center: { x: 400, y: 200 }, boxBottomY: 210 };
  assert.deepEqual(placeMoveEntryChip(small, chip, canvas, 40), { left: 416, top: 226 });
  // A bottom edge above the centre (never the case for a box) cannot lift the chip.
  const odd = { center: { x: 400, y: 200 }, boxBottomY: 190 };
  assert.deepEqual(placeMoveEntryChip(odd, chip, canvas, 40), { left: 416, top: 216 });
});

test("the move chip flips by its Absolute height, so the mode never changes the side", () => {
  const anchor = { center: { x: 400, y: 450 }, boxBottomY: 470 };
  const relative = { width: 100, height: 40 };
  // 486 + 130 > 600: above the centre glyph, with the chip's own height.
  assert.deepEqual(placeMoveEntryChip(anchor, relative, canvas, 130), { left: 416, top: 394 });
  // The same anchor with the chip already at its Absolute height flips to the same side.
  const absolute = { width: 100, height: 130 };
  assert.deepEqual(placeMoveEntryChip(anchor, absolute, canvas, 130), { left: 416, top: 304 });
  // Room below: below, in both modes.
  const roomy = { center: { x: 400, y: 100 }, boxBottomY: 150 };
  assert.equal(placeMoveEntryChip(roomy, relative, canvas, 130).top, 166);
  assert.equal(placeMoveEntryChip(roomy, absolute, canvas, 130).top, 166);
});

test("the move chip flips left at the right edge and stays inside the canvas", () => {
  const anchor = { center: { x: 780, y: 100 }, boxBottomY: 150 };
  assert.deepEqual(placeMoveEntryChip(anchor, chip, canvas, 40), { left: 664, top: 166 });
});
