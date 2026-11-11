// The Fixture jobs section's row logic (`specs/0103-fixture-jobs` `ux-notes.md` sections 3 to 5,
// 7, 13): keys, the words of lines and names, the Parts list's rules and the summary line. Pure,
// so the components only draw what these return.
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { decodeFixtureJobsText, type FixtureJobsView, type JobRow } from "../src/lib/fixtureJobsView.ts";
import {
  ADD_KINDS,
  addLabel,
  armKey,
  flatJobs,
  jobAccessibleName,
  jobLine2,
  jobNeighbour,
  landingPosition,
  partsFor,
  partsHeader,
  runningLine,
  sectionTitle,
  summaryWithFixtures,
  tickTitle,
} from "../src/lib/fixtureJobRows.ts";

const DIR = new URL("../../curvyo-editor-wasm/tests/fixtures/fixture_jobs_view/", import.meta.url);

function view(name: string): FixtureJobsView {
  const parsed = decodeFixtureJobsText(readFileSync(new URL(`${name}.json`, DIR), "utf8"));
  assert.ok(parsed, name);
  return parsed;
}

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

test("the add row has the five kinds in the order of the notes, with the fixture named", () => {
  assert.deepEqual(
    ADD_KINDS.map(([kind]) => kind),
    ["cut", "cut_out", "score", "engrave", "engrave_outline"],
  );
  assert.equal(addLabel("cut", "Holder 1"), "Add Cut job to Holder 1");
  assert.equal(addLabel("cut_out", "Board tray"), "Add Cut-out job to Board tray");
  assert.equal(addLabel("engrave_outline", "Holder 1"), "Add Engrave + Outline job to Holder 1");
});

test("the header counts every job, and the summary line counts the running ones", () => {
  assert.equal(sectionTitle(0), "Fixture jobs (0)");
  assert.equal(sectionTitle(3), "Fixture jobs (3)");
  assert.equal(runningLine(0), null);
  assert.equal(runningLine(1), "1 fixture job runs.");
  assert.equal(runningLine(2), "2 fixture jobs run.");
});

test("line 2 says not on the bed, then Output off, then the part count", () => {
  assert.equal(jobLine2(job({ partsTicked: 1 })), "Cut · 1 part");
  assert.equal(jobLine2(job({ partsTicked: 2 })), "Cut · 2 parts");
  assert.equal(jobLine2(job()), "Cut · no part ticked");
  assert.equal(jobLine2(job({ output: false, partsTicked: 2 })), "Cut · Output off");
  assert.equal(jobLine2(job({ fixtureActive: false, output: false })), "Cut · not on the bed");
  assert.equal(
    jobLine2(job({ kind: { text: "engrave_outline", word: "Engrave + Outline" }, partsTicked: 3 })),
    "Engrave + Outline · 3 parts",
  );
  assert.equal(jobLine2(job({ kind: null })), "Unknown fixture job");
});

test("the accessible name carries the kind, the count and the state words", () => {
  assert.equal(jobAccessibleName(job({ output: false })), "Cut 1, Cut, 0 parts, Output off");
  assert.equal(jobAccessibleName(job({ fixtureActive: false, partsTicked: 1 })), "Cut 1, Cut, 1 part, not on the bed");
  assert.equal(jobAccessibleName(job({ kind: null, name: "Burn 1" })), "Burn 1, unknown fixture job");
});

test("an unknown job has no Parts list, a known one has it only when selected", () => {
  const selected = view("two_fixtures");
  const row = selected.groups[1].jobs[0];
  assert.ok(partsFor(selected, row), "the selected job shows its list");
  assert.equal(partsFor(selected, selected.groups[0].jobs[0]), null, "another job shows none");
  const unknown = view("unknown_and_show_off");
  const burn = unknown.groups[0].jobs[0];
  assert.equal(partsFor({ ...unknown, selected: burn.id, parts: unknown.parts && { ...unknown.parts, job: burn.id } }, burn), null);
  assert.equal(partsFor(view("empty"), job()), null);
});

