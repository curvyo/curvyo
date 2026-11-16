// The Fixtures tab's decoder against the Rust fixtures of `curvyo-editor-wasm/tests/fixtures/
// fixtures_view/` (`specs/0102-laser-fixtures` `adrs.md` section 7, `specs/0113-fixture-sets-and-pins`
// criteria 1 to 43). The Rust contract test `tests/fixture_sets_view_contract.rs` builds the same
// states and compares them with these files, so a drift fails on both sides.
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync, readdirSync } from "node:fs";
import { lockTooltip } from "../src/lib/fixtureSets.ts";
import { allRows, decodeFixturesText, decodeFixturesView, selectedLockBar } from "../src/lib/fixturesView.ts";
import { breakEveryField } from "./contractWalk.ts";

const DIR = new URL("../../curvyo-editor-wasm/tests/fixtures/fixtures_view/", import.meta.url);

function fixture(name: string): unknown {
  return JSON.parse(readFileSync(new URL(`${name}.json`, DIR), "utf8"));
}

const NAMES = readdirSync(DIR)
  .filter((file) => file.endsWith(".json"))
  .map((file) => file.slice(0, -".json".length))
  .sort();

function decoded(name: string) {
  const view = decodeFixturesView(fixture(name));
  assert.ok(view, name);
  return view;
}

test("the twelve Rust states decode, and nothing else is in the folder", () => {
  assert.deepEqual(NAMES, [
    "apply_candidates",
    "damaged_set",
    "empty_page",
    "every_kind",
    "generic_laser_hint",
    "locked_pin_grid",
    "locked_set",
    "old_file_job_fixtures",
    "read_only_sets",
    "screw_grid_centre_mark",
    "selected_holder",
    "two_sets_one_off",
  ]);
  for (const name of NAMES) decoded(name);
});

test("every field of every shared fixture is required and typed: a missing or mistyped one is refused", () => {
  for (const name of NAMES) {
    for (const broken of breakEveryField(fixture(name))) {
      assert.equal(decodeFixturesView(broken.value), null, `${name}: ${broken.how} ${broken.path} must be refused`);
    }
  }
});

test("the empty page has no set and no job fixture, and may add both", () => {
  const view = decoded("empty_page");
  assert.deepEqual(view.sets, []);
  assert.deepEqual(view.jobFixtures, []);
  assert.equal(view.setsOnPage, 0);
  assert.equal(view.canAddSet, true);
  assert.equal(view.canAddJobFixture, true);
  assert.equal(view.armedKind, null);
  assert.equal(view.selected, null);
  assert.equal(view.snappingFixtures, true);
});

test("two sets: one on the page and unfolded with its fixtures, one off with its screw", () => {
  const view = decoded("two_sets_one_off");
  assert.equal(view.setsOnPage, 1);
  assert.deepEqual(
    view.sets.map((set) => [set.name, set.onPage, set.unfolded, set.canAddGrid, set.line2]),
    [
      ["Set 1", true, true, false, "Grid, Holder · 2 fixtures"],
      ["Coaster jig", false, false, true, "Screw · not on this page"],
    ],
  );
  const [grid, holder] = view.sets[0].fixtures;
  assert.equal(grid.grid?.spacingMm, 10);
  assert.deepEqual([grid.locked, grid.lockedBySet], [false, false], "a set fixture has its own Lock (criterion 32)");
  assert.equal(holder.active, false);
  assert.equal(holder.onBed, false);
  assert.equal(holder.slots.length, 2);
  const screw = view.sets[1].fixtures[0];
  assert.equal(screw.active, true);
  assert.equal(screw.onBed, false, "its own check is on, its set is off: not on the bed");
  assert.equal(screw.round?.diameterMm, null, "a screw has no Diameter field");
});

test("a locked set locks its fixtures, a locked fixture alone does not lock its set (criterion 32)", () => {
  const view = decoded("locked_set");
  assert.deepEqual(
    view.sets.map((set) => [set.locked, set.canAdd]),
    [
      [true, false],
      [false, true],
    ],
  );
  const [holder] = view.sets[0].fixtures;
  assert.deepEqual([holder.locked, holder.lockedBySet], [false, true]);
  const [clamp] = view.sets[1].fixtures;
  assert.deepEqual([clamp.locked, clamp.lockedBySet], [true, false]);
  assert.equal(view.selected, clamp.id);
});

test("a read only page carries its line and no add; Generic laser carries its hint (criteria 3, 46)", () => {
  const readOnly = decoded("read_only_sets");
  assert.equal(readOnly.readOnly, "From the project file. Generic laser copy is not on this computer.");
  assert.equal(readOnly.profileHint, null);
  assert.equal(readOnly.canAddSet, false);
  assert.ok(readOnly.sets.every((set) => !set.canAdd));
  const generic = decoded("generic_laser_hint");
  assert.equal(generic.readOnly, null);
  assert.match(generic.profileHint ?? "", /^Fixture sets are kept in a machine profile\./);
  assert.equal(generic.canAddSet, false);
});

