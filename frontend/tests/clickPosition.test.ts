import assert from "node:assert/strict";
import { test } from "node:test";

import { clickPosition, dragPosition } from "../src/lib/dragPosition.ts";
import { SliderGesture } from "../src/lib/sliderGesture.ts";

test("a click sets the position of the pointer on the field, clamped to 0 and 1", () => {
  assert.equal(clickPosition(122, 244), 0.5);
  assert.equal(clickPosition(0, 244), 0);
  assert.equal(clickPosition(244, 244), 1);
  assert.equal(clickPosition(-30, 244), 0);
  assert.equal(clickPosition(900, 244), 1);
});

test("a click on a field of no width or at a position that is no number is the start", () => {
  assert.equal(clickPosition(10, 0), 0);
  assert.equal(clickPosition(10, -5), 0);
  assert.equal(clickPosition(Number.NaN, 244), 0);
  assert.equal(clickPosition(Number.POSITIVE_INFINITY, 244), 1);
});

test("a click is absolute where a drag is relative to the value at the press", () => {
  const frame = { mixed: false, baseP: 0.2, baseX: 50, factor: 1, left: 0, width: 200 };
  // The pointer at 100 px: a drag from the press at 50 px adds 50/200 to 0.2; a click goes to 0.5.
  assert.equal(dragPosition(frame, 100), 0.45);
  assert.equal(clickPosition(100, 200), 0.5);
});

const field = { x: 100, y: 14 };

function click(g: SliderGesture, at: { x: number; y: number }, time: number) {
  g.down(at, false, null);
  return g.up(at, time);
}

test("a value field gesture: a click acts at the release, a double-click types, a drag past 3 px drags", () => {
  const g = new SliderGesture();
  g.down(field, false, null);
  assert.equal(g.move({ x: 101, y: 14 }).kind, "none");
  assert.equal(g.up({ x: 101, y: 14 }, 1000).kind, "click");
  assert.equal(click(g, { x: 102, y: 14 }, 1200).kind, "double-click");
  const drag = new SliderGesture();
  drag.down(field, false, null);
  assert.equal(drag.move({ x: 104, y: 14 }).kind, "drag-start");
  assert.equal(drag.up({ x: 120, y: 14 }, 2000).kind, "drag-end");
});

test("the second press of a double-click changes nothing until its release, which types", () => {
  const g = new SliderGesture();
  assert.equal(click(g, field, 1000).kind, "click");
  g.down({ x: 103, y: 14 }, false, null);
  assert.equal(g.move({ x: 103, y: 14 }).kind, "none");
  assert.equal(g.up({ x: 103, y: 14 }, 1250).kind, "double-click");
});

test("a drag past the right edge keeps raising the position up to the field's limit, a click does not", () => {
  const frame = { mixed: false, baseP: 0.9, baseX: 100, factor: 1, left: 0, width: 200, limit: 1.85 };
  // 100 px right of the press is half a field length: 0.9 + 0.5.
  assert.equal(dragPosition(frame, 200), 1.4);
  assert.equal(dragPosition(frame, 100 + 200 * 5), 1.85, "clamped at the limit");
  assert.equal(dragPosition(frame, 100 - 200 * 5), 0, "and at the left end");
  // Back inside the field the position follows the track again, with no re-basing.
  assert.ok(Math.abs(dragPosition(frame, 80) - 0.8) < 1e-12);
  // A field without room beyond its track (an opacity) stays at 1.
  assert.equal(dragPosition({ ...frame, limit: undefined }, 100 + 200 * 5), 1);
  // A click is the track only, whatever the pointer does outside.
  assert.equal(clickPosition(200 * 3, 200), 1);
});

test("a mixed drag takes the pointer's own place and overshoots the same way", () => {
  const frame = { mixed: true, baseP: 0, baseX: 0, factor: 1, left: 50, width: 200, limit: 10.18 };
  assert.equal(dragPosition(frame, 50 + 300), 1.5);
});

test("a drag that starts on a value past the track starts from its own position", () => {
  const frame = { mixed: false, baseP: 1.4, baseX: 300, factor: 1, left: 0, width: 200, limit: 1.85 };
  assert.equal(dragPosition(frame, 300), 1.4, "no jump at the first move");
  assert.equal(dragPosition(frame, 310), 1.45);
});
