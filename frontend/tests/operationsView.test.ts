// The Operations tab decoder against the Rust fixtures of `curvyo-editor-wasm/tests/fixtures/
// operations_view/` (`specs/0101-laser-operations` `adrs.md` decision 6). The Rust contract test
// `tests/operations_view_fixtures.rs` builds the same five states and compares them with these
// files, so a drift fails on both sides.
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync, readdirSync } from "node:fs";
import { decodeOperationsView, OPERATION_KINDS } from "../src/lib/operationsView.ts";

const DIR = new URL("../../curvyo-editor-wasm/tests/fixtures/operations_view/", import.meta.url);

function fixture(name: string): unknown {
  return JSON.parse(readFileSync(new URL(`${name}.json`, DIR), "utf8"));
}

// The `settings_*` files pin the settings part of `specs/0028` (`materials` decoders, Part D).
const NAMES = readdirSync(DIR)
  .filter((file) => file.endsWith(".json") && !file.startsWith("settings_"))
  .map((file) => file.slice(0, -".json".length))
  .sort();

test("the five Rust states decode, and nothing else is in the folder", () => {
  assert.deepEqual(NAMES, [
    "cuts_last_off",
    "empty",
    "five_kinds_cuts_last",
    "placed_and_clone_rows",
    "unknown_and_clone_members",
  ]);
  for (const name of NAMES) {
    assert.ok(decodeOperationsView(fixture(name)), name);
  }
});

test("the five kinds and the unknown kind decode with their section", () => {
  const view = decodeOperationsView(fixture("five_kinds_cuts_last"));
  assert.ok(view);
  assert.equal(view.cutsLast, true);
  const kinds = view.rows.map((row) => row.kind);
  assert.deepEqual(new Set(kinds), new Set(OPERATION_KINDS));
  for (const row of view.rows) {
    if (row.kind === "cut" || row.kind === "cut_out") assert.equal(row.section, "cutting", row.name);
    else assert.equal(row.section, "marking", row.name);
  }
});

test("an unknown operation has no section, and is decoded as unknown (customer 2026-11-10: no colour)", () => {
  const view = decodeOperationsView(fixture("unknown_and_clone_members"));
  assert.ok(view);
  const plasma = view.rows.find((row) => row.name === "Plasma 1");
  assert.ok(plasma);
  assert.equal(plasma.kind, "unknown");
  assert.equal(plasma.section, null);
});

test("the summary keeps the disjoint counts and has no missing count (Frage 169 A)", () => {
  const view = decodeOperationsView(fixture("five_kinds_cuts_last"));
  assert.ok(view);
  assert.equal(view.summary.operations, 5);
  assert.equal(view.summary.notAssigned, 1);
  assert.equal(view.summary.outputOff, 1);
  assert.ok(view.summary.runsWhileHidden <= view.summary.running);
  assert.ok(!JSON.stringify(view).includes("missing"));
});

test("the node id keeps its peer and counter for the calls back to the session", () => {
  const view = decodeOperationsView(fixture("unknown_and_clone_members"));
  assert.ok(view);
  assert.deepEqual(view.rows[1].id, { peer: "1", counter: 42 });
});

test("the empty page decodes with no rows; a page without operations is null", () => {
  const view = decodeOperationsView(fixture("empty"));
  assert.ok(view);
  assert.deepEqual(view.rows, []);
  assert.equal(decodeOperationsView(null), null);
  assert.equal(decodeOperationsView("null"), null);
});

test("a wrong enum value is refused, not guessed (checked decode)", () => {
  const good = fixture("five_kinds_cuts_last") as {
    rows: Array<Record<string, unknown>>;
  };
  const wrongKind = structuredClone(good);
  wrongKind.rows[0].kind = "teleport";
  // adrs.md decision 6: a kind this build does not know is `unknown`, never a failed decode.
  assert.equal(decodeOperationsView(wrongKind)?.rows[0]?.kind, "unknown");
  const wrongTick = structuredClone(good);
  wrongTick.rows[0].tick = "half";
  assert.equal(decodeOperationsView(wrongTick), null);
  const wrongSection = structuredClone(good);
  wrongSection.rows[0].section = "middle";
  assert.equal(decodeOperationsView(wrongSection), null);
  const negativeItems = structuredClone(good);
  negativeItems.rows[0].items = -1;
  assert.equal(decodeOperationsView(negativeItems), null);
  const missingSummary = structuredClone(good) as Record<string, unknown>;
  delete missingSummary.summary;
  assert.equal(decodeOperationsView(missingSummary), null);
});

test("the count items decode as node ids, and the items of each count match its number", () => {
  const view = decodeOperationsView(fixture("placed_and_clone_rows"));
  assert.ok(view);
  assert.equal(view.items.running.length, view.summary.running);
  assert.equal(view.items.notAssigned.length, view.summary.notAssigned);
  assert.equal(view.items.runsWhileHidden.length, view.summary.runsWhileHidden);
  assert.equal(view.items.outputOff.length, view.summary.outputOff);
  assert.deepEqual(view.items.running[0], { peer: "1", counter: 29 });
});

