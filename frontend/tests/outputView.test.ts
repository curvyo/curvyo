// The output group's, the play bar's and the ring labels' decoder against the Rust fixtures of
// `curvyo-editor-wasm/tests/fixtures/output_view/` (`specs/0029-laser-job-preview-and-output`). The
// Rust contract test `tests/output_view_contract.rs` builds the same states and compares them with
// these files, so a drift fails on both sides.
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync, readdirSync } from "node:fs";
import {
  decodeOutputGroup,
  decodeOutputGroupText,
  decodePlayView,
  decodeRingLabels,
  decodeRingLabelsText,
} from "../src/lib/outputView.ts";
import { breakEveryField } from "./contractWalk.ts";

const DIR = new URL("../../curvyo-editor-wasm/tests/fixtures/output_view/", import.meta.url);

interface Fixture {
  output: unknown;
  play: unknown;
  ring_labels: unknown;
  rows: unknown;
}

function fixture(name: string): Fixture {
  return JSON.parse(readFileSync(new URL(`${name}.json`, DIR), "utf8")) as Fixture;
}

const NAMES = readdirSync(DIR)
  .filter((file) => file.endsWith(".json"))
  .map((file) => file.slice(0, -".json".length))
  .sort();

test("the Rust states decode, and nothing else is in the folder", () => {
  assert.deepEqual(NAMES, [
    "blocked",
    "cut_and_engrave",
    "empty",
    "fixture_no_material",
    "raster_checks",
    "start_damaged",
    "start_head_position",
    "start_home_then_origin",
    "toolpath_half",
  ]);
  for (const name of NAMES) {
    const { output, ring_labels: rings } = fixture(name);
    assert.ok(decodeOutputGroup(output), name);
    assert.ok(decodeRingLabels(rings), name);
  }
});

test("nothing runs: the block, no total, Export disabled with its reason", () => {
  const view = decodeOutputGroup(fixture("empty").output);
  assert.ok(view);
  assert.equal(view.total, null);
  assert.equal(view.export.enabled, false);
  assert.equal(view.export.tooltip, "Nothing runs. Assign items to an operation.");
  assert.equal(view.checks[0].severity, "block");
});

test("a check splits into the text before, the button and the text after", () => {
  const view = decodeOutputGroup(fixture("cut_and_engrave").output);
  assert.ok(view);
  // The Output off line names its operation and selects its items (`0101` criterion 17).
  const off = view.checks.find((check) => check.after === " has Output off.");
  assert.ok(off);
  assert.equal(off.target.kind, "items");
  assert.equal(off.before + off.button + off.after, off.text);
  assert.equal(off.button, "Score 1");
  assert.equal(view.export.fileType, "GRBL G-code");
  assert.equal(view.machine, "Generic laser · GRBL");
  assert.equal(view.total?.runs?.text, "Runs 2 operations, 2 items.");
  assert.deepEqual(
    [view.total?.runs?.before, view.total?.runs?.button, view.total?.runs?.after],
    ["Runs 2 operations, ", "2 items", "."],
  );
});

test("the Start pick has three plain options and the stored value", () => {
  const origin = decodeOutputGroup(fixture("cut_and_engrave").output);
  assert.ok(origin);
  assert.deepEqual(origin.start.options.map((option) => option.label), [
    "Machine origin",
    "Head position",
    "Home, then machine origin",
  ]);
  assert.equal(origin.start.value, "machine_origin");
  assert.equal(origin.start.damaged, false);
  assert.equal(decodeOutputGroup(fixture("start_head_position").output)?.start.value, "head_position");
  assert.equal(decodeOutputGroup(fixture("start_home_then_origin").output)?.start.value, "home_then_origin");
  const damaged = decodeOutputGroup(fixture("start_damaged").output);
  assert.equal(damaged?.start.damaged, true);
  assert.equal(damaged?.start.value, null);
});

test("a check can open the Start pick or the fixture pick", () => {
  const damaged = decodeOutputGroup(fixture("start_damaged").output);
  assert.deepEqual(damaged?.checks[0].target, { kind: "start_pick" });
  const fixturePick = decodeOutputGroup(fixture("fixture_no_material").output);
  assert.deepEqual(fixturePick?.checks[0].target, { kind: "fixture_pick" });
  assert.equal(fixturePick?.checks[0].button, "Fixture jobs");
  const raw = structuredClone(fixture("blocked").output) as { checks: Array<{ target: unknown }> };
  raw.checks[0].target = { kind: "somewhere" };
  assert.equal(decodeOutputGroup(raw), null);
});

