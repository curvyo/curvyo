// Tester acceptance cases for `specs/0028-material-test-library` where a criterion is UI view
// only: the Worked cell named by word, never by colour (12), the Save as material form's checks
// and its Replace sentence (15), the damaged row's title and reason (29), the arming text that
// names the operations pointing to a material (21), the keyboard routes of the order (22) and
// the decoded Materials view of the Rust fixtures carrying no colour (19).
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { cellName, replacesSentence, validateSave } from "../src/lib/testGridResult.ts";
import { armText, materialKeyAction, materialRowName, moveTarget, rowTitle } from "../src/lib/materialRows.ts";
import { decodeMaterialsText } from "../src/lib/materialsView.ts";

const MATERIALS = new URL("../../curvyo-editor-wasm/tests/fixtures/materials_view/", import.meta.url);

test("c12: the Worked cell is named by the word Worked, the others are not", () => {
  assert.equal(cellName(60, 3000, true), "60 % power, 3000 mm/min, Worked");
  assert.equal(cellName(60, 3000, false), "60 % power, 3000 mm/min");
  assert.equal(cellName(37.5, 1950, false), "37.5 % power, 1950 mm/min");
});

test("c15: the form refuses an empty name, a thickness outside 0.1 to 100 and passes outside 1 to 20", () => {
  const ok = { existing: "", name: "Birch", thickness: "3", passes: "1" };
  assert.equal(validateSave(ok), null);
  assert.equal(validateSave({ ...ok, name: "  " })?.field, "name");
  assert.equal(validateSave({ ...ok, thickness: "" })?.field, "thickness", "thickness is required");
  assert.equal(validateSave({ ...ok, thickness: "0.09" })?.field, "thickness");
  assert.equal(validateSave({ ...ok, thickness: "100.1" })?.field, "thickness");
  assert.equal(validateSave({ ...ok, thickness: "100" }), null);
  assert.equal(validateSave({ ...ok, passes: "0" })?.field, "passes");
  assert.equal(validateSave({ ...ok, passes: "21" })?.field, "passes");
  assert.equal(validateSave({ ...ok, passes: "1.5" })?.field, "passes");
  // Adding to an existing material asks only for passes.
  assert.equal(validateSave({ existing: "x", name: "", thickness: "", passes: "2" }), null);
});

test("c15: the Replace sentence names the kind, the numbers and the test date", () => {
  assert.equal(
    replacesSentence("Cut", 60, 3000, "Tested 2026-11-10, 5 × 5 grid, cell 60 %, 3000 mm/min"),
    "Replaces the Cut set (60 %, 3000 mm/min, tested 2026-11-10).",
  );
});

test("c29: a damaged record is listed as Damaged material with its reason", () => {
  const row = { state: "damaged", id: "x", reason: "Unknown key colour." } as const;
  assert.equal(rowTitle(row), "Damaged material");
  assert.equal(materialRowName(row), "Damaged material, Unknown key colour.");
});

test("c21: the arming text names how many operations point to the material", () => {
  const row = {
    state: "loaded",
    id: "x",
    name: "Birch",
    thicknessMm: 3,
    kinds: ["cut"],
    uses: 2,
  } as const;
  assert.equal(armText(row), "Press again to delete Birch. 2 operations here use it.");
  assert.equal(armText({ ...row, uses: 1 }), "Press again to delete Birch. 1 operation here uses it.");
  assert.equal(armText({ ...row, uses: 0 }), "Press again to delete Birch.");
});

test("c22: Alt+Up and Alt+Down move a row; plain arrows do not", () => {
  assert.equal(materialKeyAction("ArrowUp", true, false), "move");
  assert.equal(materialKeyAction("ArrowDown", true, false), "move");
  assert.equal(materialKeyAction("ArrowUp", false, false), "none");
  assert.equal(moveTarget(3, 0, "ArrowUp"), null);
  assert.equal(moveTarget(3, 0, "ArrowDown"), 1);
  assert.equal(moveTarget(3, 2, "ArrowDown"), null);
  assert.equal(moveTarget(1, 0, "ArrowDown"), null);
});

test("c19: the decoded Materials view carries no colour", () => {
  for (const name of ["empty", "selected_fields", "damaged_newer_and_used"]) {
    const text = readFileSync(new URL(`${name}.json`, MATERIALS), "utf8");
    const view = decodeMaterialsText(text);
    assert.ok(view, name);
    const json = JSON.stringify(view);
    assert.ok(!/colou?r|#[0-9a-f]{3,6}\b|rgb\(/i.test(json.replaceAll("Unknown key colour.", "")), name);
  }
});
