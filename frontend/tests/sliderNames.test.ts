import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test } from "node:test";

import {
  HELP_COLOUR,
  HELP_RATIO,
  HELP_VALUE_FIELD,
  RATIO_SLIDER_NAMES,
  VALUE_FIELD_NAMES,
  allSliderNames,
  colourSliderName,
  tipHeading,
} from "../src/lib/sliderNames.ts";

const src = (path: string) => readFileSync(new URL(`../src/${path}`, import.meta.url), "utf8");

test("every slider and value field has a non-empty, distinct name", () => {
  const names = allSliderNames();
  assert.equal(names.length, 5 + 2 + 6);
  for (const name of names) {
    assert.ok(name.trim().length > 0, "a name is empty");
  }
  assert.equal(new Set(names).size, names.length, "two sliders share a name");
});

test("the names the customer asked for", () => {
  assert.equal(VALUE_FIELD_NAMES.strokeWidth, "Stroke width");
  assert.equal(VALUE_FIELD_NAMES.fillOpacity, "Fill opacity");
  assert.equal(VALUE_FIELD_NAMES.markerCount, "Marker count");
  assert.equal(VALUE_FIELD_NAMES.backgroundOpacity, "Background opacity");
  assert.equal(RATIO_SLIDER_NAMES.selectedStars, "Star inner ratio");
  assert.equal(colourSliderName("Stroke", "hue"), "Stroke hue");
  assert.equal(colourSliderName("Fill", "saturationValue"), "Fill saturation and value");
});

test("the first line is the name and the value; a value that does not fit leaves the name", () => {
  assert.equal(tipHeading("Stroke width", "0.50 mm"), "Stroke width: 0.50 mm");
  assert.equal(tipHeading("Fill opacity", "80 %"), "Fill opacity: 80 %");
  assert.equal(tipHeading("Stroke hue", "Mixed"), "Stroke hue: Mixed");
  assert.equal(tipHeading("Stroke hue", null), "Stroke hue");
  assert.equal(tipHeading("Stroke hue", ""), "Stroke hue");
});

test("the second line of a tooltip is the gesture help and names both gestures", () => {
  for (const help of [HELP_VALUE_FIELD, HELP_RATIO, HELP_COLOUR]) {
    assert.match(help, /Click to set/);
    assert.match(help, /double-click to type/);
  }
});

test("every slider component takes its name from the table, so the aria-label and the tooltip agree", () => {
  const picker = src("components/panel/ColourPicker.tsx");
  assert.equal((picker.match(/colourSliderName\(name, "hue"\)/g) ?? []).length, 2);
  assert.equal((picker.match(/colourSliderName\(name, "saturationValue"\)/g) ?? []).length, 2);
  const ratio = src("components/RatioSlider.tsx");
  assert.match(ratio, /aria-label=\{label\}/);
  assert.match(ratio, /name=\{label\}/);
  const field = src("components/panel/ValueField.tsx");
  assert.match(field, /aria-label=\{name\}/);
  assert.match(field, /name=\{name\}/);
  for (const [file, key] of [
    ["components/panel/StrokeSection.tsx", "strokeWidth"],
    ["components/panel/StrokeSection.tsx", "strokeOpacity"],
    ["components/panel/FillSection.tsx", "fillOpacity"],
    ["components/panel/MarkerRows.tsx", "markerCount"],
    ["components/panel/BackgroundBlock.tsx", "backgroundOpacity"],
  ] as const) {
    assert.match(src(file), new RegExp(`name=\\{VALUE_FIELD_NAMES\\.${key}\\}`), `${file} ${key}`);
  }
});
