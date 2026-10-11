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
