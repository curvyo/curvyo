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
  chipDistanceText,
  clampChipDistance,
  offsetKeyOutcome,
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
  // Third round: the default Mode is Change original (OFFSET_IDLE), Ctrl makes new objects.
  assert.deepEqual(offsetReadout({ ...dragging, distanceMm: 2.5 }), {
    lines: ["Outset 2.5 mm · Round", "Changes the original"],
    invalid: false,
  });
  assert.deepEqual(offsetReadout({ ...dragging, distanceMm: -0.8, change: false, dragJoin: "miter" }).lines, [
    "Inset 0.8 mm · Miter",
    "New object",
  ]);
  for (const distanceMm of [2.5, -2.5]) {
    assert.deepEqual(
      offsetReadout({ ...dragging, distanceMm, both: true, dragJoin: "bevel" }).lines,
      ["Both ±2.5 mm · Bevel", "Changes the original, adds the inner copy"],
      "Shift: the original takes the outset whatever the drag direction",
    );
  }
  assert.deepEqual(offsetReadout({ ...dragging, distanceMm: 2.5, both: true, change: false }).lines, [
    "Both ±2.5 mm · Round",
    "Two new objects",
  ]);
  const stopped = offsetReadout({ ...dragging, distanceMm: -1, stop: "nothing_left" });
  assert.deepEqual(stopped.lines, ["Inset 1 mm · Round", "Changes the original", "Nothing is left"]);
  assert.equal(stopped.invalid, true);
  assert.equal(offsetReadout({ ...dragging, distanceMm: -1, vanished: 1, of: 3 }).lines[2], "1 of 3 shapes vanishes");
  assert.equal(offsetReadout({ ...dragging, large: true }).lines[2], "Preview off: large selection");
  for (const line of Object.values(offsetReadout({ ...dragging, distanceMm: 2 }).lines)) {
    assert.ok(!/Stays|Becomes/.test(line), "no result-kind hint (criterion 15)");
  }
});

test("criterion 13: Alt during a drag is prevented and refreshes the readout, ignored keys are not", () => {
  assert.deepEqual(offsetKeyOutcome("offset-corner-style"), { ignored: false, cornerStyle: true });
  assert.deepEqual(offsetKeyOutcome("ignored"), { ignored: true, cornerStyle: false });
  assert.deepEqual(offsetKeyOutcome("offset-chip"), { ignored: false, cornerStyle: false });
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
