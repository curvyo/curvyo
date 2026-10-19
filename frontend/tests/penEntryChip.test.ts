// Run with `npm test`. A frozen Pen entry chip keeps off the segment from L to the typed point
// (`specs/0072-pen-angle-snap-and-typed-values/`, UX review SHOULD-FIX 1).

import assert from "node:assert/strict";
import { test } from "node:test";

import { placePenEntryChip } from "../src/lib/readoutPlacement.ts";

test("a frozen chip over the typed segment moves to the other side of L", () => {
  const chip = { width: 292, height: 40 };
  const canvas = { width: 1200, height: 800 };
  const last = { x: 600, y: 250 };
  const pointer = { x: 610, y: 245 };
  // The UX screenshot: 40 mm at -30 degrees ends at (631, 224), under the chip by the pointer.
  const end = { x: 631, y: 224 };
  const unfrozen = placePenEntryChip(pointer, last, null, chip, canvas);
  assert.deepEqual(unfrozen, { left: 622, top: 193 });
  assert.deepEqual(placePenEntryChip(pointer, last, end, chip, canvas), { left: 622, top: 265 });
});
