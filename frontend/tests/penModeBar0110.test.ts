import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test } from "node:test";

import { PEN_MODES, showsSmoothing } from "../src/lib/penModes.ts";

test("criterion 1: one mode per button, Bézier first, every one named", () => {
  assert.deepEqual(PEN_MODES.map((m) => m.value), ["bezier", "spiro", "bspline", "lines", "freehand"]);
  for (const mode of PEN_MODES) {
    assert.ok(mode.label.length > 0 && mode.tooltip.startsWith(mode.label));
  }
});

test("criterion 1: Smoothing shows for Freehand alone", () => {
  assert.deepEqual(PEN_MODES.filter((m) => showsSmoothing(m.value)).map((m) => m.value), ["freehand"]);
});

test("criterion 1: the bar shows icons, no text labels, and names each button", () => {
  const bar = readFileSync(new URL("../src/components/PenModeBar.tsx", import.meta.url), "utf8");
  assert.match(bar, /<ToggleGroup/);
  assert.doesNotMatch(bar, /<span>Mode<\/span>/);
  assert.doesNotMatch(bar, /\{label\}/);
});

test("a refused mode change shows the session's reason, not a fixed text", () => {
  const bar = readFileSync(new URL("../src/components/PenModeBar.tsx", import.meta.url), "utf8");
  assert.match(bar, /pen_mode_refusal\(next\)/);
  assert.doesNotMatch(bar, /Not in a shared document/);
});
