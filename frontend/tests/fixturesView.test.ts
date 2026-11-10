// The Fixtures block's decoder against the Rust fixtures of `curvyo-editor-wasm/tests/fixtures/
// fixtures_view/` (`specs/0102-laser-fixtures` `adrs.md` section 7). The Rust contract test
// `tests/fixtures_view_contract.rs` builds the same states and compares them with these files, so
// a drift fails on both sides.
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync, readdirSync } from "node:fs";
import { decodeFixturesText, decodeFixturesView } from "../src/lib/fixturesView.ts";

const DIR = new URL("../../curvyo-editor-wasm/tests/fixtures/fixtures_view/", import.meta.url);

function fixture(name: string): unknown {
  return JSON.parse(readFileSync(new URL(`${name}.json`, DIR), "utf8"));
}

const NAMES = readdirSync(DIR)
  .filter((file) => file.endsWith(".json"))
  .map((file) => file.slice(0, -".json".length))
  .sort();

test("the Rust states decode, and nothing else is in the folder", () => {
  assert.deepEqual(NAMES, ["empty", "every_kind", "selected_holder"]);
  for (const name of NAMES) {
    assert.ok(decodeFixturesView(fixture(name)), name);
  }
});

test("the selection's announcement decodes for a selected holder and is null without one", () => {
  const selected = decodeFixturesView(fixture("selected_holder"));
  assert.ok(selected);
  assert.equal(selected.selected, selected.rows[1].id);
  assert.equal(selected.selectionAnnouncement, "Holder 1 selected, 120.0 by 80.0 mm.");
  assert.equal(decodeFixturesView(fixture("every_kind"))?.selectionAnnouncement, null);
  assert.equal(decodeFixturesView(fixture("empty"))?.selectionAnnouncement, null);
  const good = fixture("selected_holder") as Record<string, unknown>;
  assert.equal(decodeFixturesView({ ...good, selection_announcement: 7 }), null, "a number is refused");
  assert.ok(!("selected_pose" in good), "no pose: the Select bar shows nothing for a fixture");
});

test("the empty page has no rows and may add a grid", () => {
  const view = decodeFixturesView(fixture("empty"));
  assert.ok(view);
  assert.deepEqual(view.rows, []);
  assert.equal(view.canAddGrid, true);
  assert.equal(view.selected, null);
  assert.equal(view.snappingFixtures, true);
});

test("every kind decodes with its own values, and an unknown kind stays a row", () => {
  const view = decodeFixturesView(fixture("every_kind"));
  assert.ok(view);
  assert.deepEqual(
    view.rows.map((row) => row.kind),
    ["grid", "holder", "clamp", "stop", "screw", "unknown"],
  );
  const grid = view.rows[0];
  assert.equal(grid.grid?.spacingMm, 5);
  assert.equal(grid.grid?.majorEvery, 4);
  assert.deepEqual(grid.grid?.tapes, { bottom: true, left: true, top: false, right: true });
  assert.equal(grid.grid?.originXMm, 2);
  assert.equal(view.canAddGrid, false);
  const holder = view.rows[1];
  assert.ok(!("size" in holder), "no size field: the handles size a holder");
  assert.equal(holder.canAddSlot, true);
  assert.equal(holder.slots[0].shape, "rect");
  assert.equal(holder.slots[1].shape, "circle");
  assert.equal(holder.slots[2].outside, true);
  assert.equal(view.rows[3].visible, false);
  assert.equal(view.rows[2].active, false);
  assert.equal(view.rows[4].name, "Screw 1");
  assert.equal(view.rows[4].grid, null, "a screw's row has no field");
  assert.equal(view.rows[5].name, "Pocket 1");
  assert.deepEqual(view.damageNotices, ["1 value damaged on Stop 1."]);
});

test("ids are the full id text the calls take back", () => {
  const view = decodeFixturesView(fixture("every_kind"));
  assert.ok(view);
  assert.equal(view.rows[0].id, "hhpykmt2rhhv3");
  assert.equal(view.rows[1].slots[0].id, "jmwbnf98nodw2");
});

test("a wrong enum value or a missing field is refused, not guessed (checked decode)", () => {
  const good = fixture("every_kind") as Record<string, unknown>;
  const rows = () => structuredClone(good) as { rows: Array<Record<string, unknown>> };

  // A kind this build does not know is `unknown` (criterion 32), never a failed decode.
  const teleport = rows();
  teleport.rows[0].kind = "teleport";
  assert.equal(decodeFixturesView(teleport)?.rows[0]?.kind, "unknown");

  const badShape = rows() as { rows: Array<{ slots: Array<Record<string, unknown>> }> };
  badShape.rows[1].slots[0].shape = "hexagon";
  assert.equal(decodeFixturesView(badShape), null);

  const badTape = rows() as { rows: Array<{ grid: { tapes: Record<string, unknown> } }> };
  badTape.rows[0].grid.tapes.left = "yes";
  assert.equal(decodeFixturesView(badTape), null);

  const negativeMajor = rows();
  (negativeMajor.rows[0].grid as Record<string, unknown>).major_every = -1;
  assert.equal(decodeFixturesView(negativeMajor), null);

  const noSwitch = structuredClone(good) as Record<string, unknown>;
  delete noSwitch.snapping_fixtures;
  assert.equal(decodeFixturesView(noSwitch), null);

  assert.equal(decodeFixturesView(null), null);
  assert.equal(decodeFixturesView("null"), null);
});

test("the text form decodes, and the JSON null of a page without fixtures is null", () => {
  assert.equal(decodeFixturesText("null"), null);
  assert.equal(decodeFixturesText("not json"), null);
  const view = decodeFixturesText(JSON.stringify(fixture("empty")));
  assert.ok(view);
  assert.equal(view.rows.length, 0);
});

test("hostile JSON of the wrong shape is refused as a whole, never partly read", () => {
  const good = fixture("every_kind") as Record<string, unknown>;
  const withRows = (rows: unknown) => ({ ...good, rows });
  const cases: Array<[string, unknown]> = [
    ["rows as a string", withRows("holder")],
    ["rows as an object", withRows({ 0: good })],
    ["a row that is a number", withRows([1])],
    ["an id as a number", withRows([{ ...(good.rows as Array<Record<string, unknown>>)[1], id: 7 }])],
    ["spacing as a string", withRows([{ ...(good.rows as Array<Record<string, unknown>>)[0], grid: { ...((good.rows as Array<{ grid: object }>)[0].grid), spacing_mm: "5" } }])],
    ["damage notices with a number", { ...good, damage_notices: [3] }],
    ["damage notices as a string", { ...good, damage_notices: "1 value damaged" }],
    ["selected as a number", { ...good, selected: 4 }],
    ["snapping as a string", { ...good, snapping_fixtures: "yes" }],
    ["a bare array", [good]],
  ];
  for (const [name, value] of cases) {
    assert.equal(decodeFixturesView(value), null, name);
  }
});

test("an empty view decodes with no rows, and a text that is not JSON is null", () => {
  const view = decodeFixturesView(fixture("empty"));
  assert.ok(view);
  assert.deepEqual(view.rows, []);
  assert.deepEqual(view.damageNotices, []);
  assert.equal(view.selected, null);
  assert.equal(view.armed, null);
  assert.equal(decodeFixturesText("{"), null);
  assert.equal(decodeFixturesText(""), null);
});
