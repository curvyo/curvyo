// The multi-select of operation rows (`specs/0028` ux-notes 8.2, criterion 27).
import { test } from "node:test";
import assert from "node:assert/strict";
import { NO_ROWS, extend, press, prune } from "../src/lib/rowSelection.ts";

const ORDER = ["a", "b", "c", "d", "e"];
const none = { ctrl: false, shift: false };

test("a plain press selects one row and makes it the anchor", () => {
  const sel = press(press(NO_ROWS, ORDER, "b", none), ORDER, "d", none);
  assert.deepEqual(sel, { keys: ["d"], anchor: "d" });
});

test("Ctrl toggles a row and keeps list order", () => {
  let sel = press(NO_ROWS, ORDER, "d", none);
  sel = press(sel, ORDER, "b", { ctrl: true, shift: false });
  assert.deepEqual(sel.keys, ["b", "d"]);
  sel = press(sel, ORDER, "d", { ctrl: true, shift: false });
  assert.deepEqual(sel.keys, ["b"]);
});

test("Shift extends from the anchor in both directions", () => {
  let sel = press(NO_ROWS, ORDER, "b", none);
  sel = press(sel, ORDER, "d", { ctrl: false, shift: true });
  assert.deepEqual(sel.keys, ["b", "c", "d"]);
  sel = press(sel, ORDER, "a", { ctrl: false, shift: true });
  assert.deepEqual(sel.keys, ["a", "b"]);
  assert.equal(sel.anchor, "b");
});

test("Shift without an anchor selects the pressed row", () => {
  assert.deepEqual(press(NO_ROWS, ORDER, "c", { ctrl: false, shift: true }).keys, ["c"]);
});

test("Shift+Down and Shift+Up grow and shrink the range from the anchor", () => {
  let sel = press(NO_ROWS, ORDER, "b", none);
  sel = extend(sel, ORDER, "b", 1);
  assert.deepEqual(sel.keys, ["b", "c"]);
  sel = extend(sel, ORDER, "c", 1);
  assert.deepEqual(sel.keys, ["b", "c", "d"]);
  sel = extend(sel, ORDER, "d", -1);
  assert.deepEqual(sel.keys, ["b", "c"]);
  assert.deepEqual(extend(press(NO_ROWS, ORDER, "e", none), ORDER, "e", 1).keys, ["e"]);
});

test("rows that are gone leave the selection", () => {
  const sel = { keys: ["a", "c"], anchor: "a" };
  assert.equal(prune(sel, ORDER), sel);
  assert.deepEqual(prune(sel, ["c", "d"]), { keys: ["c"], anchor: "c" });
  assert.deepEqual(prune(sel, []), { keys: [], anchor: null });
});
