// Tester cases for `specs/0121-print-and-cut` criteria 7 and 8, written from the spec and
// ADR 0081 §8: typed measures (minus sign, point or comma) and when Take head position reads.
import assert from "node:assert/strict";
import { test } from "node:test";

import { headPositionState, parseMeasure } from "../src/lib/printAndCutView.ts";

test("criterion 7: plain number entry with a minus sign and a decimal point or comma", () => {
  assert.equal(parseMeasure("0"), 0);
  assert.equal(parseMeasure("-0.01"), -0.01);
  assert.equal(parseMeasure("283,3"), 283.3);
  assert.equal(parseMeasure(" 57.1 "), 57.1);
  assert.equal(parseMeasure("-10000"), -10000);
  assert.equal(parseMeasure("1.2.3"), undefined);
  assert.equal(parseMeasure("12 mm"), undefined);
  assert.equal(parseMeasure(""), null);
});

test("criterion 8: Take head position reads in Idle, Ready and Done with a position only", () => {
  for (const state of ["Idle", "Ready", "Done"]) {
    assert.deepEqual(headPositionState(state, true), { enabled: true }, state);
    assert.deepEqual(
      headPositionState(state, false),
      { enabled: false, cause: "No position from the machine" },
      state,
    );
  }
  for (const state of ["Running", "Homing"]) {
    for (const has of [true, false]) {
      assert.deepEqual(headPositionState(state, has), { enabled: false, cause: "Machine is moving" }, state);
    }
  }
  assert.deepEqual(headPositionState(null, true), { enabled: false, cause: "No position from the machine" });
});
