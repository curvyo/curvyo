// The Machine block's pure helpers (`specs/0026-machine-profile` UX notes §3, §6).
import assert from "node:assert/strict";
import { test } from "node:test";

import { areaText, cornerKey, profileRowKey, typedNumber } from "../src/lib/machineBlockKeys.ts";

test("arrows move in the 2 x 2 corner grid and stop at its edge", () => {
  assert.equal(cornerKey("ArrowRight", "bottom_left"), "bottom_right");
  assert.equal(cornerKey("ArrowUp", "bottom_left"), "top_left");
  assert.equal(cornerKey("ArrowLeft", "top_left"), "top_left");
  assert.equal(cornerKey("Home", "bottom_right"), "top_left");
  // `0045` criterion 41: no wrap; Up from Bottom right is Top right, Right stays.
  assert.equal(cornerKey("ArrowUp", "bottom_right"), "top_right");
  assert.equal(cornerKey("ArrowRight", "bottom_right"), "bottom_right");
  assert.equal(cornerKey("End", "top_left"), "bottom_right");
  assert.equal(cornerKey("End", "top_left"), "bottom_right");
  assert.equal(cornerKey("a", "top_left"), null);
});

test("profile rows move without wrapping and name their actions", () => {
  assert.deepEqual(profileRowKey("ArrowUp", false, 0, 3), { kind: "focus", index: 0 });
  assert.deepEqual(profileRowKey("ArrowDown", false, 2, 3), { kind: "focus", index: 2 });
  assert.deepEqual(profileRowKey("End", false, 0, 3), { kind: "focus", index: 2 });
  assert.deepEqual(profileRowKey(" ", false, 1, 3), { kind: "pick" });
  assert.deepEqual(profileRowKey("F2", false, 1, 3), { kind: "rename" });
  assert.deepEqual(profileRowKey("d", true, 1, 3), { kind: "duplicate" });
  assert.deepEqual(profileRowKey("Delete", false, 1, 3), { kind: "delete" });
  assert.deepEqual(profileRowKey("Escape", false, 1, 3), { kind: "close" });
  assert.equal(profileRowKey("x", false, 1, 3), null);
});

test("typed numbers accept a comma and revert on nonsense", () => {
  assert.equal(typedNumber("1250,5"), 1250.5);
  assert.equal(typedNumber("  "), null);
  assert.equal(typedNumber("fast"), null);
  assert.equal(areaText(400, 300), "400 × 300 mm");
  assert.equal(areaText(600.12345, 400), "600.123 × 400 mm");
});
