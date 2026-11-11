// The Fixture jobs section's notices and its window (`specs/0103-fixture-jobs` criteria 10, 22;
// `ux-notes.md` 11, 14). The drawn texts are read from the section's source, since the frontend
// tests run without a DOM; the window arithmetic is tested on its pure functions.
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { decodeFixtureJobsView } from "../src/lib/fixtureJobsView.ts";
import { OVERSCAN, PARTS_FOOT_PX, PARTS_HEAD_PX, PART_ROW_PX, ROW_PX, HEADING_PX, WORK_REFUSED, entriesOf, WINDOW_ABOVE } from "../src/lib/fixtureJobRows.ts";
import { visibleRange } from "../src/lib/operationWindow.ts";

const SECTION = new URL("../src/components/panel/FixtureJobsSection.tsx", import.meta.url);

test("the damage notice of a job is shown: the decoded notice is drawn by the section", () => {
  const view = decodeFixtureJobsView({
    count: 1,
    add_to: null,
    groups: [],
    selected: null,
    parts: null,
    running: 0,
    running_fixtures: [],
    damaged: 4,
    damage_notices: ["4 values damaged on holder 1 job."],
  });
  assert.ok(view);
  assert.deepEqual(view.damageNotices, ["4 values damaged on holder 1 job."]);
  const source = readFileSync(SECTION, "utf8");
  assert.ok(source.includes("view.damageNotices.map"), "the section draws every damage notice");
});

test("the refused work-item drop shows its notice, at most 70 characters, and the section draws it", () => {
  assert.equal(WORK_REFUSED, "Work items do not run in fixture jobs.");
  assert.ok(WORK_REFUSED.length <= 70);
  const source = readFileSync(SECTION, "utf8");
  assert.ok(source.includes("api.refusal"), "the section draws the refusal");
});

test("an entry is a heading of 28 px (9 more after the first group) and a row of 40 px, plus its open Parts list", () => {
  const view = decodeFixtureJobsView({
    count: 2,
    add_to: null,
    groups: [
      { fixture: "a", name: "Holder 1", kind: "holder", jobs: [job("j1", 0, 1)] },
      { fixture: "b", name: "Grid 1", kind: "grid", jobs: [job("j2", 0, 1)] },
    ],
    selected: "j1",
    parts: {
      job: "j1",
      rows: [
        { token: "board", name: "Board", ticked: true, select: "a", state: "running", kind: "cut" },
        { token: "slot:0", name: "Slot 0", ticked: false, select: "s", state: "unticked", kind: null },
      ],
    },
    running: 0,
    running_fixtures: [],
    damaged: 0,
    damage_notices: [],
  });
  assert.ok(view);
  const { entries, rowAt } = entriesOf(view.groups, view.parts, view.selected);
  assert.deepEqual(
    entries.map((entry) => entry.height),
    [HEADING_PX, ROW_PX + PARTS_HEAD_PX + 2 * PART_ROW_PX + PARTS_FOOT_PX, HEADING_PX + 9, ROW_PX],
  );
  assert.equal(rowAt.get("j1"), 1);
  assert.equal(rowAt.get("j2"), 3);
});

test("above 60 jobs the window holds the viewport and the focused row, and nothing else", () => {
  const jobs = Array.from({ length: 100 }, (_, at) => job(`j${at}`, at, 100));
  const view = decodeFixtureJobsView({
    count: 100,
    add_to: null,
    groups: [{ fixture: "a", name: "Holder 1", kind: "holder", jobs }],
    selected: null,
    parts: null,
    running: 0,
    running_fixtures: [],
    damaged: 0,
    damage_notices: [],
  });
  assert.ok(view);
  assert.ok(view.count > WINDOW_ABOVE);
  const { entries } = entriesOf(view.groups, null, null);
  const heights = entries.map((entry) => entry.height);
  const range = visibleRange(heights, 0, 300, OVERSCAN, null, null);
  assert.ok(range.last - range.first < 30, `only the rows at the top are drawn: ${range.first}..${range.last}`);
  const total = heights.reduce((sum, h) => sum + h, 0);
  const drawn = heights.slice(range.first, range.last).reduce((sum, h) => sum + h, 0);
  assert.equal(range.before + drawn + range.after, total, "the spacers carry the rest of the height");
});

function job(id: string, position: number, of: number) {
  return {
    id,
    name: `Engrave ${position + 1}`,
    kind: { text: "engrave", word: "Engrave" },
    show: true,
    output: false,
    fixture_active: true,
    runs: false,
    parts_ticked: 0,
    parts_total: 2,
    position,
    of,
  };
}
