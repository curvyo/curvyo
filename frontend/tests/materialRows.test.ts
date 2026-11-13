// The Materials tab decoder against the shared Rust fixtures (`curvyo-editor-wasm/tests/fixtures/
// materials_view/`, `specs/0112` criteria 16 to 28) and the row logic of the tab.
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync, readdirSync } from "node:fs";
import { decodeMaterialsText, decodeMaterialsView } from "../src/lib/materialsView.ts";
import {
  armText,
  hasTrash,
  laserCountText,
  line2Text,
  materialHeights,
  materialKeyAction,
  materialRowName,
  moveTarget,
  numberText,
  saveFailureText,
  setArmText,
  setFields,
  shortName,
  testArmText,
  testRowName,
  typedNumber,
} from "../src/lib/materialRows.ts";

const DIR = new URL("../../curvyo-editor-wasm/tests/fixtures/materials_view/", import.meta.url);

function raw(name: string): string {
  return readFileSync(new URL(`${name}.json`, DIR), "utf8");
}

function fixture(name: string): Record<string, unknown> {
  return JSON.parse(raw(name)) as Record<string, unknown>;
}

test("every Rust state decodes, and nothing else is in the folder", () => {
  const names = readdirSync(DIR)
    .filter((file) => file.endsWith(".json"))
    .map((file) => file.slice(0, -".json".length))
    .sort();
  assert.deepEqual(names, [
    "damaged_record",
    "empty",
    "format1_record",
    "test_with_photo",
    "test_without_photo",
    "two_lasers",
    "typed_set",
  ]);
  for (const name of names) assert.ok(decodeMaterialsText(raw(name)), name);
});

test("two lasers: the page's laser first and open, sets, missing kinds and tests newest first", () => {
  const view = decodeMaterialsText(raw("two_lasers"));
  assert.ok(view);
  assert.equal(view.pageLaser?.name, "Generic laser");
  assert.equal(view.rows.length, 1);
  const body = view.unfolded;
  assert.ok(body);
  assert.equal(body.lasers.length, 2);
  assert.equal(body.lasers[0].thisPage, true);
  assert.deepEqual(body.lasers[0].missing, ["cut", "engrave"]);
  assert.equal(body.lasers[1].sets[1].intervalMm, 0.08);
  assert.equal(body.lasers[1].sets[0].source, "Test 7K2QX");
  assert.deepEqual(body.tests.map((t) => t.id), ["4H8ZP", "8PL4D", "7K2QX"]);
  assert.equal(body.test, null);
  assert.equal(laserCountText(body.lasers[1]), "2 sets");
  assert.equal(laserCountText({ ...body.lasers[0], sets: [] }), "No sets");
});

test("a test with a photo reads its detail, best cell, set line and place button", () => {
  const view = decodeMaterialsText(raw("test_with_photo"));
  assert.ok(view?.unfolded?.test);
  const test = view.unfolded.test;
  assert.equal(test.id, "7K2QX");
  assert.deepEqual(test.best, { column: 2, row: 1 });
  assert.equal(test.columns.values.length, 5);
  assert.equal(test.setLine, "Cut set of Diode 20 W: 60 %, 1950 mm/min, 1 ×");
  assert.equal(test.photo, "0b6c3f1e-2a4d-4b8e-9c7f-1d2e3f4a5b6c.7K2QX.jpg");
  assert.deepEqual(test.place, { onPage: false, label: "Place on Laser 1" });
});

test("a test without a photo or best cell has none of them", () => {
  const view = decodeMaterialsText(raw("test_without_photo"));
  assert.ok(view?.unfolded?.test);
  const test = view.unfolded.test;
  assert.equal(test.best, null);
  assert.equal(test.setLine, null);
  assert.equal(test.photo, null);
  assert.equal(test.note, "");
  assert.equal(test.place.label, "Select on page");
});

test("damaged and older records are rows with a reason and no body", () => {
  const damaged = decodeMaterialsText(raw("damaged_record"));
  assert.ok(damaged);
  assert.equal(damaged.rows[0].state, "damaged");
  assert.equal(line2Text(damaged.rows[0]), "A Cut set has an interval.");
  assert.equal(hasTrash(damaged.rows[0]), true);
  assert.equal(damaged.loadNotices.length, 1);
  const older = decodeMaterialsText(raw("format1_record"));
  assert.ok(older);
  assert.equal(older.rows[0].state, "older");
  assert.equal(hasTrash(older.rows[0]), false);
  assert.equal(materialRowName(older.rows[0]), "Birch plywood, has an older format");
});

