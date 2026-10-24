// Rename and delete from the tag strip (`specs/0075-font-library` criteria 7 and 8, the customer
// change of 2026-10-20): F2 renames and Delete asks before deleting an own tag; built-in tags,
// All and Favourites ignore both; the confirm names the tag and its fonts.
import assert from "node:assert/strict";
import { test } from "node:test";

import { armedTagLabel, tagNameError } from "../src/lib/fontLibraryText.ts";
import { stripCells, stripKeyAction } from "../src/lib/tagStrip.ts";

test("F2 renames an own tag from the strip, a clash is refused with the inline message", () => {
  assert.equal(stripKeyAction("F2", true), "rename");
  assert.equal(stripKeyAction("F2", false), null);
  assert.equal(tagNameError("taken", "Script"), "A tag named Script exists.");
  assert.equal(tagNameError("long"), "Use at most 24 characters.");
});

test("Delete asks before deleting an own tag from the strip", () => {
  assert.equal(stripKeyAction("Delete", true), "delete");
  assert.equal(stripKeyAction("Delete", false), null);
  assert.equal(stripKeyAction("ArrowRight", true), null);
  assert.equal(armedTagLabel("Wedding", 1), "Press again to delete tag Wedding. Its 1 font stays in Curvyo.");
});

test("a deleted tag leaves the strip; only All, Favourites and existing tags are cells", () => {
  const tags = [
    { id: "cuttable", name: "Cuttable", builtin: true, members: new Set<string>() },
    { id: "Signs", name: "Signs", builtin: false, members: new Set(["Lobster"]) },
  ];
  assert.deepEqual(
    stripCells(tags).map((cell) => `${cell.id}${cell.own ? "*" : ""}`),
    ["all", "favourites", "cuttable", "Signs*"],
  );
  assert.deepEqual(
    stripCells(tags.slice(0, 1)).map((cell) => cell.id),
    ["all", "favourites", "cuttable"],
  );
});