test("clone member rows decode with their clone, member and operations", () => {
  const view = decodeOperationsView(fixture("unknown_and_clone_members"));
  assert.ok(view);
  assert.equal(view.cloneMembers.length, 2);
  const member = view.cloneMembers[0];
  assert.equal(member.cloneName, "Clone 1");
  assert.equal(member.name, "Member 1");
  assert.equal(member.operations.length, 1);
  assert.deepEqual(member.operations[0], view.rows[1].id);
});

test("the view mode decodes to operations or drawn, and nothing else", () => {
  const view = decodeOperationsView(fixture("empty"));
  assert.ok(view);
  assert.equal(view.viewMode, "operations");
  const drawn = structuredClone(fixture("empty")) as Record<string, unknown>;
  drawn.view_mode = "drawn";
  assert.equal(decodeOperationsView(drawn)?.viewMode, "drawn");
  drawn.view_mode = "outline";
  assert.equal(decodeOperationsView(drawn), null);
  const badItems = structuredClone(fixture("empty")) as Record<string, unknown>;
  delete badItems.items;
  assert.equal(decodeOperationsView(badItems), null);
});

// The settings part of `specs/0028-material-test-library`: the `settings_*` fixtures hold a row's
// `settings` and the `pick` list, and the five older fixtures carry them too.
import { decodePick, decodeSettings } from "../src/lib/operationSettingsView.ts";

function settingsRows(name: string): { name: string; settings: unknown }[] {
  return (fixture(name) as { rows: { name: string; settings: unknown }[] }).rows;
}

test("every settings state of the Rust fixtures decodes", () => {
  const states = new Set<string>();
  for (const name of ["settings_live", "settings_stored", "settings_own_none_damaged"]) {
    const raw = fixture(name) as { pick: unknown };
    assert.ok(decodePick(raw.pick), `${name} pick`);
    for (const row of settingsRows(name)) {
      const settings = decodeSettings(row.settings);
      assert.ok(settings, `${name} ${row.name}`);
      states.add(settings.state);
    }
  }
  assert.deepEqual([...states].sort(), ["damaged", "live", "no_settings", "own", "stored"]);
});

test("a live pointing row keeps its typed field, the material's number and the Differs text", () => {
  const rows = settingsRows("settings_live");
  const settings = decodeSettings(rows[0].settings);
  assert.ok(settings);
  assert.equal(settings.differs, "Differs from Birch plywood");
  assert.equal(settings.reset, true);
  assert.equal(settings.material?.name, "Birch plywood");
  assert.deepEqual(settings.sets[0].typed, ["power"]);
  assert.equal(settings.sets[0].material?.powerPct, 35);
  assert.equal(settings.sets[0].run.powerPct, 37.5);
  assert.deepEqual(settings.disabled, [{ id: "11111111-1111-4111-8111-111111111111", reason: "No Engrave set" }]);
});

test("a stored row keeps the reason; a No settings row has empty numbers", () => {
  const stored = decodeSettings(settingsRows("settings_stored")[0].settings);
  assert.equal(stored?.reason, "Material has no Score set, using stored numbers");
  const none = decodeSettings(settingsRows("settings_live")[1].settings);
  assert.equal(none?.state, "no_settings");
  assert.deepEqual(none?.sets[0].run, { powerPct: null, speedMmMin: null, passes: null });
});

test("the pick lists the materials with their kinds in order", () => {
  const pick = decodePick((fixture("settings_live") as { pick: unknown }).pick);
  assert.ok(pick);
  assert.deepEqual(
    pick.map((entry) => [entry.name, entry.kinds]),
    [
      ["Birch plywood", ["cut", "score", "engrave"]],
      ["Cardboard", ["cut"]],
    ],
  );
});

test("a settings block or a pick with a missing field or wrong type is refused", () => {
  const good = settingsRows("settings_live")[0].settings as Record<string, unknown>;
  for (const key of Object.keys(good)) {
    const broken = { ...good };
    delete broken[key];
    assert.equal(decodeSettings(broken), undefined, `missing ${key}`);
  }
  assert.equal(decodeSettings({ ...good, state: "weird" }), undefined);
  assert.equal(decodeSettings({ ...good, reset: "yes" }), undefined);
  assert.equal(decodeSettings({ ...good, needs: "plasma" }), undefined);
  assert.equal(decodeSettings(null), null);
  const set = (good.sets as Record<string, unknown>[])[0];
  assert.equal(decodeSettings({ ...good, sets: [{ ...set, typed: ["colour"] }] }), undefined);
  assert.equal(decodeSettings({ ...good, sets: [{ ...set, run: { power_pct: "x", speed_mm_min: 1, passes: 1 } }] }), undefined);
  assert.equal(decodePick([{ id: "a", name: "b", thickness_mm: "3", kinds: [] }]), null);
  assert.equal(decodePick("none"), null);
});

test("a whole view without pick, or with a damaged row settings, is refused", () => {
  const raw = fixture("five_kinds_cuts_last") as Record<string, unknown>;
  assert.ok(decodeOperationsView(raw));
  const noPick = { ...raw };
  delete noPick.pick;
  assert.equal(decodeOperationsView(noPick), null);
  const rows = (raw.rows as Record<string, unknown>[]).map((row, at) => (at === 0 ? { ...row, settings: { state: "live" } } : row));
  assert.equal(decodeOperationsView({ ...raw, rows }), null);
  const withSettings = decodeOperationsView(raw);
  assert.ok(withSettings?.rows.some((row) => row.settings !== null));
});
