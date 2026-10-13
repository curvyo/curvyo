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

test("criteria 12, 15, 18: the readout's two lines", () => {
  const dragging = { ...OFFSET_IDLE, operands: 1, dragging: true };
  assert.deepEqual(offsetReadout({ ...dragging, distanceMm: 2.5 }), ["Outset 2.5 mm", "New object"]);
  assert.deepEqual(offsetReadout({ ...dragging, distanceMm: -0.8, change: true }), [
    "Inset 0.8 mm",
    "Changes the original",
  ]);
  assert.deepEqual(offsetReadout({ ...dragging, distanceMm: 2.5, both: true, change: true }), [
    "Both ±2.5 mm",
    "Changes the original, adds the inner copy",
  ]);
  assert.deepEqual(offsetReadout({ ...dragging, distanceMm: -2.5, both: true, change: true }), [
    "Both ±2.5 mm",
    "Changes the original, adds the outer copy",
  ]);
  assert.equal(offsetReadout({ ...dragging, distanceMm: -1, stop: "nothing_left" })[0], "Nothing is left");
  assert.equal(
    offsetReadout({ ...dragging, distanceMm: -1, vanished: 1, of: 3 })[0],
    "Inset 1 mm, 1 of 3 shapes vanish",
  );
  assert.equal(offsetReadout({ ...dragging, large: true })[1], "Preview off: large selection");
});

test("distances print without float noise", () => {
  assert.equal(formatDistance(0.1 + 0.2), "0.3");
  assert.equal(formatDistance(-2.5), "2.5");
});
