// The Test grid bar decoder against the shared Rust fixtures (`curvyo-editor-wasm/tests/fixtures/
// test_grid_view/`, `specs/0112` criteria 3 to 6, 11) and the field logic of the bar.
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync, readdirSync } from "node:fs";
import { decodeTestGridText, decodeTestGridView } from "../src/lib/testGridView.ts";
import {
  barName,
  causeOf,
  fieldText,
  filterMaterials,
  materialButtonText,
  plainNumber,
  sharedValue,
  steppedText,
  thicknessLabel,
} from "../src/lib/testGridBar.ts";

const DIR = new URL("../../curvyo-editor-wasm/tests/fixtures/test_grid_view/", import.meta.url);

function raw(name: string): string {
  return readFileSync(new URL(`${name}.json`, DIR), "utf8");
}

function parsed(name: string): Record<string, unknown> {
  return JSON.parse(raw(name)) as Record<string, unknown>;
}

test("every Rust state decodes, and nothing else is in the folder", () => {
  const names = readdirSync(DIR)
    .filter((file) => file.endsWith(".json"))
    .map((file) => file.slice(0, -".json".length))
    .sort();
  assert.deepEqual(names, ["does_not_fit", "engrave_crosshatch", "invalid_value", "lasered", "no_material"]);
  for (const name of names) assert.ok(decodeTestGridText(raw(name)), name);
});

test("a plain grid reads its id, axes and shared values", () => {
  const view = decodeTestGridText(raw("no_material"));
  assert.ok(view);
  assert.deepEqual(view.group, { peer: "5", counter: 0 });
  assert.equal(view.id, "M8W1O");
  assert.equal(view.kind, "cut");
  assert.equal(view.material, null);
  assert.equal(view.crosshatch, null);
  assert.equal(view.columns.param, "power");
  assert.deepEqual(view.columns.offered, ["power", "speed", "passes"]);
  assert.equal(view.rows.param, "speed");
  assert.equal(view.rows.count, 5);
  assert.equal(view.cellMm, 10);
  assert.equal(sharedValue(view, "passes"), 1);
  assert.equal(sharedValue(view, "interval"), null);
  assert.equal(barName(view), "Test grid M8W1O");
});

test("an Engrave grid carries Crosshatch and Interval", () => {
  const view = decodeTestGridText(raw("engrave_crosshatch"));
  assert.ok(view);
  assert.equal(view.crosshatch, true);
  assert.deepEqual(view.columns.offered, ["power", "speed", "passes", "interval"]);
  assert.equal(sharedValue(view, "interval"), 0.1);
});

test("a Lasered grid names its material and the dropdown", () => {
  const view = decodeTestGridText(raw("lasered"));
  assert.ok(view);
  assert.equal(view.lasered, true);
  assert.equal(view.material?.name, "Birch 3 mm");
  assert.equal(view.materials.length, 1);
  assert.deepEqual(materialButtonText(view), { text: "Birch 3 mm", picked: true });
  assert.equal(thicknessLabel(view.materials[0]), "3.0 mm");
});

test("a value the grid did not take stays as typed with its cause", () => {
  const view = decodeTestGridText(raw("invalid_value"));
  assert.ok(view);
  assert.equal(fieldText(view, "cell", view.cellMm), "60");
  assert.equal(causeOf(view, "cell"), "Cell size is 3 to 50 mm.");
  assert.equal(causeOf(view, "passes"), null);
  assert.equal(fieldText(view, "passes", 1), "1");
});

test("a grid wider than the bed carries the notice and no id from the draft", () => {
  const view = decodeTestGridText(raw("does_not_fit"));
  assert.ok(view);
  assert.equal(view.fit, "Grid is 639.7 mm wide, the bed is 400 mm.");
  assert.equal(view.group, null);
  assert.equal(view.id, null);
  assert.equal(barName(view), "Test grid");
});

test("a missing field or a wrong type decodes to null", () => {
  const base = parsed("no_material");
  for (const key of Object.keys(base)) {
    const copy = { ...base };
    delete copy[key];
    assert.equal(decodeTestGridView(copy), null, `missing ${key}`);
  }
  assert.equal(decodeTestGridView({ ...base, kind: "grind" }), null);
  assert.equal(decodeTestGridView({ ...base, cell_mm: "10" }), null);
  assert.equal(decodeTestGridView({ ...base, lasered: 1 }), null);
  assert.equal(decodeTestGridView({ ...base, shared: [{ param: "heat", value: 1 }] }), null);
  assert.equal(decodeTestGridView({ ...base, columns: { ...(base.columns as object), param: "heat" } }), null);
  assert.equal(decodeTestGridView({ ...base, invalid: { field: "cell" } }), null);
  assert.equal(decodeTestGridView({ ...base, material: { id: "x" } }), null);
  assert.equal(decodeTestGridView(null), null);
  assert.equal(decodeTestGridText("null"), null);
  assert.equal(decodeTestGridText("{"), null);
});

test("Up and Down step a number, Shift takes ten times, committed text is plain", () => {
  assert.equal(steppedText("50", 1, 1, false), "51");
  assert.equal(steppedText("50", 1, -1, true), "40");
  assert.equal(steppedText("3000", 100, 1, false), "3100");
  assert.equal(steppedText("0.1", 0.01, 1, false), "0.11");
  assert.equal(steppedText("0.1", 0.01, -1, true), "0");
  assert.equal(steppedText("", 1, 1, false), null);
  assert.equal(steppedText("abc", 1, 1, false), null);
  assert.equal(plainNumber(0.30000000000000004), "0.3");
});

test("the dropdown filters by name", () => {
  const list = [
    { id: "a", name: "Birch 3 mm", thicknessMm: 3 },
    { id: "b", name: "Walnut", thicknessMm: 4 },
  ];
  assert.deepEqual(filterMaterials(list, " WAL ").map((one) => one.id), ["b"]);
  assert.equal(filterMaterials(list, "").length, 2);
});