test("the parts header counts the ticked readable parts", () => {
  const parts = view("two_fixtures").parts;
  assert.ok(parts);
  assert.equal(partsHeader(parts), "2 of 2");
  assert.equal(partsHeader({ job: "j", rows: [] }), "0 of 0");
});

test("a tick adds an unticked part and removes a ticked one, with the job's name", () => {
  assert.equal(tickTitle(false, "Cut 1"), "Assign to Cut 1");
  assert.equal(tickTitle(true, "Cut 1"), "Remove from Cut 1");
});

test("every job has one flat place and one arm key, unique over the section", () => {
  const groups = view("unknown_and_show_off").groups;
  const jobs = flatJobs(groups);
  assert.equal(jobs.length, 2);
  const keys = jobs.map((row) => armKey(row.id));
  assert.equal(new Set(keys).size, keys.length);
  assert.equal(armKey("j1"), "fjob:j1");
});

test("the arrows and the drag stay inside the job's fixture group", () => {
  const groups = view("unknown_and_show_off").groups;
  const first = groups[0].jobs[0].id;
  const second = groups[0].jobs[1].id;
  assert.equal(jobNeighbour(groups, first, -1), null, "the first place has no up");
  assert.equal(jobNeighbour(groups, first, 1), 1);
  assert.equal(jobNeighbour(groups, second, 1), null, "the last place has no down");
  assert.equal(jobNeighbour(groups, "missing", 1), null);
});

test("the drop lands inside the group: down past a midpoint, up above one, else where it was", () => {
  const middles = [10, 50, 90];
  assert.equal(landingPosition(0, middles, 5), 0, "not past the next midpoint");
  assert.equal(landingPosition(0, middles, 55), 1);
  assert.equal(landingPosition(0, middles, 95), 2);
  assert.equal(landingPosition(2, middles, 45), 1, "up past the midpoint above");
  assert.equal(landingPosition(2, middles, 5), 0);
});

test("the summary puts the fixture line after the work line and drops 'Nothing runs.' when jobs run", () => {
  const work = [
    { text: "Runs 1 operation, 4 items.", count: null },
    { text: "2 items not assigned.", count: "notAssigned" as const },
  ];
  const texts = (lines: ReadonlyArray<{ text: string }>) => lines.map((line) => line.text);
  assert.deepEqual(texts(summaryWithFixtures(work, 1)), [
    "Runs 1 operation, 4 items.",
    "1 fixture job runs.",
    "2 items not assigned.",
  ]);
  assert.deepEqual(texts(summaryWithFixtures(work, 0)), ["Runs 1 operation, 4 items.", "2 items not assigned."]);
  const nothing = [{ text: "Nothing runs. Assign items to an operation.", count: null }];
  assert.deepEqual(texts(summaryWithFixtures(nothing, 2)), ["2 fixture jobs run."]);
  assert.deepEqual(texts(summaryWithFixtures(nothing, 0)), ["Nothing runs. Assign items to an operation."]);
});

const texts = (lines: ReadonlyArray<{ text: string }>) => lines.map((line) => line.text);

test("criterion 17 (J7): the work line is dropped when no operation runs; 'Nothing runs.' needs neither", () => {
  const idle = [
    { text: "Runs 0 operations, 0 items.", count: null },
    { text: "2 items not assigned.", count: "notAssigned" as const },
  ];
  assert.deepEqual(texts(summaryWithFixtures(idle, 0)), ["Nothing runs. Assign items to an operation.", "2 items not assigned."]);
  assert.deepEqual(texts(summaryWithFixtures(idle, 1)), ["1 fixture job runs.", "2 items not assigned."]);
  const running = [{ text: "Runs 3 operations, 5 items.", count: null }];
  assert.deepEqual(texts(summaryWithFixtures(running, 0)), ["Runs 3 operations, 5 items."]);
});
