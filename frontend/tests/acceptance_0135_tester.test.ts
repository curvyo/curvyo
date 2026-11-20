// Tester cases for `specs/0135-image-optimisation` slice 1 in the frontend's pure rules, written
// from the specification: the Strength slider's keys (criterion 26: Left and Down -1, Right and Up
// +1, Page Up and Page Down +-10, Home 0, End 100), the track position, the typed field (whole
// numbers 0 to 100, out of range refused), and the decoders of the Rust contract fixtures
// (criteria 1, 25, 28, 59: seven presets in order, Dots short cell, tooltips "<name> · Dither:
// <mode>", Restore original state, Strength hidden with Expert).
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync, readdirSync } from "node:fs";
import {
  clampStrength,
  parseStrength,
  strengthAfterKey,
  strengthAtPosition,
  strengthPosition,
} from "../src/lib/adjustText.ts";
import { decodeAdjustmentsText } from "../src/lib/adjustmentsView.ts";
import { decodeBeforeAfterText } from "../src/lib/beforeAfterView.ts";

const ROOT = new URL("../../curvyo-editor-wasm/tests/fixtures/", import.meta.url);

function fixture(folder: string, name: string): string {
  return readFileSync(new URL(`${folder}/${name}.json`, ROOT), "utf8");
}

test("c26 the Strength keys step by 1 and 10 and jump to the ends", () => {
  assert.equal(strengthAfterKey(50, "ArrowLeft"), 49);
  assert.equal(strengthAfterKey(50, "ArrowDown"), 49);
  assert.equal(strengthAfterKey(50, "ArrowRight"), 51);
  assert.equal(strengthAfterKey(50, "ArrowUp"), 51);
  assert.equal(strengthAfterKey(50, "PageUp"), 60);
  assert.equal(strengthAfterKey(50, "PageDown"), 40);
  assert.equal(strengthAfterKey(50, "Home"), 0);
  assert.equal(strengthAfterKey(50, "End"), 100);
  assert.equal(strengthAfterKey(95, "PageUp"), 100, "clamped at 100");
  assert.equal(strengthAfterKey(5, "PageDown"), 0, "clamped at 0");
  assert.equal(strengthAfterKey(0, "ArrowLeft"), 0);
  assert.equal(strengthAfterKey(100, "ArrowRight"), 100);
  assert.equal(strengthAfterKey(50, "a"), null, "other keys are not the slider's");
  assert.equal(strengthAfterKey(50, "Enter"), null);
});

test("c26 the track maps to whole percent and back", () => {
  assert.equal(strengthAtPosition(0, 200), 0);
  assert.equal(strengthAtPosition(200, 200), 100);
  assert.equal(strengthAtPosition(-20, 200), 0, "left of the track is Gentle");
  assert.equal(strengthAtPosition(260, 200), 100, "right of the track is Strong");
  assert.equal(strengthAtPosition(101, 200), 51, "whole numbers");
  assert.equal(strengthAtPosition(10, 0), 0, "a zero-width track is safe");
  for (let v = 0; v <= 100; v += 1) {
    assert.equal(strengthAtPosition(strengthPosition(v, 300), 300), v, `round trip ${v}`);
  }
  assert.equal(clampStrength(Number.NaN), 0);
  assert.equal(clampStrength(49.5), 50);
});

test("c26 the typed field takes whole numbers 0 to 100 only", () => {
  assert.equal(parseStrength("0"), 0);
  assert.equal(parseStrength("100"), 100);
  assert.equal(parseStrength(" 42 % "), 42);
  assert.equal(parseStrength("101"), null);
  assert.equal(parseStrength("-1"), null);
  assert.equal(parseStrength(""), null);
  assert.equal(parseStrength("abc"), null);
  assert.equal(parseStrength("1e2"), null);
});

test("c1 c25 c28 every adjustments fixture decodes and states the spec's strip", () => {
  const names = readdirSync(new URL("adjustments_view/", ROOT))
    .filter((f) => f.endsWith(".json"))
    .map((f) => f.slice(0, -5));
  assert.ok(names.length >= 5);
  for (const name of names) {
    const text = fixture("adjustments_view", name);
    const view = decodeAdjustmentsText(text);
    if (text.trim() === "null") {
      assert.equal(view, null, `${name}: no group without one picture`);
      continue;
    }
    assert.ok(view, `${name} decodes`);
    const raw = JSON.parse(text);
    const keys = raw.presets.map((p: { key: string }) => p.key);
    assert.deepEqual(keys, ["photo", "graphics", "black_white", "line_art", "dot_raster", "newspaper", "expert"]);
    assert.equal(raw.presets[4].cell, "Dots");
    assert.equal(raw.presets.filter((p: { pressed: boolean }) => p.pressed).length, 1, `${name}: one pressed cell`);
    for (const p of raw.presets.slice(0, 6)) {
      assert.match(p.tooltip, new RegExp(`^${p.name.replace(/[()]/g, "\\$&")} · Dither: `), `${name}: tooltip of ${p.key}`);
    }
    const expert = raw.presets[6].pressed;
    assert.equal(raw.strength === null, expert, `${name}: Strength shown exactly with a preset`);
    if (!expert) assert.match(raw.derived.line, /^From preset .+, Strength \d+ %$/);
  }
});

test("c25 a malformed adjustments view is refused", () => {
  const good = JSON.parse(fixture("adjustments_view", "preset_photo"));
  for (const broken of [
    { ...good, presets: "photo" },
    { ...good, restore_enabled: "yes" },
    { ...good, strength: { value: "50" } },
    { ...good, steps: [{ kind: 3 }] },
  ]) {
    assert.equal(decodeAdjustmentsText(JSON.stringify(broken)), null);
  }
  assert.equal(decodeAdjustmentsText("not json"), null);
});

test("c11 the Before and after fixtures decode; Split is the default view", () => {
  const view = decodeBeforeAfterText(fixture("before_after_view", "open_two_operations"));
  assert.ok(view);
  const raw = JSON.parse(fixture("before_after_view", "open_two_operations"));
  assert.equal(raw.view, "split");
  assert.equal(raw.operations.filter((o: { pressed: boolean }) => o.pressed).length, 1, "one operation picked");
  assert.equal(raw.operations[0].pressed, true, "the first by default");
  assert.equal(decodeBeforeAfterText("{\"open\": 3}"), null);
});
