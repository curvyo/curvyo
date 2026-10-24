// Tester cases for `specs/0079-text-layout-controls` slice 3 on the frontend text rules: the two
// groups' names and order (criterion 20) and the greyed orientation with its tooltip line
// (criterion 21).

import assert from "node:assert/strict";
import { test } from "node:test";

import {
  ORIENTATION_CODES,
  ORIENTATION_NAMES,
  WRITING_MODE_CODES,
  WRITING_MODE_NAMES,
  orientationGreyed,
  parseWritingMode,
  writingTooltipLine,
} from "../src/lib/textWritingText.ts";

test("criterion 20: the groups' accessible names in the spec's order", () => {
  assert.deepEqual(
    WRITING_MODE_CODES.map((c) => WRITING_MODE_NAMES[c]),
    ["Horizontal", "Vertical, right to left", "Vertical, left to right"],
  );
  assert.deepEqual(
    ORIENTATION_CODES.map((c) => ORIENTATION_NAMES[c]),
    ["Auto orientation", "Upright", "Sideways"],
  );
});

test("criterion 21: orientation greyed only while every subject text is horizontal", () => {
  assert.equal(orientationGreyed(parseWritingMode("horizontal")), true);
  assert.equal(orientationGreyed(parseWritingMode("vertical-rl")), false);
  assert.equal(orientationGreyed(parseWritingMode("vertical-lr")), false);
  assert.equal(orientationGreyed(parseWritingMode("")), false, "mixed modes act");
  assert.equal(writingTooltipLine("auto", true), "Only for vertical text");
  assert.equal(writingTooltipLine(null, true), "Only for vertical text");
  assert.equal(writingTooltipLine("upright", false), null);
});
