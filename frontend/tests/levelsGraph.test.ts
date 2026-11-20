// The Levels graph's handles (`specs/0135-image-optimisation` criterion 5, `ux-notes.md` §3.8).
import { test } from "node:test";
import assert from "node:assert/strict";
import { levelsMarks, levelsValueAt, levelsValueStepped } from "../src/lib/levelsGraph.ts";

test("a handle drag gives the field value at the pointer, held to the range", () => {
  assert.equal(levelsValueAt(0, 0.104, 0, 100), 10);
  assert.equal(levelsValueAt(0, 0.95, 0, 90), 88, "Black at most White - 2");
  assert.equal(levelsValueAt(2, 0.05, 10, 100), 12, "White at least Black + 2");
  assert.equal(levelsValueAt(2, 1.3, 0, 100), 100);
  // Gamma: the mid mark at the middle is 1.00; the mark is the inverse of `levelsMarks`.
  assert.equal(levelsValueAt(1, 0.5, 0, 100), 1);
  const [, mid] = levelsMarks(10, 90, 1.5);
  assert.equal(levelsValueAt(1, mid, 10, 90), 1.5);
  assert.equal(levelsValueAt(1, 0.0, 10, 90), 4, "held to 4.00");
});

test("Up and Down move a handle 1 % of the axis, Shift 10 %", () => {
  assert.equal(levelsValueStepped(0, 1, 5, 95, 1), 6);
  assert.equal(levelsValueStepped(2, -10, 5, 95, 1), 85);
  const lighter = levelsValueStepped(1, -1, 0, 100, 1);
  assert.ok(lighter > 1 && lighter < 1.1, `${lighter}`);
});
