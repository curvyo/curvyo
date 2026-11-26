// The Into slot list's spoken slot names (`specs/0154-library-into-slots` UX notes 8.2).
import assert from "node:assert/strict";
import { test } from "node:test";

import { slotOptionLabel } from "../src/lib/intoSlotWords.ts";

test("a scaled rectangle slot is read in words", () => {
  assert.equal(slotOptionLabel("Slot 2", "rect", "30 × 30 mm", "50 %", false), "Slot 2, rectangle, 30 by 30 millimetres, scaled to 50 percent");
});

test("a circle slot drops the diameter sign, a turned scale keeps its turn", () => {
  assert.equal(slotOptionLabel("Slot 3", "circle", "⌀ 25 mm", "67 %, turned", false), "Slot 3, circle, 25 millimetres, scaled to 67 percent, turned");
});

test("a filled slot says filled, a plain centre says nothing more", () => {
  assert.equal(slotOptionLabel("Slot 1", "rect", "20 × 20 mm", "6.7 %", true), "Slot 1, rectangle, 20 by 20 millimetres, filled");
  assert.equal(slotOptionLabel("Slot 1", "rect", "20 × 20 mm", null, false), "Slot 1, rectangle, 20 by 20 millimetres");
  assert.equal(slotOptionLabel("Slot 1", "rect", "20 × 20 mm", "too large", false), "Slot 1, rectangle, 20 by 20 millimetres, too large");
});
