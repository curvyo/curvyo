import assert from "node:assert/strict";
import { test } from "node:test";

import { isSecondPress, withheldPressIsDrag } from "../src/lib/doubleClick.ts";

const first = { time: 1000, x: 200, y: 150 };

test("a press soon after a click at the same place is the second press of a double-click", () => {
  assert.equal(isSecondPress(first, 1200, 202, 151, false), true);
});

test("a press after the interval, far away, with Alt or with no click before is a first press", () => {
  assert.equal(isSecondPress(first, 1500, 200, 150, false), false);
  assert.equal(isSecondPress(first, 1200, 220, 150, false), false);
  assert.equal(isSecondPress(first, 1200, 200, 150, true), false);
  assert.equal(isSecondPress(null, 1200, 200, 150, false), false);
});

test("a held-back second press that moves beyond the drag threshold is a drag", () => {
  const press = { x: 200, y: 150, touch: false };
  assert.equal(withheldPressIsDrag(press, 203, 150), true);
  assert.equal(withheldPressIsDrag(press, 260, 190), true);
});

test("a held-back second press that stays within the threshold is still a double-click", () => {
  const press = { x: 200, y: 150, touch: false };
  assert.equal(withheldPressIsDrag(press, 200, 150), false);
  assert.equal(withheldPressIsDrag(press, 202, 151), false);
});

test("a touch or a pen gets the wider tap threshold", () => {
  const press = { x: 200, y: 150, touch: true };
  assert.equal(withheldPressIsDrag(press, 205, 150), false);
  assert.equal(withheldPressIsDrag(press, 208, 150), true);
});
