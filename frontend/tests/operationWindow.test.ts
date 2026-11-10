// The row windowing of the Operations list (`specs/0101-laser-operations` `ux-notes.md` section 12:
// at most 60 row elements in the DOM, the focused and the open row kept, the scrollbar exact).
import { test } from "node:test";
import assert from "node:assert/strict";
import { visibleRange } from "../src/lib/operationWindow.ts";

test("a long list renders a window of at most 60 entries at any scroll position", () => {
  const heights = Array.from({ length: 1000 }, () => 36);
  for (let top = 0; top <= 1000 * 36; top += 997) {
    const range = visibleRange(heights, top, 360, 120, null, null);
    assert.ok(range.last - range.first <= 60, `top ${top}: ${range.last - range.first}`);
    assert.ok(range.first <= Math.floor(top / 36) || range.first === 0);
  }
});

test("the spacers before and after keep the scrollbar exact", () => {
  const heights = [36, 106, 24, 36, 36, 36, 36, 36, 36, 36];
  const total = heights.reduce((sum, h) => sum + h, 0);
  const range = visibleRange(heights, 100, 80, 0, null, null);
  const shown = heights.slice(range.first, range.last).reduce((sum, h) => sum + h, 0);
  assert.equal(range.before + shown + range.after, total);
  assert.equal(range.before, heights.slice(0, range.first).reduce((sum, h) => sum + h, 0));
});

test("the focused and the open entry stay rendered even off screen", () => {
  const heights = Array.from({ length: 200 }, () => 36);
  const range = visibleRange(heights, 0, 360, 0, 150, 160);
  assert.ok(range.first <= 0 && range.last >= 161, JSON.stringify(range));
  assert.equal(range.before, 0);
});

test("an empty list renders nothing", () => {
  assert.deepEqual(visibleRange([], 0, 360, 120, null, null), {
    first: 0,
    last: 0,
    before: 0,
    after: 0,
  });
});
