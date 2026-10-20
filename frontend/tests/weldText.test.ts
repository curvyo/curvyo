import assert from "node:assert/strict";
import { test } from "node:test";

import {
  type WeldResult,
  weldRefusalText,
  weldSuccessText,
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

test("criteria 4, 5 and 21: the notice counts, gives the line length and its sentences in order", () => {
  assert.equal(
    weldSuccessText(result({})),
    "Weld: 2 objects became 1 compound path with 140.0 mm of line.",
  );
  assert.equal(
    weldSuccessText(result({ of: 1, textsOnly: true, lengthText: "1312.6 mm" })),
    "Weld: 1 text became 1 compound path with 1312.6 mm of line.",
  );
  assert.equal(
    weldSuccessText(
      result({
        of: 3,
        lengthText: "412.3 mm",
        hidden: 1,
        closedOpen: 1,
        mixedStyles: true,
        fillSwitchedOff: true,
        strokeSwitchedOn: true,
      }),
    ),
    "Weld: 3 objects became 1 compound path with 412.3 mm of line. 1 hidden shape was removed. " +
      "1 open outline was closed with a straight line. It uses the style of the lowest object. " +
      "Fill was switched off. Stroke was switched on.",
  );
  assert.equal(
    weldSuccessText(result({ of: 1, compound: false, lengthText: "80.0 mm", hidden: 3 })),
    "Weld: 1 object became 1 path with 80.0 mm of line. 3 hidden shapes were removed.",
  );
  assert.equal(weldSuccessText(result({ kind: "ignored" })), null);
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
