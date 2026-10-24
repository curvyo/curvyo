// The inline delete confirm (`specs/0081-inline-delete-confirm` criteria 3 to 6, 11, 12 and
// 16): the first press arms, a second press after 400 ms removes, one target armed at a time,
// 4 s disarms; the armed and announced texts.
import assert from "node:assert/strict";
import { test } from "node:test";

import { ARM_GUARD_MS, ARM_TIMEOUT_MS, IDLE, disarm, focusKeepsArm, pressTrash } from "../src/lib/deleteArm.ts";
import { armedRemoveLabel, armedTagLabel, removedText, tagDeletedText } from "../src/lib/fontLibraryText.ts";

test("the first press arms and removes nothing; a second press after the guard removes", () => {
  const first = pressTrash(IDLE, "font:Lobster", 1000);
  assert.equal(first.remove, false);
  assert.equal(first.state.armed, "font:Lobster");
  const second = pressTrash(first.state, "font:Lobster", 1000 + ARM_GUARD_MS);
  assert.equal(second.remove, true);
  assert.equal(second.state.armed, null);
});

test("a second press within 400 ms is ignored and the target stays armed", () => {
  const first = pressTrash(IDLE, "tag:Wedding", 0);
  const quick = pressTrash(first.state, "tag:Wedding", ARM_GUARD_MS - 1);
  assert.equal(quick.remove, false);
  assert.deepEqual(quick.state, first.state);
});

test("pressing another trash disarms the first and arms the second", () => {
  const first = pressTrash(IDLE, "font:Lobster", 0);
  const other = pressTrash(first.state, "tag:Wedding", 1000);
  assert.equal(other.remove, false);
  assert.equal(other.state.armed, "tag:Wedding");
  assert.equal(other.state.at, 1000);
});

test("after the 4 s timeout a press arms again instead of removing", () => {
  assert.equal(ARM_TIMEOUT_MS, 4000);
  const first = pressTrash(IDLE, "font:Lobster", 0);
  const late = pressTrash(first.state, "font:Lobster", ARM_TIMEOUT_MS);
  assert.equal(late.remove, false);
  assert.equal(late.state.armed, "font:Lobster");
  assert.equal(late.state.at, ARM_TIMEOUT_MS);
});

test("disarm goes back to idle", () => {
  assert.deepEqual(disarm(), IDLE);
  assert.equal(IDLE.armed, null);
});

test("the armed names and the announced texts", () => {
  assert.equal(armedRemoveLabel("Lobster", 0), "Press again to remove Lobster from Curvyo fonts.");
  assert.equal(armedRemoveLabel("Lobster", 1), "Press again to remove Lobster from Curvyo fonts. 1 text here uses it.");
  assert.equal(armedRemoveLabel("Lobster", 2), "Press again to remove Lobster from Curvyo fonts. 2 texts here use it.");
  assert.equal(armedTagLabel("Wedding", 12), "Press again to delete tag Wedding. Its 12 fonts stay in Curvyo.");
  assert.equal(armedTagLabel("Wedding", 1), "Press again to delete tag Wedding. Its 1 font stays in Curvyo.");
  assert.equal(armedTagLabel("Wedding", 0), "Press again to delete tag Wedding.");
  assert.equal(removedText("Lobster"), "Lobster removed from Curvyo fonts.");
  assert.equal(tagDeletedText("Wedding"), "Tag Wedding deleted.");
});

test("focus keeps the arm only inside the armed row; focus to nothing disarms", () => {
  assert.equal(focusKeepsArm("font:Lobster", "font:Lobster"), true);
  assert.equal(focusKeepsArm("font:Lobster", "tag:Wedding"), false);
  assert.equal(focusKeepsArm("font:Lobster", null), false);
  assert.equal(focusKeepsArm("font:Lobster", undefined), false);
});
