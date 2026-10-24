// Tester cases for `specs/0079-text-layout-controls` slice 1 on the frontend text rules: the
// Alignment group's names, order and keys (criterion 1), the fields' labels and value texts
// (criteria 7, 10, 13) and the notices with their 70-character cap (Terms, criterion 7, 13).

import assert from "node:assert/strict";
import { test } from "node:test";

import {
  ALIGN_CODES,
  ALIGN_NAMES,
  LAYOUT_FIELDS,
  LAYOUT_FIELD_LABELS,
  type LayoutFieldState,
  alignFocusMove,
  alignTabStop,
  alignTooltipLine,
  layoutNotice,
  layoutTipValue,
  layoutValueText,
  parseAlign,
  parseLayoutBlock,
  parseLayoutOutcome,
} from "../src/lib/textLayoutText.ts";

const state = (over: Partial<LayoutFieldState>): LayoutFieldState => ({
  text: "0",
  value: 0,
  mixed: false,
  auto: false,
  barFrom: 0,
  barTo: 0,
  zero: null,
  dragBase: 0,
  dragMin: 0,
  dragLimit: 1,
  trackMin: 0,
  trackMax: 1,
  resettable: false,
  ...over,
});

test("criterion 1: four alignments in order with their accessible names", () => {
  assert.deepEqual(
    ALIGN_CODES.map((c) => ALIGN_NAMES[c]),
    ["Align left", "Align centre", "Align right", "Justify"],
  );
  assert.equal(parseAlign(""), null, "differing texts: none pressed");
  assert.equal(parseAlign("centre"), "centre");
  assert.equal(parseAlign("center"), null);
});

test("criterion 1: one Tab stop on the pressed button, Align left when none; arrows do not wrap", () => {
  assert.equal(alignTabStop(null), 0);
  assert.equal(alignTabStop("justify"), 3);
  assert.equal(alignFocusMove("ArrowLeft", 0), 0);
  assert.equal(alignFocusMove("ArrowRight", 3), 3);
  assert.equal(alignFocusMove("ArrowDown", 1), 2);
  assert.equal(alignFocusMove("Home", 2), 0);
  assert.equal(alignFocusMove("End", 0), 3);
  assert.equal(alignFocusMove("Enter", 1), null, "Enter presses, it is not a focus move");
  assert.equal(alignTooltipLine(null), "Mixed");
  assert.equal(alignTooltipLine("left"), null);
});

test("criterion 7: the four field labels in the block's order", () => {
  assert.deepEqual(
    LAYOUT_FIELDS.map((f) => LAYOUT_FIELD_LABELS[f]),
    ["Letter spacing", "Word spacing", "Line spacing", "Character rotation"],
  );
});

test("criteria 10 and 13: value texts for Auto, a set value, a rotation and Mixed", () => {
  const auto = state({ text: "12.4", value: 12.4, auto: true });
  assert.equal(layoutValueText("line-spacing", auto), "12.4 millimetres, automatic");
  assert.equal(layoutTipValue("line-spacing", auto), "12.4 mm (Auto)");
  assert.equal(layoutValueText("letter-spacing", state({ text: "1.5" })), "1.5 millimetres");
  assert.equal(layoutValueText("character-rotation", state({ text: "−12" })), "−12 degrees");
  assert.equal(layoutTipValue("character-rotation", state({ text: "−12" })), "−12°");
  for (const f of LAYOUT_FIELDS) {
    assert.equal(layoutValueText(f, state({ mixed: true, text: "" })), "Mixed");
    assert.equal(layoutTipValue(f, state({ mixed: true, text: "" })), "Mixed");
  }
});

test("criteria 7 and 13: the notices, and every notice stays within 70 characters", () => {
  assert.equal(
    layoutNotice("letter-spacing", { kind: "max", mm: 40 }),
    "Letter spacing set to 40 mm, the largest for this size.",
  );
  assert.equal(
    layoutNotice("letter-spacing", { kind: "limited", limited: 1, of: 3 }),
    "Letter spacing limited on 1 of 3 texts, to fit its size.",
  );
  assert.equal(
    layoutNotice("word-spacing", { kind: "min", mm: -12.5 }),
    "Word spacing set to −12.5 mm, the smallest for this size.",
  );
  assert.equal(layoutNotice("letter-spacing", { kind: "set" }), null);
  for (const f of LAYOUT_FIELDS) {
    for (const o of [
      { kind: "max", mm: 20000 },
      { kind: "min", mm: -10000.55 },
      { kind: "limited", limited: 9999, of: 10000 },
    ] as const) {
      const n = layoutNotice(f, o);
      assert.ok(n !== null && n.length <= 70, `${n} (${n?.length})`);
    }
  }
});

test("criterion 6: the block JSON is four fields or no block; garbage is no block", () => {
  assert.equal(parseLayoutBlock("null"), null);
  assert.equal(parseLayoutBlock("[]"), null);
  assert.equal(parseLayoutBlock("{"), null);
  assert.equal(parseLayoutBlock(JSON.stringify([state({}), state({})])), null);
  assert.equal(parseLayoutBlock(JSON.stringify(LAYOUT_FIELDS.map(() => state({}))))?.length, 4);
  assert.deepEqual(parseLayoutOutcome("not json"), { kind: "unchanged" });
});