test("the empty tab decodes with no rows; a missing or mistyped field is refused", () => {
  const empty = decodeMaterialsText(raw("empty"));
  assert.ok(empty);
  assert.deepEqual(empty.rows, []);
  assert.equal(empty.nextName, "Material 1");
  assert.equal(decodeMaterialsText("null"), null);
  assert.equal(decodeMaterialsText("{"), null);
  const base = fixture("two_lasers");
  for (const key of ["rows", "form_open", "next_name", "load_notices"]) {
    const copy = { ...base };
    delete copy[key];
    assert.equal(decodeMaterialsView(copy), null, `missing ${key}`);
  }
  assert.equal(decodeMaterialsView({ ...base, next_name: 1 }), null);
  const unfolded = base.unfolded as Record<string, unknown>;
  assert.equal(decodeMaterialsView({ ...base, unfolded: { ...unfolded, thickness_mm: "3" } }), null);
  assert.equal(decodeMaterialsView({ ...base, unfolded: { ...unfolded, tests: [{ id: "x" }] } }), null);
  assert.equal(decodeMaterialsView({ ...base, rows: [{ state: "gone", id: "x" }] }), null);
  const detail = (fixture("test_with_photo").unfolded as Record<string, unknown>).test as Record<string, unknown>;
  const withDetail = (patch: Record<string, unknown>) => ({
    ...fixture("test_with_photo"),
    unfolded: { ...(fixture("test_with_photo").unfolded as object), test: { ...detail, ...patch } },
  });
  assert.ok(decodeMaterialsView(withDetail({})));
  assert.equal(decodeMaterialsView(withDetail({ best: { column: -1, row: 0 } })), null);
  assert.equal(decodeMaterialsView(withDetail({ place: { label: "x" } })), null);
  assert.equal(decodeMaterialsView(withDetail({ photo: 3 })), null);
  assert.equal(decodeMaterialsView(withDetail({ kind: "grind" })), null);
});

test("names, arm texts and line 2 follow the notes", () => {
  const view = decodeMaterialsText(raw("two_lasers"));
  assert.ok(view);
  const row = view.rows[0];
  assert.equal(materialRowName(row), "Birch plywood, 3.0 mm, 2 lasers · 3 sets · 3 tests");
  assert.equal(line2Text(row), "2 lasers · 3 sets · 3 tests");
  assert.equal(armText(row), "Press again to delete Birch plywood. Its 3 tests go with it.");
  assert.equal(armText({ ...(row as object), tests: 1 } as typeof row), "Press again to delete Birch plywood. Its 1 test goes with it.");
  assert.equal(armText({ ...(row as object), tests: 0 } as typeof row), "Press again to delete Birch plywood.");
  assert.equal(setArmText("cut", "Diode 20 W"), "Press again to delete the Cut set of Diode 20 W.");
  assert.equal(testArmText("K7Q2M", true), "Press again to delete test K7Q2M. Its photo goes with it.");
  assert.equal(testArmText("K7Q2M", false), "Press again to delete test K7Q2M.");
  const tests = view.unfolded?.tests ?? [];
  assert.equal(testRowName(tests[2]), "Test 7K2QX, Cut, Lasered 12 Nov, Diode 20 W, with photo");
  assert.equal(testRowName(tests[0]), "Test 4H8ZP, Engrave, Not lasered, not on a page");
  assert.ok(armText(row).length <= 70);
});

test("keys: Alt+arrows move, F2 renames, Delete arms then removes, Enter opens", () => {
  assert.equal(materialKeyAction("ArrowUp", true, false), "move");
  assert.equal(materialKeyAction("ArrowUp", false, false), "none");
  assert.equal(materialKeyAction("F2", false, false), "rename");
  assert.equal(materialKeyAction("Delete", false, false), "arm");
  assert.equal(materialKeyAction("Delete", false, true), "remove");
  assert.equal(materialKeyAction("Enter", false, false), "open");
  assert.equal(materialKeyAction(" ", false, false), "open");
  assert.equal(moveTarget(3, 0, "ArrowUp"), null);
  assert.equal(moveTarget(3, 0, "ArrowDown"), 1);
  assert.equal(moveTarget(3, 1, "End"), 2);
  assert.equal(moveTarget(1, 0, "End"), null);
});

test("the window gives the unfolded row its body and every other row 40 px", () => {
  const view = decodeMaterialsText(raw("two_lasers"));
  assert.ok(view);
  assert.deepEqual(materialHeights(view.rows, view.rows[0].id, 300), [340]);
  assert.deepEqual(materialHeights(view.rows, null, 300), [40]);
});

test("set fields: Engrave adds Interval; typed text is a number or nothing", () => {
  assert.deepEqual(setFields("cut"), ["power", "passes", "speed"]);
  assert.deepEqual(setFields("engrave"), ["power", "passes", "speed", "interval"]);
  assert.equal(typedNumber(" 12,5 "), 12.5);
  assert.equal(typedNumber(""), null);
  assert.equal(typedNumber("abc"), null);
  assert.equal(numberText(12.5), "12.5");
  assert.equal(numberText(0.080000001), "0.08");
});

test("notice texts cut a long name", () => {
  assert.equal(shortName("Birch"), "Birch");
  assert.equal(shortName("x".repeat(40)).length, 24);
  assert.equal(saveFailureText("Birch", "disk full"), "Could not save Birch: disk full.");
  assert.equal(saveFailureText(null, "disk full"), "Could not save the material: disk full.");
});
