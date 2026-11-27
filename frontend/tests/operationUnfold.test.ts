// The unfolded row's rules (`specs/0111-laser-operation-tools` criterion 17, `ux-notes.md` 6.1,
// 12): which press unfolds, which row shows its body, and the one tall window item.
import { test } from "node:test";
import assert from "node:assert/strict";
import {
  UNFOLDED_BODY_ESTIMATE,
  bodyOpen,
  canUnfold,
  entryHeight,
  pressUnfolds,
  unfoldedViewport,
} from "../src/lib/operationUnfold.ts";
import { ROW_HEIGHT } from "../src/lib/operationRows.ts";

const settings = {} as never;
const base = { id: { peer: "1", counter: 7 }, kind: "engrave", unfolded: false, source: null, settings } as never;
const row = (patch: Record<string, unknown> = {}) => ({ ...(base as object), ...patch }) as never;

test("only a known, settings-bearing row unfolds", () => {
  assert.equal(canUnfold(row()), true);
  assert.equal(canUnfold(row({ kind: "unknown", settings: null })), false);
  assert.equal(canUnfold(row({ settings: null })), false);
});

test("the body shows for the unfolded row while it is the one selected row", () => {
  assert.equal(bodyOpen(row({ unfolded: true }), ["1:7"]), true);
  assert.equal(bodyOpen(row({ unfolded: true }), ["1:7", "1:8"]), false);
  assert.equal(bodyOpen(row({ unfolded: true }), ["1:8"]), false);
  assert.equal(bodyOpen(row({ unfolded: false }), ["1:7"]), false);
  assert.equal(bodyOpen(row({ kind: "unknown", settings: null, unfolded: true }), ["1:7"]), false);
});

test("a plain press unfolds a folded row and folds the open one; a modifier press never unfolds", () => {
  const plain = { ctrl: false, shift: false };
  assert.equal(pressUnfolds(row(), ["1:7"], plain), true, "folded: ask the session to unfold");
  assert.equal(pressUnfolds(row({ unfolded: true }), ["1:7"], plain), true, "open: ask it to fold");
  assert.equal(pressUnfolds(row({ unfolded: true }), ["1:8"], plain), false, "unfolded but unselected: selecting shows it");
  assert.equal(pressUnfolds(row(), ["1:7"], { ctrl: true, shift: false }), false);
  assert.equal(pressUnfolds(row(), ["1:7"], { ctrl: false, shift: true }), false);
  assert.equal(pressUnfolds(row({ kind: "unknown", settings: null }), [], plain), false);
});

test("the unfolded row is one tall window item; every other row keeps the fixed height", () => {
  assert.equal(entryHeight(false, null), ROW_HEIGHT);
  assert.equal(entryHeight(true, null), ROW_HEIGHT + UNFOLDED_BODY_ESTIMATE);
  assert.equal(entryHeight(true, 212), ROW_HEIGHT + 212);
});

test("an unfolded list fills the room under it at any window size, never past it (specs/0139 UX review)", () => {
  // 1920 x 1080: 900 px of room holds a 760 px body list whole.
  assert.equal(unfoldedViewport(760, 900, 300), 760);
  // 800 x 640: a 760 px list scrolls inside the 380 px left under its top, so its end is reachable.
  assert.equal(unfoldedViewport(760, 380, 300), 380);
  // Never under the folded window, and the old 520 px cap is gone.
  assert.equal(unfoldedViewport(760, 120, 300), 300);
  assert.equal(unfoldedViewport(1200, 1000, 300), 1000);
  // Before the room is measured the list keeps the 520 px of old.
  assert.equal(unfoldedViewport(760, null, 300), 520);
  assert.equal(unfoldedViewport(200, null, 300), 300);
});