test("the rows of every state decode in the Operations view with ring, time and numbers", () => {
  const rows = fixture("cut_and_engrave").rows as Array<Record<string, unknown>>;
  assert.deepEqual(
    rows.map((row) => [row.name, (row.run as { digit: number } | null)?.digit ?? null]),
    [
      ["Engrave 1", 1],
      ["Score 1", null],
      ["Cut 1", 3],
    ],
  );
});

test("the ring labels carry a digit or null for the fixture glyph", () => {
  const rings = decodeRingLabels(fixture("toolpath_half").ring_labels);
  assert.ok(rings);
  assert.equal(rings.length, 1);
  assert.equal(rings[0].digit, 1);
  assert.deepEqual(decodeRingLabels([{ digit: null, x: 1, y: 2 }]), [{ digit: null, x: 1, y: 2 }]);
  assert.equal(decodeRingLabels([{ digit: "1", x: 1, y: 2 }]), null);
  assert.deepEqual(decodeRingLabelsText("{"), []);
});

test("the play bar decodes only while the Toolpath view is pressed, and its readout is time only", () => {
  assert.equal(decodePlayView(fixture("empty").play), null);
  const play = decodePlayView(fixture("toolpath_half").play);
  assert.ok(play);
  assert.equal(play.fraction, 0.5);
  assert.equal(play.playing, false);
  assert.match(play.readout, /^[^·]* of [^·]*$/);
});

test("another page and broken text decode to null", () => {
  assert.equal(decodeOutputGroupText("null"), null);
  assert.equal(decodeOutputGroupText("{"), null);
});

test("every missing or mistyped field is refused", () => {
  for (const name of NAMES) {
    const { output, play, ring_labels: rings } = fixture(name);
    for (const broken of breakEveryField(output)) {
      // ADR 0098: a family page has no Start pick and no Frame button, so their keys may be absent.
      if (broken.how === "missing" && (broken.path === "start" || broken.path === "frame")) continue;
      assert.equal(decodeOutputGroup(broken.value), null, `${name}: ${broken.how} ${broken.path} must be refused`);
    }
    for (const broken of breakEveryField(rings)) {
      assert.equal(decodeRingLabels(broken.value), null, `${name}: ${broken.how} ring_labels.${broken.path} must be refused`);
    }
    if (play !== null) {
      for (const broken of breakEveryField(play)) {
        assert.equal(decodePlayView(broken.value), null, `${name}: ${broken.how} play.${broken.path} must be refused`);
      }
    }
  }
});

test("the total's fixture line decodes with its button; a work line may be absent", () => {
  const raw = fixture("cut_and_engrave").output as Record<string, unknown>;
  const total = raw.total as Record<string, unknown>;
  const fixtures = { text: "1 fixture job runs.", button: "1 fixture job", after: " runs.", ids: [{ peer: "1", counter: 7 }] };
  const view = decodeOutputGroup({ ...raw, total: { ...total, runs: null, fixtures } });
  assert.ok(view?.total);
  assert.equal(view.total.runs, null);
  assert.equal(view.total.fixtures?.button + (view.total.fixtures?.after ?? ""), "1 fixture job runs.");
  assert.equal(decodeOutputGroup({ ...raw, total: { ...total, fixtures: { ...fixtures, ids: "1:7" } } }), null);
});

test("the raster checks keep their order and the Lowest power line focuses its field (specs/0123 AC 32 b, 35, 36)", () => {
  const { output } = fixture("raster_checks");
  const view = decodeOutputGroup(output);
  assert.ok(view);
  const texts = view.checks.map((c) => c.text);
  const at = (text: string) => texts.indexOf(text);
  assert.ok(at("1 image is coarser than its Interval.") >= 0, texts.join(" | "));
  assert.ok(at("1 image is coarser than its Interval.") < at("1 image cannot be read and is skipped."));
  assert.ok(at("1 image cannot be read and is skipped.") < at("Image 1: Lowest power is above Power."));
  const lowest = view.checks[at("Image 1: Lowest power is above Power.")];
  assert.ok(lowest && lowest.target.kind === "operations");
  assert.equal(lowest.target.focus, "lowest_power");
});
