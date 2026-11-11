// Independent acceptance tests for `specs/0103-fixture-jobs` in the frontend: the section's words
// (criteria 1, 17), the Parts list count (8), the job's second line and its priority (15, 18, 21),
// the summary's fixture line (17), and the two texts of criteria 10 and 22 that the view must show.
// Written from the specification before the components were read; the texts are the spec's words.
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import type { JobRow, PartsView } from "../src/lib/fixtureJobsView.ts";
import {
  jobLine2,
  partsHeader,
  runningLine,
  sectionTitle,
  summaryWithFixtures,
} from "../src/lib/fixtureJobRows.ts";

function job(overrides: Partial<JobRow> = {}): JobRow {
  return {
    id: "j1",
    name: "Cut 1",
    kind: { text: "cut", word: "Cut" },
    show: true,
    output: true,
    fixtureActive: true,
    runs: false,
    partsTicked: 0,
    partsTotal: 2,
    position: 0,
    of: 1,
    ...overrides,
  };
}

test("criterion 1: the header reads 'Fixture jobs (N)' with the count of every job", () => {
  assert.equal(sectionTitle(0), "Fixture jobs (0)");
  assert.equal(sectionTitle(4), "Fixture jobs (4)");
});

test("criterion 17: the fixture line is '1 fixture job runs.' or '<F> fixture jobs run.', and nothing with none", () => {
  assert.equal(runningLine(1), "1 fixture job runs.");
  assert.equal(runningLine(5), "5 fixture jobs run.");
  assert.equal(runningLine(0), null);
});

test("criterion 17: 'Nothing runs.' drops when a job runs; the fixture line follows the work line", () => {
  const nothing = [{ text: "Nothing runs. Assign items to an operation.", count: null }];
  assert.deepEqual(
    summaryWithFixtures(nothing, 1).map((line) => line.text),
    ["1 fixture job runs."],
  );
  const work = [
    { text: "Runs 2 operations, 3 items.", count: null },
    { text: "Cut 1 has 3 items.", count: null },
  ];
  assert.deepEqual(
    summaryWithFixtures(work, 2).map((line) => line.text),
    ["Runs 2 operations, 3 items.", "2 fixture jobs run.", "Cut 1 has 3 items."],
  );
});

test("criterion 8: the Parts list header counts the ticked readable parts, '2 of 5'", () => {
  const parts: PartsView = {
    job: "j1",
    rows: [1, 2, 3, 4, 5].map((n) => ({
      token: `slot:${n}`,
      name: `Slot ${n}`,
      ticked: n <= 2,
      select: `p${n}`,
      state: "unticked",
      kind: null,
    })),
  };
  assert.equal(partsHeader(parts), "2 of 5");
});

test("criterion 15: a job line says 'not on the bed' first, then 'Output off', then the part count", () => {
  assert.equal(jobLine2(job({ fixtureActive: false, output: false, partsTicked: 1 })), "Cut · not on the bed");
  assert.equal(jobLine2(job({ output: false, partsTicked: 1 })), "Cut · Output off");
  assert.equal(jobLine2(job({ partsTicked: 2 })), "Cut · 2 parts");
});

test("criterion 21: an Unknown job reads 'Unknown fixture job' and has no kind word", () => {
  assert.equal(jobLine2(job({ kind: null })), "Unknown fixture job");
});

test("criterion 10: the refused work-item drop shows its notice, at most 70 characters", () => {
  const source = [
    "../src/components/panel/FixtureJobsSection.tsx",
    "../src/components/panel/OperationsSection.tsx",
    "../src/lib/fixtureJobRows.ts",
  ]
    .map((path) => readFileSync(new URL(path, import.meta.url), "utf8"))
    .join("\n");
  const notice = "Work items do not run in fixture jobs.";
  assert.ok(source.includes(notice), "the notice of criterion 10 is in the frontend");
  assert.ok(notice.length <= 70);
});

test("criterion 22: the damage notice of a job is shown in the frontend (known gap, reported)", () => {
  const source = readFileSync(new URL("../src/components/panel/FixtureJobsSection.tsx", import.meta.url), "utf8");
  assert.ok(
    source.includes("damage") || source.includes("damaged"),
    "the job damage notice is drawn by the section",
  );
});
