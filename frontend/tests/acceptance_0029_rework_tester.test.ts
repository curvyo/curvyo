// Independent acceptance tests for the rework of `specs/0029-laser-job-preview-and-output` after the
// customer's try of PR #371 (2026-11-17): the output group's checks (before, button, after), the
// Export state, the Start pick (a damaged one included), the rows' lines 2 and 3 (a row that does
// not run keeps its plain digit), the fixture ring label and the time-only play readout. Written
// from the amended specification; the views are the session's shared JSON fixtures.
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

import { decodeOutputGroup, decodePlayView, decodeRingLabels } from "../src/lib/outputView.ts";
import { decodeOperationsView } from "../src/lib/operationsView.ts";
import { rowLine2, rowName, rowRunLine2 } from "../src/lib/operationRows.ts";
import { numbersParts, readoutParts } from "../src/lib/outputText.ts";

const OUTPUT = new URL("../../curvyo-editor-wasm/tests/fixtures/output_view/", import.meta.url);
const OPS = new URL("../../curvyo-editor-wasm/tests/fixtures/operations_view/", import.meta.url);
const json = (dir: URL, name: string): Record<string, unknown> =>
  JSON.parse(readFileSync(new URL(`${name}.json`, dir), "utf8")) as Record<string, unknown>;

test("criteria 4 and 5: every check of the shared fixtures is before + button + after, a button exactly with a target", () => {
  for (const name of [
    "blocked",
    "cut_and_engrave",
    "empty",
    "fixture_no_material",
    "start_damaged",
    "start_head_position",
    "start_home_then_origin",
    "toolpath_half",
  ]) {
    const group = decodeOutputGroup(json(OUTPUT, name).output);
    assert.ok(group, `${name} decodes`);
    for (const check of group.checks) {
      assert.equal(check.text, `${check.before}${check.button}${check.after}`, name);
      assert.equal(check.button === "", check.target.kind === "none", `${name}: ${check.text}`);
    }
    const kinds = group.checks.map((c) => c.severity);
    assert.deepEqual(kinds, [...kinds].sort(), `${name}: blocks first, then warns`);
  }
});

test("criterion 4: a blocked group has Export disabled with the first block as its tooltip", () => {
  const group = decodeOutputGroup(json(OUTPUT, "blocked").output);
  assert.ok(group);
  assert.equal(group.export.enabled, false);
  const first = group.checks.find((c) => c.severity === "block");
  assert.equal(group.export.tooltip, first?.text);
  assert.equal(group.export.fileType, "GRBL G-code");
  assert.equal(group.export.extension, "gcode");
  const op = group.checks.find((c) => c.target.kind === "operations");
  assert.equal(op?.text, "Cut 1 has no power and speed.");
  assert.equal(op?.button, "Cut 1");
});

test("criterion 4: an unblocked group has Export enabled and no tooltip", () => {
  const group = decodeOutputGroup(json(OUTPUT, "start_head_position").output);
  assert.ok(group);
  assert.equal(group.export.enabled, true);
  assert.equal(group.export.tooltip, null);
});

test("criterion 7: with nothing to run the total is absent and Export is disabled", () => {
  const group = decodeOutputGroup(json(OUTPUT, "empty").output);
  assert.ok(group);
  assert.equal(group.total, null);
  assert.equal(group.export.enabled, false);
  assert.ok(group.checks.some((c) => c.text === "Nothing runs. Assign items to an operation." && c.severity === "block"));
});

test("criterion 2: the machine line is '<profile> · <format>' and nothing of the bed or the laser mode", () => {
  const group = decodeOutputGroup(json(OUTPUT, "blocked").output);
  assert.ok(group);
  assert.equal(group.machine, "Generic laser · GRBL");
});

test("criterion 38: the Start pick has three values in order and reads the stored one", () => {
  for (const [name, value] of [
    ["start_head_position", "head_position"],
    ["start_home_then_origin", "home_then_origin"],
    ["blocked", "machine_origin"],
  ] as const) {
    const group = decodeOutputGroup(json(OUTPUT, name).output);
    assert.ok(group);
    assert.equal(group.start.value, value);
    assert.deepEqual(
      group.start.options.map((o) => o.label),
      ["Machine origin", "Head position", "Home, then machine origin"],
    );
  }
});

test("criterion 4 e: a damaged Start (value null) still decodes, so its check and its button show", () => {
  // The session's shared fixture of a damaged Start (`output_view_contract.rs` `start_damaged`).
  const group = decodeOutputGroup(json(OUTPUT, "start_damaged").output);
  assert.ok(group, "the output group of a page with a damaged Start must not vanish");
  assert.equal(group.start.damaged, true);
  assert.equal(group.start.value, null);
  assert.equal(group.checks[0].text, "Start is damaged. Pick one above.");
  assert.equal(group.checks[0].target.kind, "start_pick");
  assert.equal(group.export.enabled, false);
});

test("criterion 2: a running row's name is the session's run name; a row that does not run keeps its plain digit", () => {
  const view = decodeOperationsView(json(OPS, "five_kinds_cuts_last"));
  assert.ok(view);
  const running = view.rows.filter((r) => r.run !== null);
  const idle = view.rows.filter((r) => r.run === null);
  assert.ok(running.length > 0, "a running row in the fixture");
  for (const row of running) {
    assert.ok(rowName(row).startsWith(`${row.name}, run ${row.run?.digit}, `), rowName(row));
    assert.doesNotMatch(rowRunLine2(row).text, /^\d/, "the digit is the ring, not the text");
  }
  for (const row of view.rows) {
    const index = view.rows.indexOf(row);
    if (row.run === null) assert.match(rowLine2(row, index + 1).text, new RegExp(`^${index + 1} · `));
    else assert.equal(row.run.digit, index + 1, `${row.name}: the ring is the row's digit`);
  }
  assert.ok(idle.every((r) => !rowName(r).includes(", run ")), "no run name for a row that does not run");
});

test("criterion 2: line 3 marks each typed number with 'typed' after it", () => {
  const parts = numbersParts("65 %, 300 mm/min, 2 ×, 0.1 mm", ["power", "passes"]);
  assert.deepEqual(
    parts.map((p) => p.typed),
    [true, false, true, false],
  );
});

test("criterion 37: the readout is time only ('<elapsed> of <total>')", () => {
  const play = decodePlayView(json(OUTPUT, "toolpath_half").play);
  assert.ok(play);
  assert.doesNotMatch(play.readout, /\bmm\b|\bm\b|·/);
  assert.match(play.readout, /^[\d hmins]+ of [\d hmins]+$/);
  assert.deepEqual(
    readoutParts("6 min 6 s of 9 min 51 s").map((p) => p.muted),
    [false, true, false],
  );
});

test("criterion 2: a fixture ring label carries no digit (the fixture glyph), an operation ring its digit", () => {
  const rings = decodeRingLabels([
    { digit: null, x: 10, y: 20 },
    { digit: 3, x: 30, y: 40 },
  ]);
  assert.deepEqual(rings?.map((r) => r.digit), [null, 3]);
});
