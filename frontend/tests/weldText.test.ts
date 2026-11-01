import assert from "node:assert/strict";
import { test } from "node:test";

import {
  type WeldResult,
  weldRefusalText,
  weldTooltipNote,
} from "../src/lib/weldText.ts";
import { BOOLEAN_CARD_COMMANDS } from "../src/lib/stackText.ts";

const result = (fields: Partial<WeldResult>): WeldResult => ({
  kind: "applied",
  count: 0,
  of: 2,
  textsOnly: false,
  compound: true,
  lengthText: "140.0 mm",
  hidden: 0,
  closedOpen: 0,
  removedOpen: 0,
  mixedStyles: false,
  fillSwitchedOff: false,
  strokeSwitchedOn: false,
  ...fields,
});

test("criterion 1: Weld is the eighth button, enabled with one or more objects", () => {
  assert.equal(BOOLEAN_CARD_COMMANDS[7], "weld");
  assert.equal(BOOLEAN_CARD_COMMANDS.length, 8);
  assert.equal(weldTooltipNote({ needsOne: true, open: 0 }, true), "Select one or more objects.");
  assert.equal(
    weldTooltipNote({ needsOne: true, open: 0 }, false),
    "Select one or more objects. Use the Select tool.",
  );
  assert.equal(
    weldTooltipNote({ needsOne: false, open: 2 }, true),
    "Closes 2 open outlines with a straight line.",
  );
  assert.equal(weldTooltipNote({ needsOne: false, open: 0 }, true), "Replaces the selection.");
});

test("criterion 24: refusals name Weld, and one selected object reads as such", () => {
  assert.equal(
    weldRefusalText(result({ kind: "no_area", count: 3, of: 3 })),
    "Weld needs shapes that enclose an area. 3 of 3 selected objects have no area. Nothing was changed.",
  );
  assert.equal(
    weldRefusalText(result({ kind: "no_area", count: 1, of: 1 })),
    "Weld needs shapes that enclose an area. The selected object has no area. Nothing was changed.",
  );
  assert.equal(
    weldRefusalText(result({ kind: "out_of_range", count: 1, of: 1 })),
    "Weld works only within 10 km of the point 0, 0. The selected object reaches further. Nothing was changed.",
  );
});
