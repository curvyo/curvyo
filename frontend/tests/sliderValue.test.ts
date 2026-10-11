// Run with `npm test`. The numbers of the ratio sliders (`src/lib/sliderValue.ts`,
// `specs/0047-transform-polish/` criteria 17 and 22).

import assert from "node:assert/strict";
import { test } from "node:test";

import {
  RATIO_RANGE,
  parseRatioEntry,
  positionOfValue,
  snapToStep,
  valueAtPosition,
} from "../src/lib/sliderValue.ts";

test("a click at the middle of the track gives 0.50, at the ends 0.01 and 0.99", () => {
  assert.equal(valueAtPosition(50, 100, RATIO_RANGE), 0.5);
  assert.equal(valueAtPosition(0, 100, RATIO_RANGE), 0.01);
  assert.equal(valueAtPosition(100, 100, RATIO_RANGE), 0.99);
  assert.equal(valueAtPosition(-40, 100, RATIO_RANGE), 0.01);
  assert.equal(valueAtPosition(400, 100, RATIO_RANGE), 0.99);
});

test("a position rounds to the nearest step", () => {
  assert.equal(valueAtPosition(30.4, 100, RATIO_RANGE), 0.31);
  assert.equal(snapToStep(0.456, RATIO_RANGE), 0.46);
  assert.equal(snapToStep(0.3 + 0.01 * 3, RATIO_RANGE), 0.33);
});

test("the thumb sits where the value is", () => {
  assert.equal(positionOfValue(0.5, 100, RATIO_RANGE), 50);
  assert.equal(positionOfValue(0.01, 100, RATIO_RANGE), 0);
  assert.equal(positionOfValue(0.99, 100, RATIO_RANGE), 100);
});

test("a typed ratio rounds to two decimals and takes a point or a comma", () => {
  assert.deepEqual(parseRatioEntry("0.456"), { ok: true, value: 0.46 });
  assert.deepEqual(parseRatioEntry(" 0,5 "), { ok: true, value: 0.5 });
  assert.deepEqual(parseRatioEntry(".25"), { ok: true, value: 0.25 });
  assert.deepEqual(parseRatioEntry("0.01"), { ok: true, value: 0.01 });
  assert.deepEqual(parseRatioEntry("0.99"), { ok: true, value: 0.99 });
});

test("a typed ratio outside 0.01 to 0.99 as typed, or not a number, is refused", () => {
  for (const text of [
    "0.995",
    "0.994",
    "1",
    "0",
    "0.004",
    "-0.5",
    "abc",
    "",
    "0.5.5",
    "1e-1",
    "Infinity",
    "NaN",
  ]) {
    assert.deepEqual(
      parseRatioEntry(text),
      { ok: false },
      JSON.stringify(text),
    );
  }
});
