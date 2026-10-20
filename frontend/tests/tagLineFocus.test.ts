// The 0075 UX re-review's focus fixes: the add field returns the focus to Add tag after a key
// (or when the earlier element is gone); a removed chip's place goes to the next tag, shown or
// not.
import assert from "node:assert/strict";
import { test } from "node:test";

import { addFieldReturn, tagAfterRemove } from "../src/lib/tagLineFocus.ts";

test("the add field gives the focus back to Add tag after a key or when the element is gone", () => {
  const canvas = { isConnected: true };
  assert.equal(addFieldReturn(true, canvas), "add-tag");
  assert.equal(addFieldReturn(false, canvas), canvas);
  assert.equal(addFieldReturn(false, { isConnected: false }), "add-tag");
  assert.equal(addFieldReturn(false, null), "add-tag");
});


test("the next tag takes a removed chip's place, also from +N", () => {
  assert.equal(tagAfterRemove(["Cuttable", "Script", "Wedding"], "Script"), "Wedding");
  assert.equal(tagAfterRemove(["Cuttable", "Script"], "Script"), null);
});
