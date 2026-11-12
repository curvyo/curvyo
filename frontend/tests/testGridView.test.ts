// The Grid block decoder, its fields and the result helpers against the Rust fixtures of
// `curvyo-editor-wasm/tests/fixtures/test_grid_view/` (`specs/0028-material-test-library`
// `adrs.md` section 6). The Rust contract test builds the same states, so a drift fails on both sides.
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync, readdirSync } from "node:fs";
import { decodeAddResult, decodeTestGridText, decodeTestGridView } from "../src/lib/testGridView.ts";
import { fieldShown, gridRowLine, sizeLine } from "../src/lib/testGridFields.ts";
import { digitRow, rowDigit } from "../src/lib/operationRows.ts";
import type { OperationRow, OperationsView } from "../src/lib/operationsView.ts";
import {
  cellName,
  cellWidth,
  markStatus,
  moveCell,
  passesOf,
  replacesSentence,
  savedStatus,
  sparseHeader,
  thicknessOf,
  validateSave,
} from "../src/lib/testGridResult.ts";

const DIR = new URL("../../curvyo-editor-wasm/tests/fixtures/test_grid_view/", import.meta.url);

function raw(name: string): string {
  return readFileSync(new URL(`${name}.json`, DIR), "utf8");
}

test("every Rust state decodes, and nothing else is in the folder", () => {
  const names = readdirSync(DIR)
    .filter((file) => file.endsWith(".json"))
    .map((file) => file.slice(0, -".json".length))
    .sort();
  assert.deepEqual(names, ["default_unmarked", "does_not_fit", "invalid_value", "marked"]);
  for (const name of names) assert.ok(decodeTestGridText(raw(name)), name);
});

test("the default grid reads its fields, labels and result heads", () => {
  const view = decodeTestGridText(raw("default_unmarked"));
  assert.ok(view);
  assert.equal(view.name, "Test grid 1");
  assert.equal(view.kind, "cut");
  assert.deepEqual(view.powers, [20, 40, 60, 80, 100]);
  assert.deepEqual(view.speeds, [600, 1950, 3300, 4650, 6000]);
  assert.equal(view.worked, null);
  assert.equal(view.handEdited, false);
  assert.equal(view.labels.on, true);
  assert.equal(fieldShown(view, "columns"), "5");
  assert.equal(fieldShown(view, "power_from"), "20");
  assert.equal(fieldShown(view, "speed_to"), "6000");
  assert.equal(fieldShown(view, "label_speed"), "3000");
  assert.equal(sizeLine(view), "Generic laser · 72.1 × 70 mm");
});

test("a marked grid carries the Worked line and its cell", () => {
  const view = decodeTestGridText(raw("marked"));
  assert.deepEqual(view?.worked?.cell, { column: 2, row: 1 });
  assert.equal(view?.worked?.text, "Worked: 60 % power, 1950 mm/min");
});

test("a value the grid did not take is decoded with its chip; the fields keep the grid's own numbers", () => {
  const view = decodeTestGridText(raw("invalid_value"));
  assert.equal(view?.invalid?.chip, "Columns are 2 to 12.");
  assert.equal(view?.invalid?.typed, "13");
  assert.equal(fieldShown(view!, "columns"), "5");
  assert.equal(fieldShown(view!, "rows"), "5");
});

test("a missing field or a wrong type decodes to null", () => {
  const good = JSON.parse(raw("default_unmarked")) as Record<string, unknown>;
  for (const key of Object.keys(good)) {
    const { [key]: _gone, ...rest } = good;
    assert.equal(decodeTestGridView(rest), null, `without ${key}`);
  }
  assert.equal(decodeTestGridView({ ...good, kind: "hatch" }), null);
  assert.equal(decodeTestGridView({ ...good, hand_edited: "no" }), null);
  assert.equal(decodeTestGridView(null), null);
  assert.equal(decodeTestGridText("not json"), null);
});

test("the add answer is a row or a refusal", () => {
  assert.deepEqual(decodeAddResult('{"ok":true,"row":{"peer":"3","counter":427}}'), {
    ok: true,
    row: { peer: "3", counter: 427 },
  });
  assert.deepEqual(decodeAddResult('{"ok":false,"text":"Test grid is 260 mm wide, the bed is 200 mm."}'), {
    ok: false,
    text: "Test grid is 260 mm wide, the bed is 200 mm.",
  });
  assert.deepEqual(decodeAddResult("{}"), { ok: false, text: "" });
});

