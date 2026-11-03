import assert from "node:assert/strict";
import test from "node:test";

import { fixtureRowKey, focusAfterRemove, tabStopRow } from "../src/lib/fixtureRowKeys.ts";

test("Up and Down move the roving focus between fixture rows without wrapping (0093 UX notes §4)", () => {
  assert.deepEqual(fixtureRowKey("ArrowDown", 0, 3), { kind: "focus", index: 1 });
  assert.deepEqual(fixtureRowKey("ArrowDown", 2, 3), { kind: "focus", index: 2 });
  assert.deepEqual(fixtureRowKey("ArrowUp", 1, 3), { kind: "focus", index: 0 });
  assert.deepEqual(fixtureRowKey("ArrowUp", 0, 3), { kind: "focus", index: 0 });
  assert.deepEqual(fixtureRowKey("Home", 2, 3), { kind: "focus", index: 0 });
  assert.deepEqual(fixtureRowKey("End", 0, 3), { kind: "focus", index: 2 });
});

test("Space on a row toggles its eye and Delete removes the fixture at once", () => {
  assert.deepEqual(fixtureRowKey(" ", 1, 3), { kind: "toggle_visible" });
  assert.deepEqual(fixtureRowKey("Delete", 1, 3), { kind: "remove" });
});

test("other keys are left to the row (Enter expands it, Tab leaves the list)", () => {
  assert.equal(fixtureRowKey("Enter", 0, 3), null);
  assert.equal(fixtureRowKey("Tab", 0, 3), null);
  assert.equal(fixtureRowKey("ArrowDown", 0, 0), null);
});

test("after a removal the focus goes to the row that took its place, else the one above, else none", () => {
  assert.equal(focusAfterRemove(0, 3), 0);
  assert.equal(focusAfterRemove(2, 3), 1);
  assert.equal(focusAfterRemove(0, 1), null);
});

test("the list has one Tab stop: the active row, or the first row when the active one is gone", () => {
  assert.equal(tabStopRow(["a", "b", "c"], "b"), 1);
  assert.equal(tabStopRow(["a", "b", "c"], "x"), 0);
  assert.equal(tabStopRow(["a", "b", "c"], null), 0);
  assert.equal(tabStopRow([], null), -1);
});
