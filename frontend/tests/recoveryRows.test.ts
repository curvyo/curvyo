import assert from "node:assert/strict";
import { test } from "node:test";

import { cutPath, focusAfterRemoval, localParts, rowKey } from "../src/lib/recoveryRows.ts";

test("the roving list moves without wrapping", () => {
  assert.deepEqual(rowKey("ArrowDown", 2, 3, true), { kind: "move", to: 2 });
  assert.deepEqual(rowKey("ArrowUp", 0, 3, true), { kind: "move", to: 0 });
  assert.deepEqual(rowKey("Home", 2, 3, true), { kind: "move", to: 0 });
  assert.deepEqual(rowKey("End", 0, 3, true), { kind: "move", to: 2 });
});

test("Enter restores, Right goes to the Restore button, Delete arms", () => {
  assert.deepEqual(rowKey("Enter", 0, 1, true), { kind: "restore" });
  assert.deepEqual(rowKey("ArrowRight", 0, 1, true), { kind: "to-button" });
  assert.deepEqual(rowKey("ArrowLeft", 0, 1, true), { kind: "to-row" });
  assert.deepEqual(rowKey("Delete", 0, 1, true), { kind: "arm" });
  assert.deepEqual(rowKey("Escape", 0, 1, true), { kind: "escape" });
});

test("a damaged row has no Restore: Enter and Right do nothing, Delete still arms", () => {
  assert.deepEqual(rowKey("Enter", 0, 1, false), { kind: "none" });
  assert.deepEqual(rowKey("ArrowRight", 0, 1, false), { kind: "none" });
  assert.deepEqual(rowKey("Delete", 0, 1, false), { kind: "arm" });
});

test("after a row goes the focus takes the next row, else the previous, else the tab", () => {
  assert.equal(focusAfterRemoval(0, 3), 0);
  assert.equal(focusAfterRemoval(1, 3), 1);
  assert.equal(focusAfterRemoval(2, 3), 1);
  assert.equal(focusAfterRemoval(0, 1), null);
});

test("local parts follow the page's zone", () => {
  const parts = localParts(new Date(2026, 10, 23, 12, 3).getTime());
  assert.deepEqual(parts, { year: 2026, month: 11, day: 23, hour: 12, minute: 3 });
});

test("a long path is cut at the start", () => {
  assert.equal(cutPath("/a/b", 30), "/a/b");
  assert.equal(cutPath("/home/maker/.local/share/curvyo/recovery", 20).length, 20);
  assert.ok(cutPath("/home/maker/.local/share/curvyo/recovery", 20).startsWith("..."));
});
