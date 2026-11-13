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
