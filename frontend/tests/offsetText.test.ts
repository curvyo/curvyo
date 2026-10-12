// Run with `npm test`. The words of `src/lib/offsetText.ts` against `specs/0038-path-offset`
// (criteria 7, 8, 13, 14 and the UX notes).

import assert from "node:assert/strict";
import { test } from "node:test";

import {
  OFFSET_CLOSED,
  formatDistance,
  offsetCaption,
  offsetRefusalText,
  offsetSuccessText,
} from "../src/lib/offsetText.ts";

const applied = (count: number, distanceMm: number, vanished = 0, of = count) => ({
  kind: "applied",
  count,
  of,
  vanished,
  distanceMm,
});

test("the success notice names the count, the distance and the side", () => {
  assert.equal(offsetSuccessText(applied(2, 5)), "Offset: added 2 shapes, 5 mm outward.");
  assert.equal(offsetSuccessText(applied(1, -0.15)), "Offset: added 1 shape, 0.15 mm inward.");
  assert.equal(
    offsetSuccessText(applied(2, -3, 1, 3)),
    "Offset: added 2 shapes, 3 mm inward. 1 of 3 shapes vanished.",
  );
  assert.equal(offsetSuccessText({ ...applied(0, 1), kind: "ignored" }), null);
});

test("the refusals are the sentences of the spec", () => {
  const refusal = (kind: string, count = 0, of = 0) => ({ kind, count, of, vanished: 0, distanceMm: 0 });
  assert.equal(
    offsetRefusalText(refusal("empty"), -10),
    "Inset is empty: nothing is left at 10 mm. Nothing was changed.",
  );
  assert.equal(
    offsetRefusalText(refusal("inset_open", 1, 2), -1),
    "Inset needs closed paths. 1 of 2 selected objects is open. Nothing was changed.",
  );
  assert.equal(
    offsetRefusalText(refusal("groups", 3, 3), 1),
    "Offset does not work on groups. Ungroup first. Nothing was changed.",
  );
  assert.match(offsetRefusalText(refusal("out_of_range", 1, 2), 1) ?? "", /^Offset works only within 10 km/);
  assert.match(offsetRefusalText(refusal("no_area", 1, 1), 1) ?? "", /^Offset needs shapes that enclose an area/);
  assert.equal(offsetRefusalText(refusal("closed_zero"), 0), null);
});

test("the caption turns the sign into a word and shows the reason of a refusal", () => {
  assert.deepEqual(offsetCaption({ ...OFFSET_CLOSED, kind: "result", distanceMm: 2 }), {
    text: "Outset",
    refusal: false,
  });
  assert.deepEqual(offsetCaption({ ...OFFSET_CLOSED, kind: "result", distanceMm: -2 }), {
    text: "Inset",
    refusal: false,
  });
  assert.deepEqual(offsetCaption({ ...OFFSET_CLOSED, kind: "zero", distanceMm: 0 }), {
    text: "",
    refusal: false,
  });
  assert.deepEqual(offsetCaption({ ...OFFSET_CLOSED, kind: "empty", distanceMm: -10 }), {
    text: "Nothing is left at −10 mm.",
    refusal: true,
  });
  assert.deepEqual(
    offsetCaption({ ...OFFSET_CLOSED, kind: "inset_open", count: 1, of: 2, distanceMm: -1 }),
    { text: "Needs closed paths: 1 of 2 selected is open.", refusal: true },
  );
});

test("a group in the selection is named in the caption", () => {
  assert.deepEqual(offsetCaption({ ...OFFSET_CLOSED, kind: "groups", count: 2, of: 2 }), {
    text: "Does not work on groups. Ungroup first.",
    refusal: true,
  });
});

test("distances keep three decimals at most", () => {
  assert.equal(formatDistance(0.15, false), "0.15");
  assert.equal(formatDistance(-2.0004, true), "−2");
  assert.equal(formatDistance(1000, false), "1000");
});
