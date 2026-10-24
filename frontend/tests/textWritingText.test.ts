// The words and keys of the Writing mode and Glyph orientation groups
// (`specs/0079-text-layout-controls` criteria 20, 21).
import assert from "node:assert/strict";
import { test } from "node:test";

import {
  ORIENTATION_NAMES,
  WRITING_MODE_NAMES,
  choiceFocusMove,
  orientationGreyed,
  parseOrientation,
  parseWritingMode,
  writingTooltipLine,
} from "../src/lib/textWritingText.ts";

test("criterion 20: the buttons' names in order", () => {
  assert.deepEqual(Object.values(WRITING_MODE_NAMES), [
    "Horizontal",
    "Vertical, right to left",
    "Vertical, left to right",
  ]);
  assert.deepEqual(Object.values(ORIENTATION_NAMES), ["Auto orientation", "Upright", "Sideways"]);
});

test("the session's codes, empty for mixed", () => {
  assert.equal(parseWritingMode("vertical-rl"), "vertical-rl");
  assert.equal(parseWritingMode(""), null);
  assert.equal(parseOrientation("sideways"), "sideways");
  assert.equal(parseOrientation("diagonal"), null);
});

test("criterion 21: orientation greyed only while horizontal, with its tooltip line", () => {
  assert.equal(orientationGreyed("horizontal"), true);
  assert.equal(orientationGreyed("vertical-lr"), false);
  assert.equal(orientationGreyed(null), false, "mixed modes: some text is vertical");
  assert.equal(writingTooltipLine("auto", true), "Only for vertical text");
  assert.equal(writingTooltipLine(null, false), "Mixed");
  assert.equal(writingTooltipLine("auto", false), null);
});

test("focus moves without wrap", () => {
  assert.equal(choiceFocusMove("ArrowLeft", 0, 3), 0);
  assert.equal(choiceFocusMove("ArrowRight", 2, 3), 2);
  assert.equal(choiceFocusMove("End", 0, 3), 2);
  assert.equal(choiceFocusMove("Enter", 1, 3), null);
});
