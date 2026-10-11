import assert from "node:assert/strict";
import test from "node:test";

import {
  type HistoryRowData,
  canRestoreScroll,
  choiceAfterSelection,
  effectiveScope,
  leadWords,
  objectButtonTooltip,
  objectWipeQuestion,
  objectWipedNotice,
  parseObjectInfo,
  rowDisplay,
  rowTooltip,
  secondLine,
  statusWord,
  wipeConfirmKey,
} from "../src/lib/historyRows.ts";
import {
  historyKeyDirection,
  isObjectDirection,
  noticeDurationMs,
  workingNotice,
} from "../src/lib/historyText.ts";

const row = (over: Partial<HistoryRowData> = {}): HistoryRowData => ({
  id: "a1b2c3d",
  step: "7@3",
  label: "translate_objects",
  name: "Move",
  known: true,
  author: "you",
  time: new Date(2026, 9, 10, 12, 3, 41).getTime() / 1000,
  kind: "change",
  summary: "",
  status: "applied",
  objects: [],
  ...over,
});

const today = new Date(2026, 9, 10, 15, 0, 0);

const key = (
  k: string,
  mods: Partial<{ shiftKey: boolean; ctrlKey: boolean; metaKey: boolean; altKey: boolean }> = {},
) => ({ key: k, shiftKey: false, ctrlKey: false, metaKey: false, altKey: false, ...mods });

test("Ctrl+U and Ctrl+Shift+U are the object keys, by the typed character", () => {
  assert.equal(historyKeyDirection(key("u", { ctrlKey: true }), false), "object-undo");
  assert.equal(
    historyKeyDirection(key("U", { ctrlKey: true, shiftKey: true }), false),
    "object-redo",
  );
  assert.equal(historyKeyDirection(key("u", { metaKey: true }), true), "object-undo");
  assert.equal(historyKeyDirection(key("u"), false), null);
  assert.equal(historyKeyDirection(key("u", { ctrlKey: true, altKey: true }), false), null);
  assert.ok(isObjectDirection("object-undo") && !isObjectDirection("undo"));
  assert.equal(workingNotice("object-redo"), "Redo: working...");
});

test("a step taken back by an object undo says by whom", () => {
  assert.equal(statusWord(row({ status: "undone-by", undoneBy: "you" })), "Undone by You");
  assert.equal(
    statusWord(row({ status: "undone-by", undoneBy: "earlier" })),
    "Undone by Earlier session",
  );
  assert.equal(statusWord(row({ status: "undone" })), "Undone");
  assert.equal(
    secondLine(row({ status: "undone-by", undoneBy: "you", summary: "with 2 other objects" }), today),
    "Undone by You · 12:03 · with 2 other objects",
  );
});

test("an inherited row starts its second line with where it came from", () => {
  const inherited = row({ from: "3f9a1c", summary: "" });
  assert.equal(leadWords(inherited), "from 3f9a1c");
  assert.equal(secondLine(inherited, today), "from 3f9a1c · You · 12:03");
  assert.match(rowTooltip(inherited), /Taken over from 3f9a1c/);
  assert.match(rowTooltip(row({ sources: ["aaa", "bbb"] })), /Sources: aaa, bbb/);
});

test("the scope exists with one selected object and a press on Document is kept until it ends", () => {
  assert.equal(effectiveScope(1, "object"), "object");
  assert.equal(effectiveScope(1, "document"), "document");
  assert.equal(effectiveScope(0, "object"), "document");
  assert.equal(effectiveScope(3, "object"), "document");
  assert.equal(choiceAfterSelection(1, "document"), "document");
  assert.equal(choiceAfterSelection(0, "document"), "object");
  assert.equal(choiceAfterSelection(2, "document"), "object");
});

