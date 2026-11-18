// The display helpers of the Gradient block (`specs/0124-gradient-fill` UX notes §3, §4).
import assert from "node:assert/strict";
import { test } from "node:test";

import { stepAngle, stepRadius, stopColour, trimmedNumber } from "../src/lib/gradientText.ts";

test("a stop colour reads #RRGGBBAA into a colour and an opacity percent", () => {
  assert.deepEqual(stopColour("#3366CC80"), { rgb: 0x3366cc, opacity: (128 / 255) * 100 });
  assert.equal(stopColour("#FFFFFFFF")?.opacity, 100);
  assert.equal(stopColour(""), null, "empty text is a mixed row");
  assert.equal(stopColour("#FFF"), null);
});

test("the Angle steps by 1 degree and by 15 with Shift, keeping one decimal", () => {
  assert.equal(stepAngle("30.0", "ArrowUp", false), "31.0");
  assert.equal(stepAngle("30.0", "ArrowDown", true), "15.0");
  assert.equal(stepAngle("x", "ArrowUp", false), null, "text that is no number is left to the field");
  assert.equal(stepAngle("30.0", "a", false), null);
});

test("the Radius steps by 0.1 mm and by 1 with Shift, trailing zeros trimmed", () => {
  assert.equal(stepRadius("24.5", "ArrowUp", false), "24.6");
  assert.equal(stepRadius("24.5", "ArrowUp", true), "25.5");
  assert.equal(stepRadius("1", "ArrowDown", false), "0.9");
  assert.equal(trimmedNumber(50, 3), "50");
  assert.equal(trimmedNumber(33.25, 1), "33.3");
});
