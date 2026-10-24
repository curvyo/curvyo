// The words and keys of the text layout controls (`specs/0079-text-layout-controls` criteria 1,
// 7, 10, 13; UX notes §0.3, §0.4, §1, §6).

import assert from "node:assert/strict";
import { test } from "node:test";

import {
  ALIGN_CODES,
  ALIGN_NAMES,
  LAYOUT_FIELD_LABELS,
  type LayoutFieldState,
  alignFocusMove,
  alignTabStop,
  alignTooltipLine,
  entryAnswer,
  layoutDefaultText,
  layoutNotice,
  layoutTipValue,
  layoutValueText,
  parseAlign,
  parseLayoutBlock,
} from "../src/lib/textLayoutText.ts";

const field = (over: Partial<LayoutFieldState>): LayoutFieldState => ({
  text: "1.5",
  value: 1.5,
  mixed: false,
  auto: false,
  barFrom: 0.2,
  barTo: 0.32,
  zero: 0.2,
  dragBase: 0.32,
  dragMin: -0.2,
  dragLimit: 2.2,
  trackMin: -2.5,
  trackMax: 10,
  resettable: true,
  ...over,
});

test("criterion 1: four buttons in order with their names, one Tab stop", () => {
  assert.deepEqual(
    ALIGN_CODES.map((code) => ALIGN_NAMES[code]),
    ["Align left", "Align centre", "Align right", "Justify"],
  );
  assert.equal(alignTabStop("right"), 2);
  assert.equal(alignTabStop(null), 0, "mixed: Align left holds the stop");
  assert.equal(parseAlign(""), null);
  assert.equal(parseAlign("justify"), "justify");
  assert.equal(alignTooltipLine(null), "Mixed");
  assert.equal(alignTooltipLine("left"), null);
});

test("UX notes §1: arrows, Home and End move focus without wrapping", () => {
  assert.equal(alignFocusMove("ArrowRight", 3), 3);
  assert.equal(alignFocusMove("ArrowLeft", 0), 0);
  assert.equal(alignFocusMove("ArrowDown", 1), 2);
  assert.equal(alignFocusMove("End", 0), 3);
  assert.equal(alignFocusMove("Enter", 1), null);
});

test("criterion 7: the field names, the defaults and the readouts", () => {
  assert.deepEqual(Object.values(LAYOUT_FIELD_LABELS), [
    "Letter spacing",
    "Word spacing",
    "Line spacing",
    "Character rotation",
  ]);
  assert.equal(layoutDefaultText("line-spacing"), "Auto");
  assert.equal(layoutDefaultText("character-rotation"), "0°");
  assert.equal(layoutValueText("letter-spacing", field({})), "1.5 millimetres");
  assert.equal(layoutTipValue("letter-spacing", field({})), "1.5 mm");
  assert.equal(layoutTipValue("character-rotation", field({ text: "−12" })), "−12°");
  assert.equal(layoutValueText("character-rotation", field({ text: "−12" })), "−12 degrees");
});

test("criterion 10 and UX notes §0.3: Auto reads as automatic", () => {
  const auto = field({ text: "12.4", auto: true });
  assert.equal(layoutValueText("line-spacing", auto), "12.4 millimetres, automatic");
  assert.equal(layoutTipValue("line-spacing", auto), "12.4 mm (Auto)");
});

test("criterion 13: Mixed", () => {
  assert.equal(layoutValueText("word-spacing", field({ mixed: true })), "Mixed");
  assert.equal(layoutTipValue("word-spacing", field({ mixed: true })), "Mixed");
});

test("criterion 7 and UX notes §0.4: the notices fit 70 characters", () => {
  assert.equal(
    layoutNotice("letter-spacing", { kind: "max", mm: 40 }),
    "Letter spacing set to 40 mm, the largest for this size.",
  );
  assert.equal(
    layoutNotice("line-spacing", { kind: "min", mm: 0 }),
    "Line spacing set to 0 mm, the smallest for this size.",
  );
  const limited = layoutNotice("character-rotation", { kind: "limited", limited: 12, of: 30 });
  assert.ok(limited && limited.length <= 70, limited ?? "");
  assert.equal(
    layoutNotice("letter-spacing", { kind: "limited", limited: 1, of: 3 }),
    "Letter spacing limited on 1 of 3 texts, to fit its size.",
  );
  assert.equal(layoutNotice("letter-spacing", { kind: "set" }), null);
  assert.equal(entryAnswer({ kind: "invalid" }), "invalid:number");
  assert.equal(entryAnswer({ kind: "max", mm: 40 }), "committed");
});

test("criterion 6: the block is there only with four fields", () => {
  assert.equal(parseLayoutBlock("null"), null);
  assert.equal(parseLayoutBlock("not json"), null);
  const four = JSON.stringify([field({}), field({}), field({}), field({})]);
  assert.equal(parseLayoutBlock(four)?.length, 4);
});
