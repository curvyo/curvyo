// The Fixture jobs section's decoder against the Rust fixtures of `curvyo-editor-wasm/tests/
// fixtures/fixture_jobs_view/` (`specs/0103-fixture-jobs` `adrs.md` section 6). The Rust contract
// test `tests/fixture_jobs_view_contract.rs` builds the same states and compares them with these
// files, so a drift fails on both sides.
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync, readdirSync } from "node:fs";
import { decodeFixtureJobsText, decodeFixtureJobsView } from "../src/lib/fixtureJobsView.ts";

const DIR = new URL("../../curvyo-editor-wasm/tests/fixtures/fixture_jobs_view/", import.meta.url);

function fixture(name: string): unknown {
  return JSON.parse(readFileSync(new URL(`${name}.json`, DIR), "utf8"));
}

function clone<T>(value: T): T {
  return JSON.parse(JSON.stringify(value)) as T;
}

const NAMES = readdirSync(DIR)
  .filter((file) => file.endsWith(".json"))
  .map((file) => file.slice(0, -".json".length))
  .sort();

test("the Rust states decode, and nothing else is in the folder", () => {
  assert.deepEqual(NAMES, ["empty", "two_fixtures", "unknown_and_show_off"]);
  for (const name of NAMES) {
    assert.ok(decodeFixtureJobsView(fixture(name)), name);
  }
});

test("the empty page has no groups, no parts and no add target", () => {
  const view = decodeFixtureJobsView(fixture("empty"));
  assert.ok(view);
  assert.equal(view.count, 0);
  assert.deepEqual(view.groups, []);
  assert.equal(view.addTo, null);
  assert.equal(view.parts, null);
  assert.equal(view.selected, null);
  assert.equal(view.running, 0);
  assert.deepEqual(view.runningFixtures, []);
});

test("two fixtures decode with their groups, the running job and the open Parts list", () => {
  const view = decodeFixtureJobsView(fixture("two_fixtures"));
  assert.ok(view);
  assert.equal(view.count, 3);
  assert.deepEqual(
    view.groups.map((group) => [group.name, group.kind]),
    [
      ["Grid 1", "grid"],
      ["Holder 1", "holder"],
      ["Holder 2", "holder"],
    ],
  );
  const engrave = view.groups[1].jobs[0];
  assert.equal(engrave.name, "Engrave + Outline 1");
  assert.deepEqual(engrave.kind, { text: "engrave_outline", word: "Engrave + Outline" });
  assert.equal(engrave.output, true);
  assert.equal(engrave.runs, true);
  assert.equal(engrave.partsTicked, 2);
  assert.equal(engrave.partsTotal, 2);
  assert.equal(view.groups[2].jobs[0].fixtureActive, false, "a fixture off the bed");
  assert.equal(view.running, 1);
  assert.deepEqual(view.runningFixtures, [view.groups[1].fixture]);
  assert.equal(view.selected, engrave.id);
  assert.ok(view.parts);
  assert.equal(view.parts.job, engrave.id);
  assert.deepEqual(
    view.parts.rows.map((row) => [row.token, row.name, row.ticked, row.state, row.kind]),
    [
      ["board", "Board", true, "running", "engrave_outline"],
      ["slot:0", "Coaster 1", true, "running", "engrave_outline"],
    ],
  );
});

test("an unknown job decodes with no kind, and a Show-off job with its state", () => {
  const view = decodeFixtureJobsView(fixture("unknown_and_show_off"));
  assert.ok(view);
  const [burn, cut] = view.groups[0].jobs;
  assert.equal(burn.kind, null, "a kind this build does not know is no kind");
  assert.equal(burn.name, "Burn 1");
  assert.equal(cut.show, false);
  assert.equal(cut.position, 1);
  assert.equal(cut.of, 2);
  assert.equal(view.parts?.rows[1].state, "show_off");
  assert.equal(view.parts?.rows[1].kind, null);
});

test("an unknown kind text in a job is no kind, and an unknown fixture kind is 'unknown'", () => {
  const good = clone(fixture("two_fixtures")) as Record<string, unknown>;
  const groups = good.groups as Array<Record<string, unknown>>;
  (groups[0].jobs as Array<Record<string, unknown>>)[0].kind = { text: "laser_wave", word: "Wave" };
  groups[1].kind = "turntable";
  const view = decodeFixtureJobsView(good);
  assert.ok(view);
  assert.equal(view.groups[0].jobs[0].kind, null);
  assert.equal(view.groups[1].kind, "unknown");
});

test("hostile values decode to null, never to a guess", () => {
  const good = fixture("two_fixtures") as Record<string, unknown>;
  const cases: unknown[] = [
    null,
    "text",
    42,
    [],
    { ...good, groups: "no" },
    { ...good, count: -1 },
    { ...good, count: 1.5 },
    { ...good, running: "1" },
    { ...good, selected: 7 },
    { ...good, running_fixtures: [1] },
    { ...good, add_to: { fixture: "x" } },
    { ...good, parts: { job: "x", rows: [{ token: "board" }] } },
    (() => {
      const bad = clone(good) as Record<string, unknown>;
      (bad.groups as Array<Record<string, unknown>>)[0].jobs = [{ id: "a" }];
      return bad;
    })(),
    (() => {
      const bad = clone(good) as Record<string, unknown>;
      ((bad.parts as Record<string, unknown>).rows as Array<Record<string, unknown>>)[0].state = "dimmed";
      return bad;
    })(),
    (() => {
      const bad = clone(good) as Record<string, unknown>;
      ((bad.groups as Array<Record<string, unknown>>)[0].jobs as Array<Record<string, unknown>>)[0].show = "yes";
      return bad;
    })(),
  ];
  for (const value of cases) {
    assert.equal(decodeFixtureJobsView(value), null, JSON.stringify(value));
  }
});

test("the damage count and its notices decode; the empty page has none", () => {
  assert.equal(decodeFixtureJobsView(fixture("empty"))?.damaged, 0);
  assert.deepEqual(decodeFixtureJobsView(fixture("empty"))?.damageNotices, []);
  const good = clone(fixture("two_fixtures")) as Record<string, unknown>;
  good.damaged = 4;
  good.damage_notices = ["4 values damaged on holder 1 job."];
  const view = decodeFixtureJobsView(good);
  assert.ok(view);
  assert.equal(view.damaged, 4);
  assert.deepEqual(view.damageNotices, ["4 values damaged on holder 1 job."]);
  assert.equal(decodeFixtureJobsView({ ...good, damaged: -1 }), null, "a negative count is refused");
  assert.equal(decodeFixtureJobsView({ ...good, damage_notices: "text" }), null, "a text is not a list");
  assert.equal(decodeFixtureJobsView({ ...good, damage_notices: [4] }), null, "a number notice is refused");
});

test("the text form decodes, and bad JSON or the text null is no view", () => {
  assert.equal(decodeFixtureJobsText("null"), null);
  assert.equal(decodeFixtureJobsText("{not json"), null);
  assert.equal(decodeFixtureJobsText(JSON.stringify(fixture("empty")))?.count, 0);
});
