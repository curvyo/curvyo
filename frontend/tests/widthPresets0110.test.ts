import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test } from "node:test";

import { WIDTH_PRESETS, WIDTH_PRESET_GLYPHS } from "../src/lib/widthPresets.ts";

test("criterion 5: Uniform, Taper start, Taper end and Taper both, each with a glyph", () => {
  assert.deepEqual(WIDTH_PRESETS.map((p) => p.code), ["uniform", "taper-start", "taper-end", "taper-both"]);
  for (const preset of WIDTH_PRESETS) {
    assert.ok(preset.tooltip.startsWith(preset.label));
    assert.match(WIDTH_PRESET_GLYPHS[preset.code], /^M/);
  }
});

test("criterion 5 and 0114 criterion 4: the Style panel offers the presets, the tool bar Edit width", () => {
  const section = readFileSync(new URL("../src/components/panel/StrokeSection.tsx", import.meta.url), "utf8");
  assert.match(section, /WIDTH_PRESETS\.map/);
  assert.match(section, /applyWidthPreset/);
  assert.doesNotMatch(section, /Edit width/);
  const toggle = readFileSync(new URL("../src/components/WidthEditToggle.tsx", import.meta.url), "utf8");
  assert.match(toggle, /Edit width/);
  assert.match(toggle, /aria-pressed=\{active\}/);
});
