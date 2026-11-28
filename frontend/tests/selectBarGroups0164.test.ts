// Which groups the Select bar shows per selection (`specs/0164-ui-consistency`, Frage 460 B): a
// kind group only while every selected object is of its kind; in the narrow window class Align
// and Distribute only from two selected objects.

import assert from "node:assert/strict";
import { test } from "node:test";

import {
  arrangeGroupsShown,
  bridgeParts,
  isNarrowSelectBar,
  kindGroupShown,
  NARROW_SELECT_BAR_PX,
  readSelectionKinds,
  type KindGroup,
} from "../src/lib/selectBarGroups.ts";

const ALL_GROUPS: KindGroup[] = [
  "rectangle",
  "points",
  "ratio",
  "ellipse",
  "curve",
  "randomized",
  "text",
  "objectToPath",
  "path",
];

const shown = (kinds: string[]) => ALL_GROUPS.filter((group) => kindGroupShown(group, kinds));

test("433 B (a): nothing selected shows no kind group", () => {
  assert.deepEqual(shown([]), []);
});

test("433 B (a): one kind shows its own groups", () => {
  assert.deepEqual(shown(["rectangle"]), ["rectangle", "objectToPath"]);
  assert.deepEqual(shown(["star"]), ["points", "ratio", "curve", "randomized", "objectToPath"]);
  assert.deepEqual(shown(["polygon"]), ["points", "curve", "randomized", "objectToPath"]);
  assert.deepEqual(shown(["ellipse"]), ["ellipse", "curve", "objectToPath"]);
  assert.deepEqual(shown(["text"]), ["text", "objectToPath"]);
  assert.deepEqual(shown(["path"]), ["path"]);
  assert.deepEqual(shown(["image"]), []);
});

test("433 B (a): a mixed selection shows only what every kind shares", () => {
  assert.deepEqual(shown(["rectangle", "path"]), [], "rectangle and path share no kind group");
  assert.deepEqual(shown(["rectangle", "ellipse"]), ["objectToPath"]);
  assert.deepEqual(shown(["polygon", "star"]), ["points", "curve", "randomized", "objectToPath"], "no Ratio for a polygon");
  assert.deepEqual(shown(["ellipse", "star"]), ["curve", "objectToPath"], "no Arc kind, no Points");
  assert.deepEqual(shown(["text", "rectangle"]), ["objectToPath"], "no text groups beside a rectangle");
  assert.deepEqual(shown(["rectangle", "ellipse", "star", "path", "text"]), []);
  assert.deepEqual(shown(["rectangle", "generated"]), ["objectToPath"]);
});

test("433 B (c): the narrow class is the 800 px window, not the 1440 px one", () => {
  assert.equal(NARROW_SELECT_BAR_PX, 690);
  assert.equal(isNarrowSelectBar(324), true, "800 x 640 with the panel open");
  assert.equal(isNarrowSelectBar(689), true);
  assert.equal(isNarrowSelectBar(690), false);
  assert.equal(isNarrowSelectBar(1000), false);
});

test("433 B (c): Align and Distribute from two objects in the narrow class, always in a wide one", () => {
  for (const count of [0, 1]) {
    assert.deepEqual(arrangeGroupsShown(count, true), { align: false, distribute: false, grid: false }, `${count} narrow`);
    assert.deepEqual(arrangeGroupsShown(count, false), { align: true, distribute: true, grid: false }, `${count} wide`);
  }
  for (const narrow of [true, false]) {
    assert.deepEqual(arrangeGroupsShown(2, narrow), { align: true, distribute: true, grid: true });
    assert.deepEqual(arrangeGroupsShown(7, narrow), { align: true, distribute: true, grid: true });
  }
});

test("the session's kind codes are read as a list", () => {
  assert.deepEqual(readSelectionKinds(""), []);
  assert.deepEqual(readSelectionKinds("rectangle"), ["rectangle"]);
  assert.deepEqual(readSelectionKinds("path,rectangle,polygon"), ["path", "rectangle", "polygon"]);
});

test("the Bridge value fields are absent while Bridge options is off (Frage 461 A)", () => {
  assert.deepEqual(bridgeParts(false, "lettering"), ["button", "options"]);
  assert.deepEqual(bridgeParts(false, "stencil"), ["button", "options"]);
});

test("the Bridge value fields are present while Bridge options is on (Frage 461 A)", () => {
  assert.deepEqual(bridgeParts(true, "lettering"), ["button", "options", "mode", "width", "reach", "joinAll"]);
  assert.deepEqual(bridgeParts(true, "stencil"), ["button", "options", "mode", "width", "bridges"]);
});
