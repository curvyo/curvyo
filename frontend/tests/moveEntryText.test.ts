import assert from "node:assert/strict";
import { test } from "node:test";

import { submittedTexts } from "../src/lib/moveEntryText.ts";

// A 30 x 40 box at (10, 20): the nine points in the order of the radios.
const entry = {
  relative: ["0", "0"] as const,
  absolute: [
    ["10.0", "20.0"],
    ["25.0", "20.0"],
    ["40.0", "20.0"],
    ["10.0", "40.0"],
    ["25.0", "40.0"],
    ["40.0", "40.0"],
    ["10.0", "60.0"],
    ["25.0", "60.0"],
    ["40.0", "60.0"],
  ] as const,
};
const CENTRE = 4;
const BOTTOM_RIGHT = 8;

test("an untouched field sends the prefill of the chosen reference, whatever mark the pointer rests on", () => {
  // The maker typed X only; the pointer rests on the bottom right mark (8) while Enter is pressed.
  const fields = { edited: [true, false] as const, typed: ["100", ""] as const };
  const [x, y] = submittedTexts(entry, fields, true, CENTRE);
  assert.equal(x, "100");
  assert.equal(y, "40.0", "Y is the centre's own Y, so Rust reads it as untouched");
  assert.notEqual(y, entry.absolute[BOTTOM_RIGHT][1]);
});

test("a typed field is sent as typed, and Relative sends the relative prefill", () => {
  const fields = { edited: [false, true] as const, typed: ["", "-3"] as const };
  assert.deepEqual(submittedTexts(entry, fields, false, CENTRE), ["0", "-3"]);
  assert.deepEqual(submittedTexts(entry, fields, true, 0), ["10.0", "-3"]);
});

test("a reference outside 0 to 8 sends an empty untouched field, which Rust refuses", () => {
  const fields = { edited: [false, false] as const, typed: ["", ""] as const };
  assert.deepEqual(submittedTexts(entry, fields, true, 9), ["", ""]);
});
