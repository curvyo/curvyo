// Independent acceptance tests for `specs/0026-machine-profile` on the Machine block's pure UI
// logic (UX notes §2, §3, §6): the 2 x 2 origin-corner grid, the profile list keys, the typed
// number reading (empty or non-numeric reverts silently, criterion 1) and the work area row text.
import assert from "node:assert/strict";
import { test } from "node:test";

import {
  CORNERS,
  CORNER_NAMES,
  type Corner,
  areaText,
  cornerKey,
  profileRowKey,
  typedNumber,
} from "../src/lib/machineBlockKeys.ts";

test("ac11: the corners are the four stored names in grid order with their accessible names", () => {
  assert.deepEqual(CORNERS, ["top_left", "top_right", "bottom_left", "bottom_right"]);
  assert.deepEqual(CORNER_NAMES, {
    top_left: "Top left",
    top_right: "Top right",
    bottom_left: "Bottom left",
    bottom_right: "Bottom right",
  });
});

test("ac11: arrows move inside the 2 x 2 grid from every corner and never leave it", () => {
  const expected: Record<Corner, Record<string, Corner>> = {
    top_left: { ArrowRight: "top_right", ArrowDown: "bottom_left", ArrowLeft: "top_left", ArrowUp: "top_left" },
    top_right: { ArrowLeft: "top_left", ArrowDown: "bottom_right", ArrowRight: "top_right", ArrowUp: "top_right" },
    bottom_left: { ArrowRight: "bottom_right", ArrowUp: "top_left", ArrowLeft: "bottom_left", ArrowDown: "bottom_left" },
    bottom_right: { ArrowLeft: "bottom_left", ArrowUp: "top_right", ArrowRight: "bottom_right", ArrowDown: "bottom_right" },
  };
  for (const corner of CORNERS) {
    for (const [key, to] of Object.entries(expected[corner])) {
      assert.equal(cornerKey(key, corner), to, `${key} from ${corner}`);
    }
    assert.equal(cornerKey("Home", corner), "top_left");
    assert.equal(cornerKey("End", corner), "bottom_right");
    assert.equal(cornerKey("Enter", corner), null);
    assert.equal(cornerKey("Tab", corner), null);
  }
});

test("ac3: list keys move without wrap, pick, rename, duplicate, delete and close", () => {
  assert.deepEqual(profileRowKey("ArrowDown", false, 0, 1), { kind: "focus", index: 0 }, "one row");
  assert.deepEqual(profileRowKey("ArrowUp", false, 0, 5), { kind: "focus", index: 0 });
  assert.deepEqual(profileRowKey("ArrowDown", false, 4, 5), { kind: "focus", index: 4 });
  assert.deepEqual(profileRowKey("ArrowDown", false, 2, 5), { kind: "focus", index: 3 });
  assert.deepEqual(profileRowKey("Home", false, 3, 5), { kind: "focus", index: 0 });
  assert.deepEqual(profileRowKey("End", false, 0, 201), { kind: "focus", index: 200 });
  assert.deepEqual(profileRowKey("Enter", false, 1, 5), { kind: "pick" });
  assert.deepEqual(profileRowKey(" ", false, 1, 5), { kind: "pick" });
  assert.deepEqual(profileRowKey("F2", false, 1, 5), { kind: "rename" });
  assert.deepEqual(profileRowKey("d", true, 1, 5), { kind: "duplicate" });
  assert.deepEqual(profileRowKey("D", true, 1, 5), { kind: "duplicate" });
  assert.deepEqual(profileRowKey("Delete", false, 1, 5), { kind: "delete" });
  assert.deepEqual(profileRowKey("Escape", false, 1, 5), { kind: "close" });
  assert.equal(profileRowKey("d", false, 1, 5), null, "plain d does nothing");
  assert.equal(profileRowKey("Enter", true, 1, 5), null, "Ctrl+Enter is not a pick");
  assert.equal(profileRowKey("x", false, 1, 5), null);
});

test("ac1: a typed number reads a comma as the decimal point; empty or not a number reverts", () => {
  assert.equal(typedNumber("1250"), 1250);
  assert.equal(typedNumber(" 1250.4 "), 1250.4);
  assert.equal(typedNumber("1250,6"), 1250.6);
  assert.equal(typedNumber("-5"), -5);
  assert.equal(typedNumber("1e6"), 1_000_000);
  assert.equal(typedNumber("200000"), 200_000, "out of range is passed on to be limited, not refused");
  assert.equal(typedNumber(""), null);
  assert.equal(typedNumber("   "), null);
  assert.equal(typedNumber("abc"), null);
  assert.equal(typedNumber("Infinity"), null);
  assert.equal(typedNumber("NaN"), null);
  assert.equal(typedNumber("12mm"), null);
});

test("ac3: a profile row shows its work area with up to three decimals", () => {
  assert.equal(areaText(400, 300), "400 × 300 mm");
  assert.equal(areaText(600.5, 400.25), "600.5 × 400.25 mm");
  assert.equal(areaText(1, 10_000), "1 × 10000 mm");
  assert.equal(areaText(123.4567, 1.0004), "123.457 × 1 mm");
});
