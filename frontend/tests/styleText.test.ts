// The Style group's words and keys (`specs/0078-font-style-buttons` criteria 1, 6, 11, 12, 19,
// 21).

import assert from "node:assert/strict";
import { test } from "node:test";

import {
  EMPTY_STYLE,
  type StyleState,
  boldKeyTarget,
  parseStyle,
  styleNotice,
  styleTooltip,
  weightFocusMove,
  weightTabStop,
} from "../src/lib/styleText.ts";

const idle = { pressed: "off" as const, greyed: null };

function withWeights(pressed: number, greyed: number[] = []): StyleState {
  return {
    weights: [0, 1, 2, 3, 4].map((i) => ({
      pressed: i === pressed ? ("on" as const) : ("off" as const),
      greyed: greyed.includes(i) ? { kind: "not_in" as const, name: "Source Sans 3", count: 0 } : null,
    })),
    italic: idle,
  };
}

test("criterion 1: names, and the keys only in edit mode", () => {
  assert.deepEqual(styleTooltip("extra-light", idle, false, false), { name: "Extra Light", line: null });
  assert.equal(styleTooltip("bold", idle, false, false).name, "Bold");
  assert.equal(styleTooltip("bold", idle, true, false).name, "Bold (Ctrl+B)");
  assert.equal(styleTooltip("italic", idle, true, true).name, "Italic (Cmd+I)");
  assert.equal(styleTooltip("light", idle, true, false).name, "Light");
});

test("criterion 6: the greyed reasons and the mixed line", () => {
  const line = (kind: "not_in" | "not_in_as_italic" | "not_in_as_upright" | "not_in_any" | "font_missing", name = "", count = 0) =>
    styleTooltip("bold", { pressed: "off", greyed: { kind, name, count } }, false, false).line;
  assert.equal(line("not_in", "Great Vibes"), "Not in Great Vibes");
  assert.equal(line("not_in_as_italic", "Source Serif 4"), "Not in Source Serif 4 as italic");
  assert.equal(line("not_in_as_upright", "Lobster"), "Not in Lobster as upright");
  assert.equal(line("not_in_any", "", 3), "Not in any of the 3 fonts");
  assert.equal(line("font_missing"), "Font missing");
  assert.equal(styleTooltip("italic", { pressed: "mixed", greyed: null }, false, false).line, "Mixed");
});

test("criteria 11 and 12: the notices, with one text left and with several", () => {
  assert.equal(styleNotice("bold", { set: 2, left: 1, upright: false }), "Bold set on 2 of 3 texts; 1 has no Bold.");
  assert.equal(
    styleNotice("extra-light", { set: 128, left: 12, upright: false }),
    "Extra Light set on 128 of 140 texts; the others have no Extra Light.",
  );
  assert.equal(
    styleNotice("italic", { set: 2, left: 2, upright: false }),
    "Italic set on 2 of 4 texts; the others have no italic.",
  );
  assert.equal(styleNotice("italic", { set: 2, left: 1, upright: true }), "Upright set on 2 of 3 texts; 1 has no upright face.");
  assert.equal(styleNotice("bold", { set: 3, left: 0, upright: false }), null);
});

test("criterion 21: arrows, Home and End move focus without wrapping", () => {
  assert.equal(weightFocusMove("ArrowRight", 4), 4);
  assert.equal(weightFocusMove("ArrowLeft", 0), 0);
  assert.equal(weightFocusMove("ArrowDown", 1), 2);
  assert.equal(weightFocusMove("Home", 3), 0);
  assert.equal(weightFocusMove("End", 0), 4);
  assert.equal(weightFocusMove("Enter", 2), null);
  assert.equal(weightTabStop(withWeights(3)), 3);
  assert.equal(weightTabStop(EMPTY_STYLE), 2);
});

test("criterion 19: Ctrl+B presses Bold, Regular on a Bold text, nothing when greyed", () => {
  assert.equal(boldKeyTarget(withWeights(2)), "bold");
  assert.equal(boldKeyTarget(withWeights(3)), "regular");
  assert.equal(boldKeyTarget(withWeights(2, [3])), null);
});

test("the session's JSON reads, and anything else is the idle group", () => {
  assert.deepEqual(parseStyle("nope"), EMPTY_STYLE);
  const state = withWeights(1);
  assert.deepEqual(parseStyle(JSON.stringify(state)), state);
});
