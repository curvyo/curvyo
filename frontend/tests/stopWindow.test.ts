// The windowed stop list of the Gradient block (`specs/0124-gradient-fill` UX review U4).
import { test } from "node:test";
import assert from "node:assert/strict";
import { STOP_ROW_PX, STOP_WINDOW_FROM, STOP_WINDOW_ROWS, scrollToRow, stopWindow } from "../src/lib/stopWindow.ts";

test("short lists render every row", () => {
  assert.deepEqual(stopWindow(2, 0), { first: 0, end: 2 });
  assert.deepEqual(stopWindow(STOP_WINDOW_FROM, 500), { first: 0, end: STOP_WINDOW_FROM });
});

test("a long list renders the rows in view and a margin", () => {
  assert.deepEqual(stopWindow(256, 0), { first: 0, end: STOP_WINDOW_ROWS + 4 });
  const at = stopWindow(256, 100 * STOP_ROW_PX);
  assert.deepEqual(at, { first: 96, end: 100 + STOP_WINDOW_ROWS + 4 });
  const last = stopWindow(256, 10_000 * STOP_ROW_PX);
  assert.equal(last.end, 256);
  assert.ok(last.first <= 256 - STOP_WINDOW_ROWS);
  for (const scrollTop of [0, 17, 999, 4_000, 8_000]) {
    const w = stopWindow(256, scrollTop);
    assert.ok(w.end - w.first <= STOP_WINDOW_ROWS + 8, `${scrollTop}`);
  }
});

test("scrolling to a row brings it into the box and leaves a visible row alone", () => {
  assert.equal(scrollToRow(5, 0), 0);
  assert.equal(scrollToRow(30, 0), (31 - STOP_WINDOW_ROWS) * STOP_ROW_PX);
  assert.equal(scrollToRow(3, 20 * STOP_ROW_PX), 3 * STOP_ROW_PX);
});