test("the row line has no digit and names the size, or no grid", () => {
  assert.equal(gridRowLine("Cut", 5, 5, true), "Cut test grid · 5 × 5 cells");
  assert.equal(gridRowLine("Score", 3, 4, false), "Score test grid · 3 × 4 cells · Output off");
  assert.equal(gridRowLine("Cut", 0, 0, true), "Cut test grid · no grid");
});

test("the result table: cell widths share 204 px from 9 columns, names end with Worked, keys roam", () => {
  assert.equal(cellWidth(8), 24);
  assert.equal(cellWidth(12), 17);
  assert.equal(cellName(60, 3000, false), "60 % power, 3000 mm/min");
  assert.equal(cellName(62.5, 3000, true), "62.5 % power, 3000 mm/min, Worked");
  const at = { column: 1, row: 1 };
  assert.deepEqual(moveCell(at, "ArrowLeft", false, 5, 5), { column: 0, row: 1 });
  assert.deepEqual(moveCell({ column: 0, row: 0 }, "ArrowUp", false, 5, 5), { column: 0, row: 0 });
  assert.deepEqual(moveCell(at, "End", false, 5, 4), { column: 4, row: 1 });
  assert.deepEqual(moveCell(at, "End", true, 5, 4), { column: 4, row: 3 });
  assert.deepEqual(moveCell(at, "Home", true, 5, 4), { column: 0, row: 0 });
  assert.equal(moveCell(at, "Enter", false, 5, 5), null);
});

test("Save as material validates in the order of the form and names the first wrong field", () => {
  const draft = { existing: "", name: "Birch", thickness: "3", passes: "1" };
  assert.equal(validateSave(draft), null);
  assert.deepEqual(validateSave({ ...draft, name: "  " }), { field: "name", text: "Enter a name." });
  assert.deepEqual(validateSave({ ...draft, thickness: "" }), { field: "thickness", text: "Enter a thickness." });
  assert.deepEqual(validateSave({ ...draft, thickness: "100.1" }), { field: "thickness", text: "Thickness is 0.1 to 100 mm." });
  assert.deepEqual(validateSave({ ...draft, passes: "21" }), { field: "passes", text: "Passes are 1 to 20." });
  assert.equal(validateSave({ existing: "abc", name: "", thickness: "", passes: "2" }), null);
  assert.equal(thicknessOf({ ...draft, thickness: "3,5" }), 3.5);
  assert.equal(passesOf(draft), 1);
  assert.equal(replacesSentence("Cut", 60, 3000, "Tested 2026-11-10, 5 × 5 grid"), "Replaces the Cut set (60 %, 3000 mm/min, tested 2026-11-10).");
  assert.equal(savedStatus("Birch 3 mm"), "Birch 3 mm saved.");
});

test("the header row draws every other power only when a label is wider than its cell", () => {
  assert.equal(sparseHeader([20, 40, 60, 80, 100], cellWidth(5)), false);
  assert.equal(sparseHeader([20, 30, 40, 50, 60, 70, 80, 90, 100], cellWidth(9)), false);
  const twelve = Array.from({ length: 12 }, (_, i) => 20 + (80 / 11) * i);
  assert.equal(sparseHeader(twelve, cellWidth(12)), true);
});

test("a press in the result table is announced as a mark or its removal", () => {
  assert.equal(markStatus(60, 3000, true), "Cell marked: 60 % power, 3000 mm/min.");
  assert.equal(markStatus(60, 3000, false), "Mark removed.");
});

test("digits skip test grid rows: the rows after a grid count on past it (UX notes §5, §12)", () => {
  const row = (name: string, grid: OperationRow["grid"]): OperationRow =>
    ({ id: { peer: "1", counter: name.length }, name, kind: "cut", section: "cutting", show: true, output: true,
      marginMm: 0, items: 0, tick: "none", settings: null, grid }) as OperationRow;
  const view = {
    cutsLast: true,
    rows: [row("Cut 1", null), row("Test grid 1", { group: null, columns: 5, rows: 5 }), row("Cut 2", null)],
  } as unknown as OperationsView;
  assert.deepEqual([0, 1, 2].map((index) => rowDigit(view, index)), [1, null, 2]);
  assert.equal(digitRow(view, 2)?.name, "Cut 2");
  assert.equal(digitRow(view, 3), null);
});
