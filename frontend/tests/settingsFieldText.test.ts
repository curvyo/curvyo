// Typed numbers of the settings fields (`specs/0028` ux-notes 8.3).
import { test } from "node:test";
import assert from "node:assert/strict";
import { CHIP_IS_MESSAGE, outcomeOf, stepOf, stepText, typedNumber } from "../src/lib/settingsFieldText.ts";

test("text becomes a number, a comma is a decimal point, and non-numbers are NaN for the session to refuse", () => {
  assert.equal(typedNumber("65"), 65);
  assert.equal(typedNumber(" 12,5 "), 12.5);
  assert.ok(Number.isNaN(typedNumber("")));
  assert.ok(Number.isNaN(typedNumber("abc")));
});

test("a chip is its own message and an empty answer is a commit", () => {
  assert.equal(outcomeOf(""), "committed");
  assert.equal(outcomeOf("Power is 1 to 100 %."), "invalid:Power is 1 to 100 %.");
  assert.equal(CHIP_IS_MESSAGE["Power is 1 to 100 %."], "Power is 1 to 100 %.");
});

test("Up and Down step by 1, Shift by 10", () => {
  assert.equal(stepOf(60, "ArrowUp", false), 61);
  assert.equal(stepOf(60, "ArrowDown", true), 50);
  assert.equal(stepOf(null, "ArrowUp", false), 1);
  assert.equal(stepOf(60, "Enter", false), null);
});

test("stepText steps typed text by 1, Shift by 10, and ignores other keys and non-numbers", () => {
  assert.equal(stepText("60", "ArrowUp", false), "61");
  assert.equal(stepText("60", "ArrowDown", true), "50");
  assert.equal(stepText("2,5", "ArrowUp", false), "3.5");
  assert.equal(stepText("", "ArrowUp", false), null);
  assert.equal(stepText("abc", "ArrowUp", false), null);
  assert.equal(stepText("60", "Enter", false), null);
});
