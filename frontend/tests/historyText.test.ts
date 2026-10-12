import assert from "node:assert/strict";
import test from "node:test";

import {
  historyKeyDirection,
  isMac,
  menuNames,
  noticeDurationMs,
  noticeText,
  workingNotice,
} from "../src/lib/historyText.ts";

const key = (
  k: string,
  mods: Partial<{ shiftKey: boolean; ctrlKey: boolean; metaKey: boolean; altKey: boolean }> = {},
) => ({ key: k, shiftKey: false, ctrlKey: false, metaKey: false, altKey: false, ...mods });

test("a notice stays 3 s, and 5 s above 70 characters", () => {
  assert.equal(noticeDurationMs("Undid: Move (3 paths)."), 3000);
  assert.equal(noticeDurationMs("x".repeat(70)), 3000);
  assert.equal(noticeDurationMs("x".repeat(71)), 5000);
});

test("Ctrl+Z undoes, Ctrl+Shift+Z redoes, by the typed character", () => {
  assert.equal(historyKeyDirection(key("z", { ctrlKey: true }), false), "undo");
  assert.equal(historyKeyDirection(key("Z", { ctrlKey: true, shiftKey: true }), false), "redo");
  // A QWERTZ keyboard types z where the US layout types y, and the other way round.
  assert.equal(historyKeyDirection(key("y", { ctrlKey: true }), false), "redo");
  assert.equal(historyKeyDirection(key("z", { metaKey: true }), true), "undo");
});

test("Ctrl+Y is no redo on macOS, and other combinations are no history key", () => {
  assert.equal(historyKeyDirection(key("y", { metaKey: true }), true), null);
  assert.equal(historyKeyDirection(key("y", { ctrlKey: true, shiftKey: true }), false), null);
  assert.equal(historyKeyDirection(key("z"), false), null);
  assert.equal(historyKeyDirection(key("z", { ctrlKey: true, altKey: true }), false), null);
  assert.equal(historyKeyDirection(key("x", { ctrlKey: true }), false), null);
});

test("platform names and small helpers", () => {
  assert.equal(isMac("MacIntel"), true);
  assert.equal(isMac("Linux x86_64"), false);
  assert.equal(workingNotice("undo"), "Undo: working...");
  assert.equal(workingNotice("redo"), "Redo: working...");
  assert.equal(noticeText(""), null);
  assert.equal(noticeText("Nothing to undo."), "Nothing to undo.");
  assert.deepEqual(menuNames("Move", ""), { undo: "Move", redo: null });
});
