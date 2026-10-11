// Run with `npm test`. `backingBufferFor` sizes the canvas buffer from its CSS size, and
// `sameBackingBuffer` tells the editor session hook whether a canvas that was measured before
// the async `attach_canvas` still has that size afterwards; if not, the session gets the real
// size, which it centres a new document in (`src/lib/canvasSize.ts`).

import assert from "node:assert/strict";
import { test } from "node:test";

import { backingBufferFor, sameBackingBuffer } from "../src/lib/canvasSize.ts";

test("the buffer is the CSS size times the ratio, rounded, at least 1 px", () => {
  assert.deepEqual(backingBufferFor(800, 600, 1), { width: 800, height: 600, devicePixelRatio: 1 });
  assert.deepEqual(backingBufferFor(801, 333, 1.5), {
    width: 1202,
    height: 500,
    devicePixelRatio: 1.5,
  });
  assert.deepEqual(backingBufferFor(0, 0, 2), { width: 1, height: 1, devicePixelRatio: 2 });
});

test("a ratio that is not a positive number reads as 1", () => {
  assert.equal(backingBufferFor(10, 10, 0).devicePixelRatio, 1);
  assert.equal(backingBufferFor(10, 10, Number.NaN).devicePixelRatio, 1);
});

test("a canvas that changed size during the attach differs from the one measured before", () => {
  const before = backingBufferFor(900, 700, 1);
  assert.equal(sameBackingBuffer(before, backingBufferFor(900, 700, 1)), true);
  assert.equal(sameBackingBuffer(before, backingBufferFor(1180, 700, 1)), false);
  assert.equal(sameBackingBuffer(before, backingBufferFor(900, 640, 1)), false);
  assert.equal(sameBackingBuffer(before, backingBufferFor(900, 700, 2)), false);
});
