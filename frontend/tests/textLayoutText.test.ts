// The words and keys of the text layout controls (`specs/0079-text-layout-controls` criteria 1,
// 7, 10, 13; UX notes §0.3, §0.4, §1, §6).

import assert from "node:assert/strict";
import { test } from "node:test";

import {
  ALIGN_CODES,
  ALIGN_NAMES,
  APPLY_TO_ALL_NAME,
  LAYOUT_FIELDS,
  LAYOUT_FIELD_LABELS,
  type LayoutFieldState,
  alignFocusMove,
  alignTabStop,
  alignTooltipLine,
  appliesToAll,
  entryAnswer,
  kernKey,
  layoutDefaultText,
  layoutResetLabel,
  layoutNotice,
  layoutTipValue,
  layoutUnit,
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
    "Horizontal kerning",
    "Vertical kerning",
    "Per-character rotation",
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

test("criteria 6, 26 and 27b: the block is there only with seven fields", () => {
  assert.equal(parseLayoutBlock("null"), null);
  assert.equal(parseLayoutBlock("not json"), null);
  const six = JSON.stringify(Array.from({ length: 6 }, () => field({})));
  assert.equal(parseLayoutBlock(six), null);
  const seven = JSON.stringify(Array.from({ length: 7 }, () => field({})));
  assert.equal(parseLayoutBlock(seven)?.length, 7);
});

test("criteria 27b and 27c: Per-character rotation in degrees with its own reset; All characters on the three", () => {
  assert.equal(LAYOUT_FIELD_LABELS["char-rotation"], "Per-character rotation");
  assert.equal(layoutUnit("char-rotation"), "°");
  assert.equal(layoutTipValue("char-rotation", field({ text: "30" })), "30°");
  assert.equal(layoutResetLabel("char-rotation", false), "Clear per-character rotation");
  assert.equal(layoutResetLabel("char-rotation", true), undefined);
  assert.equal(
    layoutNotice("char-rotation", { kind: "max", mm: 180 }),
    "Per-character rotation set to 180°, the largest for this size.",
  );
  assert.deepEqual(
    LAYOUT_FIELDS.filter(appliesToAll),
    ["horizontal-kerning", "vertical-kerning", "char-rotation"],
  );
  assert.equal(APPLY_TO_ALL_NAME, "Apply to all characters");
});

test("criterion 29: Alt+arrows kern, Shift for 1 mm; other keys do not", () => {
  assert.deepEqual(kernKey("ArrowLeft", true, false, false), [-1, 0, false]);
  assert.deepEqual(kernKey("ArrowUp", true, true, false), [0, 1, true]);
  assert.deepEqual(kernKey("ArrowDown", true, false, false), [0, -1, false]);
  assert.equal(kernKey("ArrowLeft", false, false, false), null);
  assert.equal(kernKey("ArrowLeft", true, false, true), null);
  assert.equal(kernKey("Home", true, false, false), null);
});

test("criterion 28: outside edit mode a kerning reset is named Clear kerning", () => {
  assert.equal(layoutResetLabel("horizontal-kerning", false), "Clear kerning");
  assert.equal(layoutResetLabel("vertical-kerning", false), "Clear kerning");
  assert.equal(layoutResetLabel("horizontal-kerning", true), undefined);
  assert.equal(layoutResetLabel("letter-spacing", false), undefined);
});
