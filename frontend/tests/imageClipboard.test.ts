// The page rules of the image clipboard (`specs/0087-image-clipboard-and-drop` criteria 4, 5, 6,
// 13, 15, 16).

import assert from "node:assert/strict";
import { test } from "node:test";

import {
  type ClipboardFile,
  copyFailureReason,
  isCopyChord,
  pastedName,
  pickPasted,
} from "../src/lib/imageClipboard.ts";

const press = (key: string, extra: Partial<Parameters<typeof isCopyChord>[0]> = {}) => ({
  key,
  ctrlKey: false,
  metaKey: false,
  shiftKey: false,
  altKey: false,
  repeat: false,
  ...extra,
});

test("Ctrl+C is the copy chord, Cmd+C on macOS, once per press, never with Shift or Alt", () => {
  assert.equal(isCopyChord(press("c", { ctrlKey: true }), false), true);
  assert.equal(isCopyChord(press("C", { ctrlKey: true }), false), true);
  assert.equal(isCopyChord(press("c", { metaKey: true }), true), true);
  assert.equal(isCopyChord(press("c", { ctrlKey: true }), true), false);
  assert.equal(isCopyChord(press("c", { ctrlKey: true, shiftKey: true }), false), false);
  assert.equal(isCopyChord(press("c", { ctrlKey: true, altKey: true }), false), false);
  assert.equal(isCopyChord(press("c", { ctrlKey: true, repeat: true }), false), false);
  assert.equal(isCopyChord(press("v", { ctrlKey: true }), false), false);
});

test("a clipboard bitmap's placeholder name gives way to the session's name; a file keeps its own", () => {
  assert.equal(pastedName("image.png"), "");
  assert.equal(pastedName("Image.PNG"), "");
  assert.equal(pastedName("holiday.jpg"), "holiday.jpg");
});

test("the paste takes the best ranked file, the first of equals, and nothing for rank 0", () => {
  const file = (name: string): ClipboardFile => ({ name, type: "", bytes: new Uint8Array() });
  const ranks: Record<string, number> = { text: 0, gif: 1, png: 2, jpg: 2 };
  const rank = (f: ClipboardFile) => ranks[f.name];
  assert.equal(pickPasted([file("text"), file("gif"), file("png"), file("jpg")], rank)?.name, "png");
  assert.equal(pickPasted([file("text"), file("gif")], rank)?.name, "gif");
  assert.equal(pickPasted([file("text")], rank), null);
  assert.equal(pickPasted([], rank), null);
});

test("a failed copy names the error's message, else its name", () => {
  assert.equal(copyFailureReason(new Error("Write permission denied.")), "Write permission denied.");
  assert.equal(copyFailureReason({ name: "NotAllowedError", message: "" }), "NotAllowedError");
  assert.equal(copyFailureReason("refused"), "refused");
});