test("a locked pin grid carries its counts, spacings and the toggle states", () => {
  const view = decoded("locked_pin_grid");
  const pin = view.jobFixtures[0];
  assert.equal(pin.kind, "pin");
  assert.equal(pin.locked, true);
  assert.equal(pin.line2, "Pin · 2 × 2 · ⌀ 6 mm · locked · Diameters");
  assert.equal(pin.round?.diameterMm, 6);
  assert.deepEqual([pin.round?.columns, pin.round?.rows], [2, 2]);
  assert.deepEqual([pin.round?.columnSpacingMm, pin.round?.rowSpacingMm], [30, 20]);
  assert.equal(pin.round?.centreMark?.on, false);
  assert.equal(pin.round?.diameters?.on, true);
  assert.equal(view.selected, pin.id);
  assert.equal(view.selectionAnnouncement, "Pin 1 selected, diameter 6.0 mm.");
});

test("a screw grid with its Centre mark on", () => {
  const screw = decoded("screw_grid_centre_mark").jobFixtures[0];
  assert.equal(screw.kind, "screw");
  assert.equal(screw.round?.columns, 4);
  assert.equal(screw.round?.centreMark?.on, true);
  assert.equal(screw.round?.markMm, 3);
});

test("an old file's job fixtures: a screw has no role jobs, so no toggles (criterion 39)", () => {
  const view = decoded("old_file_job_fixtures");
  assert.deepEqual(view.sets, []);
  const screw = view.jobFixtures.find((row) => row.kind === "screw");
  assert.ok(screw);
  assert.equal(screw.round?.centreMark, null);
  assert.equal(screw.round?.diameters, null);
  assert.ok(view.jobFixtures.every((row) => row.locked === false));
});

test("Apply candidates and the picked page decode", () => {
  const view = decoded("apply_candidates");
  assert.deepEqual(view.applyCandidates, [{ id: "v9n291gl1jnw2", name: "Laser 1", count: 2 }]);
  assert.equal(view.applyPicked, "v9n291gl1jnw2");
});

test("damage notices come per set and for the job fixtures (criterion 40)", () => {
  assert.deepEqual(decoded("damaged_set").damageNotices, [
    "3 values damaged in Set 1.",
    "3 values damaged in Set 2.",
    "1 value damaged in No id.",
    "5 values damaged in Job fixtures.",
  ]);
});

test("every kind decodes, an unknown kind stays a row, and rows list sets before job fixtures", () => {
  const view = decoded("every_kind");
  assert.deepEqual(
    view.jobFixtures.map((row) => row.kind),
    ["grid", "holder", "clamp", "stop", "screw", "unknown"],
  );
  assert.deepEqual(allRows(decoded("two_sets_one_off")).map((row) => row.name), ["Grid 1", "Holder 1", "Screw 1"]);
});

test("a selected holder's announcement decodes, and a number is refused", () => {
  const view = decoded("selected_holder");
  assert.equal(view.selected, view.jobFixtures[1].id);
  assert.equal(view.selectionAnnouncement, "Holder 1 selected, 120.0 by 80.0 mm.");
  assert.equal(decoded("every_kind").selectionAnnouncement, null);
  const bad = { ...(fixture("selected_holder") as Record<string, unknown>), selection_announcement: 7 };
  assert.equal(decodeFixturesView(bad), null);
});

test("a kind text this build does not know is 'unknown', never a failed decode", () => {
  const good = fixture("every_kind") as { job_fixtures: Array<Record<string, unknown>> };
  const teleport = structuredClone(good);
  teleport.job_fixtures[0].kind = "teleport";
  assert.equal(decodeFixturesView(teleport)?.jobFixtures[0].kind, "unknown");
});

test("the text form decodes; the JSON null of a page without fixtures, bad JSON and other values are null", () => {
  assert.equal(decodeFixturesText("null"), null);
  assert.equal(decodeFixturesText("not json"), null);
  assert.equal(decodeFixturesText(""), null);
  assert.equal(decodeFixturesView([fixture("empty_page")]), null);
  assert.equal(decodeFixturesView("null"), null);
  assert.equal(decodeFixturesText(JSON.stringify(fixture("empty_page")))?.sets.length, 0);
});

test("the Select bar's Lock follows the selected fixture, job or set, and hides on a read only page (criterion 32)", () => {
  const bar = selectedLockBar(decoded("locked_set"));
  assert.equal(bar?.row.name, "Clamp 1");
  assert.equal(bar?.setName, "Set 2");
  assert.equal(selectedLockBar(decoded("locked_pin_grid"))?.setName, null);
  assert.equal(selectedLockBar(decoded("empty_page")), null);
  const readOnly = decoded("read_only_sets");
  assert.equal(selectedLockBar({ ...readOnly, selected: readOnly.sets[0].fixtures[0].id }), null);
});

test("the Lock tooltip names the set while it holds the fixture still", () => {
  assert.equal(lockTooltip(false, null), "Lock");
  assert.equal(lockTooltip(true, null), "Unlock");
  assert.equal(lockTooltip(true, "Set 1"), "Locked with Set 1");
});
