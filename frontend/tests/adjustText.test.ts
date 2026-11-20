// The pure rules of the Adjustments group (`specs/0135-image-optimisation` criteria 2, 3, 26).
import { test } from "node:test";
import assert from "node:assert/strict";
import {
  bandBreak,
  clampStrength,
  dropSlots,
  fieldStep,
  insertedAt,
  landingIndex,
  openAfterInsert,
  openAfterMove,
  openAfterRemove,
  parseStrength,
  strengthAfterKey,
  strengthAtPosition,
  strengthPosition,
} from "../src/lib/adjustText.ts";
import type { StepRow } from "../src/lib/adjustmentsView.ts";

function row(kind: StepRow["kind"]): StepRow {
  return {
    index: 0,
    kind,
    name: kind,
    enabled: true,
    presetInUse: false,
    unknown: kind === "unknown",
    line: "",
    fields: [],
    up: true,
    down: true,
    accessibleName: kind,
  };
}

test("criterion 26: the keys of the Strength slider", () => {
  assert.equal(strengthAfterKey(50, "ArrowRight"), 51);
  assert.equal(strengthAfterKey(50, "ArrowUp"), 51);
  assert.equal(strengthAfterKey(50, "ArrowLeft"), 49);
  assert.equal(strengthAfterKey(50, "ArrowDown"), 49);
  assert.equal(strengthAfterKey(50, "PageUp"), 60);
  assert.equal(strengthAfterKey(95, "PageUp"), 100, "held to 100");
  assert.equal(strengthAfterKey(3, "PageDown"), 0, "held to 0");
  assert.equal(strengthAfterKey(50, "Home"), 0);
  assert.equal(strengthAfterKey(50, "End"), 100);
  assert.equal(strengthAfterKey(50, "a"), null);
});

test("the slider's track maps a position to a whole percent and back", () => {
  assert.equal(strengthAtPosition(0, 130), 0);
  assert.equal(strengthAtPosition(65, 130), 50);
  assert.equal(strengthAtPosition(500, 130), 100);
  assert.equal(strengthAtPosition(Number.NaN, 130), 0);
  assert.equal(strengthPosition(50, 130), 65);
  assert.equal(clampStrength(49.6), 50);
});

test("a typed Strength is a whole number from 0 to 100", () => {
  assert.equal(parseStrength("80"), 80);
  assert.equal(parseStrength(" 80 % "), 80);
  assert.equal(parseStrength("100"), 100);
  assert.equal(parseStrength("120"), null);
  assert.equal(parseStrength("-1"), null);
  assert.equal(parseStrength("5.5"), null);
  assert.equal(parseStrength(""), null);
});

test("criterion 3: a drag lands inside the source row's band only", () => {
  const rows = [row("denoise"), row("levels"), row("sharpen")];
  assert.deepEqual(dropSlots(rows, 1), { first: 0, last: 3 });
  assert.equal(landingIndex(0, 3), 2, "below the last row: the end");
  assert.equal(landingIndex(2, 0), 0);
  assert.equal(landingIndex(1, 1), 1, "its own place");
  assert.equal(landingIndex(1, 2), 1, "its own place from below");
  assert.equal(dropSlots([row("unknown")], 0), null, "an unknown row does not drag");
  const mixed = [row("levels"), row("unknown"), row("sharpen")];
  assert.deepEqual(dropSlots(mixed, 0), { first: 0, last: 1 });
  assert.equal(bandBreak(mixed, 1), true);
  assert.equal(bandBreak(mixed, 0), false);
  assert.equal(bandBreak(rows, 1), false);
});

test("the open rows follow a move, a remove and an add", () => {
  assert.deepEqual([...openAfterMove(new Set([1]), 1, 2)], [2]);
  assert.deepEqual([...openAfterMove(new Set([2]), 1, 2)].sort(), [1]);
  assert.deepEqual([...openAfterMove(new Set([0, 3]), 3, 1)].sort(), [0, 1]);
  assert.deepEqual([...openAfterRemove(new Set([0, 1, 2]), 1)].sort(), [0, 1]);
  assert.deepEqual([...openAfterInsert(new Set([0, 1]), 1)].sort(), [0, 1, 2]);
});

test("the inserted step is found by its place in the new list", () => {
  assert.equal(insertedAt(["levels", "sharpen"], ["levels", "contours", "sharpen"]), 1);
  assert.equal(insertedAt(["levels"], ["levels", "levels"]), 1);
  assert.equal(insertedAt([], ["levels"]), 0);
  assert.equal(insertedAt(["levels"], ["levels"]), null);
});

test("the field steps follow the decimals", () => {
  assert.equal(fieldStep(0)("37", "ArrowUp", false), "38");
  assert.equal(fieldStep(2)("0.88", "ArrowDown", false), "0.87");
  assert.equal(fieldStep(2)("0.88", "ArrowUp", true), "0.98");
  assert.equal(fieldStep(0)("x", "ArrowUp", false), null);
  assert.equal(fieldStep(0)("5", "a", false), null);
});
