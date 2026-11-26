import assert from "node:assert/strict";
import { test } from "node:test";

import { entryHasCopyCheck } from "../src/lib/paramEntryCopy.ts";

test("only the parameter chips have the Copy check (0073 criterion 11)", () => {
  for (const kind of ["curve", "arc-angle", "corner-radius", "inner-ratio", "rounded"]) {
    assert.equal(entryHasCopyCheck(kind), true, kind);
  }
  for (const kind of ["size", "angle", "skew", "radius", "fixture-length", "fixture-size"]) {
    assert.equal(entryHasCopyCheck(kind), false, kind);
  }
});

