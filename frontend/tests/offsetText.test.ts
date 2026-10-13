// Run with `npm test`. The words of `src/lib/offsetText.ts` against `specs/0038-path-offset`
// (criteria 12, 15, 24, 29, 32).

import assert from "node:assert/strict";
import { test } from "node:test";

import {
  OFFSET_IDLE,
  type OffsetOutcome,
  formatDistance,
  offsetReadout,
  offsetRefusalText,
  offsetSuccessText,
  chipDistanceText,
  clampChipDistance,
  cornerStyleLines,
  keepsCornerStyleChip,
  parseChipDistance,
  stepChipDistance,
} from "../src/lib/offsetText.ts";

const outcome = (fields: Partial<OffsetOutcome>): OffsetOutcome => ({
  kind: "applied",
  added: 0,
  changed: 0,
  count: 0,
  of: 1,
  vanished: 0,
  distanceMm: 2.5,
  direction: "out",
  ...fields,
});

test("criterion 24: the notice names what was added or changed, the distance and the side", () => {
  assert.equal(
    offsetSuccessText(outcome({ added: 1 })),
    "Offset: added 1 shape, 2.5 mm outward.",
  );
  assert.equal(
    offsetSuccessText(outcome({ changed: 1, direction: "in" })),
    "Offset: changed 1 shape, 2.5 mm inward.",
  );
  assert.equal(
    offsetSuccessText(outcome({ changed: 1, added: 1, direction: "both" })),
    "Offset: changed 1 shape and added 1, 2.5 mm both sides.",
  );
  assert.equal(
    offsetSuccessText(outcome({ added: 2, of: 3, vanished: 1, direction: "in" })),
    "Offset: added 2 shapes, 2.5 mm inward. 1 of 3 shapes vanished.",
  );
  assert.equal(offsetSuccessText(outcome({ kind: "ignored" })), null);
});

test("criteria 29, 32: the refusals", () => {
  assert.equal(
    offsetRefusalText(outcome({ kind: "empty", distanceMm: 10 })),
    "Inset is empty: nothing is left at 10 mm. Nothing was changed.",
  );
  assert.equal(
    offsetRefusalText(outcome({ kind: "groups", count: 2 })),
    "Offset does not work on groups. Ungroup first. Nothing was changed.",
  );
  assert.equal(offsetRefusalText(outcome({ kind: "applied" })), null);
});

test("criteria 12, 15, 18: the readout names side, distance and corner style, then the mode", () => {
  const dragging = { ...OFFSET_IDLE, operands: 1, dragging: true };
  assert.deepEqual(offsetReadout({ ...dragging, distanceMm: 2.5 }), {
    lines: ["Outset 2.5 mm · Round", "New object"],
    invalid: false,
  });
  assert.deepEqual(offsetReadout({ ...dragging, distanceMm: -0.8, change: true, dragJoin: "miter" }).lines, [
    "Inset 0.8 mm · Miter",
    "Changes the original",
  ]);
  assert.deepEqual(
    offsetReadout({ ...dragging, distanceMm: 2.5, both: true, change: true, dragJoin: "bevel" }).lines,
    ["Both ±2.5 mm · Bevel", "Changes the original, adds the inner copy"],
  );
  assert.deepEqual(offsetReadout({ ...dragging, distanceMm: -2.5, both: true, change: true }).lines, [
    "Both ±2.5 mm · Round",
    "Changes the original, adds the outer copy",
  ]);
  const stopped = offsetReadout({ ...dragging, distanceMm: -1, stop: "nothing_left" });
  assert.deepEqual(stopped.lines, ["Inset 1 mm · Round", "New object", "Nothing is left"]);
  assert.equal(stopped.invalid, true);
  assert.equal(offsetReadout({ ...dragging, distanceMm: -1, vanished: 1, of: 3 }).lines[2], "1 of 3 shapes vanishes");
  assert.equal(offsetReadout({ ...dragging, large: true }).lines[2], "Preview off: large selection");
  for (const line of Object.values(offsetReadout({ ...dragging, distanceMm: 2 }).lines)) {
    assert.ok(!/Stays|Becomes/.test(line), "no result-kind hint (criterion 15)");
  }
});

test("criterion 56: the corner style chip after a double-click", () => {
  assert.deepEqual(cornerStyleLines("miter"), ["Miter corners", "For the next drag"]);
  assert.deepEqual(cornerStyleLines("round"), ["Round corners", "For the next drag"]);
});

test("criterion 56: the corner style chip survives its own release and goes off the handle", () => {
  assert.equal(keepsCornerStyleChip(true, "offset-corner", false), true);
  assert.equal(keepsCornerStyleChip(true, "offset-corner-change", false), true);
  assert.equal(keepsCornerStyleChip(true, "", false), false, "left the handle");
  assert.equal(keepsCornerStyleChip(true, "offset-corner", true), false, "a drag readout");
  assert.equal(keepsCornerStyleChip(false, "offset-corner", false), false);
});

test("criteria 6, 25, 54: the chip's Distance text, steps and parsing", () => {
  assert.equal(chipDistanceText(1), "1");
  assert.equal(chipDistanceText(2.5), "2.5");
  assert.equal(chipDistanceText(0.15), "0.15");
  assert.equal(chipDistanceText(0.1 + 0.2), "0.3");
  assert.equal(parseChipDistance("2,5"), 2.5);
  assert.equal(parseChipDistance(" 3 "), 3);
  assert.ok(Number.isNaN(parseChipDistance("")));
  assert.ok(Number.isNaN(parseChipDistance("abc")));
  assert.ok(Number.isNaN(parseChipDistance("2mm")));
  assert.equal(stepChipDistance(1, 1, {}), 1.1);
  assert.equal(stepChipDistance(1, -1, { shift: true }), 0.01, "clamped to 0.01");
  assert.equal(stepChipDistance(2.53, 1, {}), 2.6, "rounded to the step");
  assert.equal(stepChipDistance(2.5, 1, { ctrl: true }), 2.51);
  assert.equal(stepChipDistance(999.5, 1, { shift: true }), 1000);
  assert.equal(chipDistanceText(clampChipDistance(1500)), "1000");
  assert.equal(chipDistanceText(clampChipDistance(0.001)), "0.01");
  assert.equal(clampChipDistance(2.5), 2.5);
});

test("distances print without float noise", () => {
  assert.equal(formatDistance(0.1 + 0.2), "0.3");
  assert.equal(formatDistance(-2.5), "2.5");
});
