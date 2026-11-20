// The Image settings fields' look and steps (`specs/0123-raster-engraving` UX notes 2.2, 2.4, P34).
import { test } from "node:test";
import assert from "node:assert/strict";

import { IMAGE_FIELD_LOOK, imageFieldStep, imageFieldText } from "../src/lib/imageSettingsRows.ts";

test("a field shows at most two decimals without trailing zeros", () => {
  assert.equal(imageFieldText(1), "1");
  assert.equal(imageFieldText(1.5), "1.5");
  assert.equal(imageFieldText(0.333), "0.33");
});

test("whole percents step by 1 and Shift 10, Gamma by 0.01 and Shift 0.1", () => {
  assert.equal(imageFieldStep("brightness")("-10", "ArrowUp", false), "-9");
  assert.equal(imageFieldStep("contrast")("20", "ArrowDown", true), "10");
  assert.equal(imageFieldStep("gamma")("1.5", "ArrowUp", false), "1.51");
  assert.equal(imageFieldStep("gamma")("1.5", "ArrowDown", true), "1.4");
});

test("other keys and non-numbers are left alone", () => {
  assert.equal(imageFieldStep("threshold")("5", "a", false), null);
  assert.equal(imageFieldStep("threshold")("x", "ArrowUp", false), null);
});

test("the widths are those of the notes", () => {
  assert.equal(IMAGE_FIELD_LOOK.threshold.width, 124);
  assert.equal(IMAGE_FIELD_LOOK.lowest_power.width, 132);
  assert.equal(IMAGE_FIELD_LOOK.brightness.width, 120);
  assert.equal(IMAGE_FIELD_LOOK.contrast.width, 108);
  assert.equal(IMAGE_FIELD_LOOK.screen_size.width, 136);
  assert.equal(
    IMAGE_FIELD_LOOK.brightness.width + 4 + IMAGE_FIELD_LOOK.contrast.width,
    232,
    "one row of 232 px (P34, UX review U3)",
  );
});
