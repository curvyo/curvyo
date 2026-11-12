// The row logic of the Materials section (`specs/0028` ux-notes 10, 13).
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { decodeMaterialsText } from "../src/lib/materialsView.ts";
import {
  armText,
  fieldsHeight,
  hasTrash,
  line2Text,
  materialHeights,
  materialKeyAction,
  materialRowName,
  moveTarget,
  opensFields,
  rowTitle,
  saveFailureText,
  setArmText,
  shortName,
  undoFailureText,
  usedByText,
} from "../src/lib/materialRows.ts";

const DIR = new URL("../../curvyo-editor-wasm/tests/fixtures/materials_view/", import.meta.url);

function rows() {
  const view = decodeMaterialsText(readFileSync(new URL("damaged_newer_and_used.json", DIR), "utf8"));
  assert.ok(view);
  return view.rows;
}

test("accessible names follow the notes for loaded, damaged and newer rows", () => {
  const [birch, damaged, newer, cardboard] = rows();
  assert.equal(materialRowName(birch), "Birch plywood, 3.0 mm, Cut, Score and Engrave sets");
  assert.equal(materialRowName(cardboard), "Cardboard, 3.0 mm, Cut set");
  assert.equal(materialRowName(damaged), "Damaged material, Unknown key colour.");
  assert.equal(materialRowName(newer), "Birch plywood, needs a newer Curvyo");
  assert.equal(rowTitle(damaged), "Damaged material");
});

test("line 2 is the thickness, the reason or the newer words", () => {
  const [birch, damaged, newer] = rows();
  assert.equal(line2Text(birch), "3.0 mm");
  assert.equal(line2Text(damaged), "Unknown key colour.");
  assert.equal(line2Text(newer), "Needs a newer Curvyo");
});

test("a newer record has no trash and opens nothing; damaged has a trash and opens nothing", () => {
  const [birch, damaged, newer] = rows();
  assert.deepEqual([birch, damaged, newer].map(hasTrash), [true, true, false]);
  assert.deepEqual([birch, damaged, newer].map(opensFields), [true, false, false]);
});

test("the armed text names the use count and leaves it out at 0", () => {
  const [birch, , , cardboard] = rows();
  assert.equal(armText(birch), "Press again to delete Birch plywood.");
  assert.equal(armText(cardboard), "Press again to delete Cardboard. 1 operation here uses it.");
  assert.equal(armText({ ...cardboard, uses: 3 }), "Press again to delete Cardboard. 3 operations here use it.");
  assert.equal(setArmText("Cut", "Birch 3 mm"), "Press again to delete Cut set of Birch 3 mm.");
});

test("order keys: Alt+arrows, Home and End move, F2 renames, Delete arms then removes", () => {
  assert.equal(moveTarget(4, 2, "ArrowUp"), 1);
  assert.equal(moveTarget(4, 0, "ArrowUp"), null);
  assert.equal(moveTarget(4, 3, "ArrowDown"), null);
  assert.equal(moveTarget(4, 1, "End"), 3);
  assert.equal(moveTarget(4, 2, "Home"), 0);
  assert.equal(moveTarget(1, 0, "End"), null);
  assert.equal(materialKeyAction("ArrowUp", true, false), "move");
  assert.equal(materialKeyAction("ArrowUp", false, false), "none");
  assert.equal(materialKeyAction("F2", false, false), "rename");
  assert.equal(materialKeyAction("Delete", false, false), "arm");
  assert.equal(materialKeyAction("Delete", false, true), "remove");
  assert.equal(materialKeyAction("Enter", false, false), "open");
  assert.equal(materialKeyAction("Enter", false, true), "remove");
});

test("the window heights add the fields to the selected row only", () => {
  const list = rows();
  const fields = fieldsHeight(3, 1);
  assert.equal(fields, 32 + 20 + 3 * 104);
  assert.deepEqual(materialHeights(list, list[0].id, fields), [40 + fields, 40, 40, 40]);
  assert.deepEqual(materialHeights(list, null, fields), [40, 40, 40, 40]);
  assert.equal(fieldsHeight(1, 0), 136);
});

test("notice texts cut long names and stay short", () => {
  assert.equal(shortName("Birch 3 mm"), "Birch 3 mm");
  const long = "A very long material name that goes on";
  const cut = shortName(long);
  assert.equal(Array.from(cut).length, 24);
  assert.ok(cut.endsWith("…"));
  assert.equal(undoFailureText("undo", "Birch 3 mm", "storage full"), "Could not undo Birch 3 mm: storage full.");
  assert.equal(undoFailureText("redo", null, "storage full"), "Could not redo the material: storage full.");
  assert.ok(undoFailureText("undo", long, "storage full").length <= 70);
  assert.equal(saveFailureText("Birch 3 mm", "storage full"), "Could not save Birch 3 mm: storage full.");
});

test("the use line is absent at 0", () => {
  assert.equal(usedByText(0), null);
  assert.equal(usedByText(1), "Used by 1 operation here");
  assert.equal(usedByText(3), "Used by 3 operations here");
});
