// Run with `npm test`. The text edit sink sits at the caret, but never outside the canvas box: a
// focused field past the view made the webview scroll the whole app page to reveal it (customer
// report 2026-10-24, `src/lib/pageScroll.ts`).

import assert from "node:assert/strict";
import { test } from "node:test";

import { clampSinkPoint } from "../src/lib/pageScroll.ts";

test("a caret inside the canvas keeps its point", () => {
  assert.deepEqual(clampSinkPoint({ x: 120.5, y: 80 }, 800, 600), { x: 120.5, y: 80 });
});

test("a caret below or right of the canvas is held at its last pixel", () => {
  assert.deepEqual(clampSinkPoint({ x: 400, y: 1900 }, 800, 600), { x: 400, y: 599 });
  assert.deepEqual(clampSinkPoint({ x: 2600, y: 300 }, 800, 600), { x: 799, y: 300 });
  assert.deepEqual(clampSinkPoint({ x: 5000, y: 5000 }, 800, 600), { x: 799, y: 599 });
});

test("a caret above or left of the canvas is held at its first pixel", () => {
  assert.deepEqual(clampSinkPoint({ x: -40, y: -3 }, 800, 600), { x: 0, y: 0 });
});

test("no caret, an unmeasured box or a non-finite point puts the sink at the corner", () => {
  assert.deepEqual(clampSinkPoint(null, 800, 600), { x: 0, y: 0 });
  assert.deepEqual(clampSinkPoint({ x: 300, y: 200 }, 0, 0), { x: 0, y: 0 });
  assert.deepEqual(clampSinkPoint({ x: Number.NaN, y: Infinity }, 800, 600), { x: 0, y: 599 });
});
