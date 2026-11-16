// Independent acceptance tests for `specs/0113-fixture-sets-and-pins` in the frontend, for the
// criteria the session view does not word itself: the Machine sets heading (criterion 2), the Apply
// button and its rows (33, 34), the pin and screw body with Spacing hidden at a count of 1, no
// screw Head or Thread field and the Size only while Centre mark is on (19, 20, 25), the hidden
// toggles of an old screw (39), and the priority of a role job's second line (28). The texts are
// the spec's words.
import { test } from "node:test";
import assert from "node:assert/strict";
import type { RoundValues } from "../src/lib/fixturesView.ts";
import type { JobRow } from "../src/lib/fixtureJobsView.ts";
import { applyLabel, candidateCountText, setsCountText } from "../src/lib/fixtureSets.ts";
import { roundBody } from "../src/lib/fixtureRound.ts";
import { isFixedJob, jobLine2 } from "../src/lib/fixtureJobRows.ts";

function round(overrides: Partial<RoundValues> = {}): RoundValues {
  return {
    diameterMm: 6,
    columns: 1,
    rows: 1,
    columnSpacingMm: 20,
    rowSpacingMm: 20,
    markMm: 4,
    centreMark: { job: "c", on: false },
    diameters: { job: "d", on: false },
    ...overrides,
  };
}

function job(overrides: Partial<JobRow> = {}): JobRow {
  return {
    id: "j1",
    name: "Centre mark",
    kind: { text: "score", word: "Score" },
    show: true,
    output: true,
    fixtureActive: true,
    runs: true,
    partsTicked: 1,
    partsTotal: 2,
    position: 0,
    of: 2,
    role: "centre_mark",
    onThisPage: true,
    ...overrides,
  } as JobRow;
}

test("criterion 2: the heading reads '<n> of <m> on this page'", () => {
  const sets = [{}, {}] as never[];
  assert.equal(setsCountText({ sets, setsOnPage: 1 }), "1 of 2 on this page");
  assert.equal(setsCountText({ sets, setsOnPage: 2 }), "2 of 2 on this page");
});

test("criteria 33, 34: a candidate counts its job fixtures and the button reads 'Apply <n> fixtures'", () => {
  assert.equal(candidateCountText(4), "4 job fixtures");
  assert.equal(applyLabel(4), "Apply 4 fixtures");
});

test("criteria 19, 20: Spacing is greyed out while its count is 1 and editable above (customer, 2026-11-16)", () => {
  const single = roundBody(round());
  assert.deepEqual(single.columns.map((f) => [f.label, f.disabled === true]), [["Columns", false], ["Spacing", true]]);
  assert.deepEqual(single.rows.map((f) => [f.label, f.disabled === true]), [["Rows", false], ["Spacing", true]]);
  const grid = roundBody(round({ columns: 3, rows: 2 }));
  assert.ok([...grid.columns, ...grid.rows].every((f) => f.disabled !== true), "nothing greyed out above 1");
  assert.deepEqual(grid.columns.map((f) => f.key), ["columns", "column_spacing_mm"]);
  assert.deepEqual(grid.rows.map((f) => f.key), ["rows", "row_spacing_mm"]);
  assert.ok(grid.columns[0].whole && !grid.columns[1].whole, "a count is whole, a spacing is not");
});

test("criterion 20: a pin shows Diameter, a screw shows no Head, Thread or Diameter field", () => {
  assert.equal(roundBody(round()).diameter?.label, "Diameter");
  const screw = roundBody(round({ diameterMm: null }));
  assert.equal(screw.diameter, null);
  const labels = [...screw.columns, ...screw.rows].map((f) => f.label);
  assert.ok(!labels.some((l) => /Head|Thread/.test(l)), labels.join());
});

test("criterion 25: Centre mark and Diameters toggles; Size only while Centre mark is on", () => {
  const off = roundBody(round());
  assert.deepEqual(off.jobs.map((j) => j.label), ["Centre mark", "Diameters"]);
  assert.equal(off.jobs[0].size, null, "Size hidden while off");
  const on = roundBody(round({ centreMark: { job: "c", on: true }, markMm: 3 }));
  assert.equal(on.jobs[0].size?.label, "Size");
  assert.equal(on.jobs[0].size?.value, 3);
  assert.equal(on.jobs[1].size, null, "Diameters has no Size");
});

test("criterion 39: an old screw without role jobs shows no Jobs toggles", () => {
  const old = roundBody(round({ diameterMm: null, centreMark: null, diameters: null }));
  assert.deepEqual(old.jobs, []);
});

test("criterion 28: line 2 priority is not on this page, not on the bed, Output off, parts", () => {
  assert.equal(
    jobLine2(job({ onThisPage: false, fixtureActive: false, output: false })),
    "Score · not on this page",
  );
  assert.equal(jobLine2(job({ fixtureActive: false, output: false })), "Score · not on the bed");
  assert.equal(jobLine2(job({ output: false })), "Score · Output off");
});

test("criterion 27: a role job is fixed (no trash, no rename); another job is not", () => {
  assert.equal(isFixedJob(job()), true);
  assert.equal(isFixedJob(job({ role: null })), false);
});
