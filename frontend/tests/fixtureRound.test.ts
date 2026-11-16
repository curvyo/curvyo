// The pin and screw body (`specs/0113-fixture-sets-and-pins` `ux-notes.md` section 6.1): which
// fields and toggles show for the shared fixtures, and how typed text becomes a value.
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { decodeFixturesView } from "../src/lib/fixturesView.ts";
import { fieldText, fieldValue, hasRoundBody, roundBody } from "../src/lib/fixtureRound.ts";

const DIR = new URL("../../curvyo-editor-wasm/tests/fixtures/fixtures_view/", import.meta.url);

function round(name: string, index = 0) {
  const view = decodeFixturesView(JSON.parse(readFileSync(new URL(`${name}.json`, DIR), "utf8")));
  const values = view?.jobFixtures[index]?.round;
  assert.ok(values, name);
  return values;
}

test("a pin grid shows Diameter, Columns and Rows with their spacings, and both toggles", () => {
  const body = roundBody(round("locked_pin_grid"));
  assert.equal(body.diameter?.key, "diameter_mm");
  assert.equal(body.diameter?.value, 6);
  assert.deepEqual(body.columns.map((f) => [f.key, f.label, f.value, f.whole]), [
    ["columns", "Columns", 2, true],
    ["column_spacing_mm", "Spacing", 30, false],
  ]);
  assert.deepEqual(body.rows.map((f) => [f.key, f.value]), [
    ["rows", 2],
    ["row_spacing_mm", 20],
  ]);
  assert.deepEqual(body.jobs.map((job) => [job.label, job.toggle.on, job.size]), [
    ["Centre mark", false, null],
    ["Diameters", true, null],
  ]);
});

test("a screw has no Diameter; Spacing stays, greyed out at one; Size shows while Centre mark is on", () => {
  const body = roundBody(round("screw_grid_centre_mark"));
  assert.equal(body.diameter, null);
  assert.deepEqual(body.columns.map((f) => [f.key, f.disabled === true]), [
    ["columns", false],
    ["column_spacing_mm", false],
  ]);
  // Customer, 2026-11-16: the Spacing is always shown and only greyed out for one row.
  assert.deepEqual(body.rows.map((f) => [f.key, f.disabled === true]), [
    ["rows", false],
    ["row_spacing_mm", true],
  ]);
  const centre = body.jobs[0];
  assert.equal(centre.label, "Centre mark");
  assert.equal(centre.size?.key, "mark_mm");
  assert.equal(centre.size?.value, 3);
  assert.ok(centre.tooltip.length > 0 && body.jobs[1].tooltip.length > 0);
});

test("an old screw shows no Jobs toggles: hidden, not disabled (criterion 39)", () => {
  const view = decodeFixturesView(JSON.parse(readFileSync(new URL("old_file_job_fixtures.json", DIR), "utf8")));
  const screw = view?.jobFixtures.find((row) => row.kind === "screw")?.round;
  assert.ok(screw);
  assert.deepEqual(roundBody(screw).jobs, []);
});

test("only a pin and a screw have the body", () => {
  assert.equal(hasRoundBody("pin"), true);
  assert.equal(hasRoundBody("screw"), true);
  assert.equal(hasRoundBody("holder"), false);
  assert.equal(hasRoundBody("unknown"), false);
});

test("typed text becomes a value: counts are rounded, an empty or non-number text is nothing", () => {
  assert.equal(fieldValue({ whole: true }, "3.6"), 4);
  assert.equal(fieldValue({ whole: false }, "6,5"), 6.5);
  assert.equal(fieldValue({ whole: false }, "  12 "), 12);
  assert.equal(fieldValue({ whole: false }, ""), null);
  assert.equal(fieldValue({ whole: false }, "abc"), null);
  assert.equal(fieldText(6), "6");
  assert.equal(fieldText(6.12345), "6.123");
});