test("the Object scope reads its subject and names the steps of its buttons", () => {
  assert.equal(parseObjectInfo(""), null);
  const info = parseObjectInfo(
    JSON.stringify({
      key: "k",
      kind: "Rectangle",
      id: "3f9a1c",
      fullId: "3f9a1cabcdefg",
      undo: { id: "a1b2c3d", name: "Resize", author: "you", time: row().time },
      redo: null,
      wipeCount: 12,
      total: 13,
    }),
  );
  assert.ok(info !== null && info.undo !== null);
  assert.equal(objectButtonTooltip("Undo", info.undo, "Ctrl+U", today), "Undo: Resize, You, 12:03 (Ctrl+U)");
  assert.equal(objectButtonTooltip("Redo", info.undo, "Ctrl+Shift+U", today), "Redo: Resize (Ctrl+Shift+U)");
});

test("the wipe of one object says it hides and the file does not shrink", () => {
  assert.equal(
    objectWipeQuestion(12),
    "Hide the 12 earlier steps of this object? They stay in the document history. The file does not get smaller.",
  );
  assert.match(objectWipeQuestion(1), /the 1 earlier step of/);
  assert.equal(objectWipedNotice(12), "Hid 12 earlier steps of this object.");
  assert.equal(objectWipedNotice(1), "Hid 1 earlier step of this object.");
});

test("the notices of the object keys stay 3 s up to 70 characters and 5 s above", () => {
  const creation = "Undid: Draw rectangle. The object is gone. Ctrl+Z brings it back.";
  assert.ok(creation.length <= 70);
  assert.equal(noticeDurationMs(creation), 3000);
  const passed =
    "Undid: Draw rectangle. The object is gone. Ctrl+Z brings it back. Skipped 2 steps that only moved or reshaped the original.";
  assert.equal(noticeDurationMs(passed), 5000);
  assert.equal(noticeDurationMs("Nothing to undo for this object."), 3000);
});

test("an open wipe confirm closes when the selection moves to another object with as many steps", () => {
  const first = { selectionCount: 1, subjectKey: "copy-a", total: 3, scope: "object" } as const;
  const second = { ...first, subjectKey: "copy-b" };
  assert.notEqual(wipeConfirmKey(first), wipeConfirmKey(second));
  assert.notEqual(wipeConfirmKey(first), wipeConfirmKey({ ...first, total: 4 }));
  assert.notEqual(wipeConfirmKey(first), wipeConfirmKey({ ...first, scope: "document" }));
  assert.notEqual(wipeConfirmKey(first), wipeConfirmKey({ ...first, selectionCount: 2 }));
  assert.equal(wipeConfirmKey(first), wipeConfirmKey({ ...first }));
});

test("the wipe of one object is titled without its object so the id is not cut", () => {
  const wipe = row({
    label: "wipe_object_history",
    name: "Wipe history of Rectangle, 8qdzay",
    summary: "",
  });
  assert.deepEqual(rowDisplay(wipe), { name: "Wipe history", summary: "Rectangle 8qdzay" });
  assert.equal(secondLine(wipe, today), "Rectangle 8qdzay · You · 12:03");
  assert.deepEqual(rowDisplay(row()), { name: "Move", summary: "" });
});

test("a row taken back by its own author's object undo says it once and keeps author and time", () => {
  const taken = row({ status: "undone-by", undoneBy: "you", summary: "with 2 other objects" });
  assert.equal(secondLine(taken, today), "Undone by You · 12:03 · with 2 other objects");
  assert.match(rowTooltip(taken), /Undone by You\./);
});

test("a kept scroll position waits for the rows of the wanted scope", () => {
  const base = { pending: 500, total: 26, loadedFor: "object:k", wanted: "document:k" };
  // The list still holds the scope just left: restoring now would clamp to the top.
  assert.equal(canRestoreScroll(base), false);
  assert.equal(canRestoreScroll({ ...base, loadedFor: "document:k" }), true);
  assert.equal(canRestoreScroll({ ...base, loadedFor: "document:k", total: 0 }), false);
  assert.equal(canRestoreScroll({ ...base, loadedFor: "document:k", pending: null }), false);
});
