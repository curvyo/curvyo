// Run with `npm test`. The sentences of `src/lib/stackText.ts` against the wording of
// `specs/0037-fracture-and-flatten/specification.md` (criteria 1 to 4, 9, 11, 13 and the UX
// notes).

import assert from "node:assert/strict";
import { test } from "node:test";

import {
  BOOLEAN_CARD_COMMANDS,
  type StackResult,
  stackName,
  stackRefusalText,
  stackRule,
  stackSuccessText,
} from "../src/lib/stackText.ts";

function result(fields: Partial<StackResult>): StackResult {
  return {
    kind: "applied",
    count: 0,
    of: 0,
    removed: 0,
    pieces: 0,
    hidden: 0,
    strokesFollow: false,
    closedOpen: 0,
    removedOpen: 0,
    ...fields,
  };
}

test("criterion 1: buttons six and seven with their names and rule lines", () => {
  assert.deepEqual(BOOLEAN_CARD_COMMANDS.slice(5), ["fracture", "flatten"]);
  assert.equal(stackName("fracture"), "Fracture");
  assert.equal(stackRule("fracture"), "Cuts overlapping shapes into pieces.");
  assert.equal(stackName("flatten"), "Flatten");
  assert.equal(stackRule("flatten"), "Trims each shape to what shows.");
});

test("criteria 9, 11 and 13: success notices", () => {
  assert.equal(
    stackSuccessText("fracture", result({ of: 3, removed: 3, pieces: 7 })),
    "Fracture: 3 objects became 7 pieces.",
  );
  assert.equal(
    stackSuccessText("flatten", result({ of: 2, removed: 1, hidden: 1 })),
    "Flatten: 2 objects became 1. 1 hidden object was removed.",
  );
  assert.equal(
    stackSuccessText("flatten", result({ of: 2, removed: 1, pieces: 1, strokesFollow: true })),
    "Flatten: 2 objects became 2. Strokes follow the trimmed outlines.",
  );
});

test("criteria 2 to 4: refusals", () => {
  assert.equal(
    stackRefusalText("fracture", result({ kind: "no_overlap" })),
    "Fracture: the selected shapes do not overlap. Nothing was changed.",
  );
  assert.equal(
    stackRefusalText("fracture", result({ kind: "too_many" })),
    "Fracture would make more than 5000 pieces. Nothing was changed.",
  );
  assert.equal(
    stackRefusalText("flatten", result({ kind: "groups", count: 2, of: 2 })),
    "Flatten does not work on groups. Ungroup first. Nothing was changed.",
  );
});
