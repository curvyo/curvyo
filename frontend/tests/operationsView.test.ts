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
    "group_tick_mixed",
    "options_cut_z_hidden",
    "options_damaged",
    "options_engrave",
    "placed_and_clone_rows",
    "select_bar_kinds",
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

// `specs/0111-laser-operation-tools` Part C1: the new fields, each checked (AC 1, 10, 17 to 21).
type Raw = Record<string, unknown> & { rows: Array<Record<string, unknown>> };

test("the kind buttons decode in bar order with their descriptions (AC 1, 2)", () => {
  const view = decodeOperationsView(fixture("select_bar_kinds"));
  assert.ok(view?.selectBarKinds);
  assert.deepEqual(
    view.selectBarKinds.map((button) => button.kind),
    ["cut", "score", "engrave", "engrave_outline", "cut_out"],
  );
  assert.equal(view.selectBarKinds[2].description, "Assigns to Engrave 1");
  assert.equal(view.selectBarKinds[0].description, "Creates Cut 1");
  assert.equal(decodeOperationsView(fixture("empty"))?.selectBarKinds, null);
});

test("the Operations view toggle and the entered group decode (AC 11, 15)", () => {
  const view = decodeOperationsView(fixture("empty"));
  assert.ok(view);
  assert.equal(view.operationsView, true);
  assert.equal(view.enteredGroup, null);
  const entered = structuredClone(fixture("empty")) as Raw;
  entered.operations_view = false;
  entered.entered_group = { page: "Coasters", groups: ["Group 3"] };
  const decoded = decodeOperationsView(entered);
  assert.equal(decoded?.operationsView, false);
  assert.deepEqual(decoded?.enteredGroup, { page: "Coasters", groups: ["Group 3"] });
});

test("the mixed group row keeps its words and the unfolded row is marked (AC 10, 17)", () => {
  const mixed = decodeOperationsView(fixture("group_tick_mixed"));
  assert.ok(mixed);
  assert.equal(mixed.rows[0].tick, "some");
  assert.equal(mixed.rows[0].groupInPart, "Group vyl1en");
  assert.equal(mixed.rows[1].groupInPart, null);
  const engrave = decodeOperationsView(fixture("options_engrave"));
  assert.equal(engrave?.rows[0].unfolded, true);
  assert.equal(mixed.rows.some((row) => row.unfolded), false);
});

test("Interval, the hatch options and the Z offset capability decode (AC 19 to 21)", () => {
  const engrave = decodeOperationsView(fixture("options_engrave"))?.rows[0].settings;
  assert.ok(engrave);
  assert.equal(engrave.intervalMm, 0.25);
  assert.equal(engrave.zOffsetShown, true);
  assert.deepEqual(engrave.options, {
    hatch: { crosshatch: true, bidirectional: false, scanAngleDeg: 15, overscanPct: 5 },
    zOffsetMm: 1.5,
  });
  const cut = decodeOperationsView(fixture("options_cut_z_hidden"))?.rows[0].settings;
  assert.equal(cut?.options?.hatch, null);
  assert.equal(cut?.options?.zOffsetMm, 1.5);
  assert.equal(cut?.zOffsetShown, false);
  assert.equal(cut?.intervalMm, null);
  assert.equal(decodeOperationsView(fixture("options_damaged"))?.rows[0].settings?.options, null);
});

test("a view with a new field missing or of the wrong type is refused", () => {
  const good = fixture("options_engrave") as Raw;
  for (const key of ["operations_view", "select_bar_kinds", "entered_group"]) {
    const broken = structuredClone(good);
    delete broken[key];
    assert.equal(decodeOperationsView(broken), null, `missing ${key}`);
  }
  for (const key of ["group_in_part", "unfolded"]) {
    const broken = structuredClone(good);
    delete broken.rows[0][key];
    assert.equal(decodeOperationsView(broken), null, `missing row ${key}`);
  }
  const settings = (broken: Raw) => broken.rows[0].settings as Record<string, unknown>;
  for (const key of ["interval_mm", "options", "z_offset_shown"]) {
    const broken = structuredClone(good);
    delete settings(broken)[key];
    assert.equal(decodeOperationsView(broken), null, `missing settings ${key}`);
  }
  const wrong: Array<[string, (view: Raw) => void]> = [
    ["operations_view", (v) => void (v.operations_view = "yes")],
    ["kind button", (v) => void (v.select_bar_kinds = [{ kind: "teleport", description: "x" }])],
    ["kind list", (v) => void (v.select_bar_kinds = "all")],
    ["entered group", (v) => void (v.entered_group = { page: "P", groups: [] })],
    ["entered page", (v) => void (v.entered_group = { page: 3, groups: ["G"] })],
    ["unfolded", (v) => void (v.rows[0].unfolded = 1)],
    ["group_in_part", (v) => void (v.rows[0].group_in_part = 3)],
    ["interval", (v) => void (settings(v).interval_mm = "0.1")],
    ["options", (v) => void (settings(v).options = [])],
    ["hatch bool", (v) => void (((settings(v).options as Raw).hatch as Raw).crosshatch = "on")],
    ["hatch overscan", (v) => void (((settings(v).options as Raw).hatch as Raw).overscan_pct = 2.5)],
    ["hatch angle", (v) => void (((settings(v).options as Raw).hatch as Raw).scan_angle_deg = null)],
    ["z offset", (v) => void ((settings(v).options as Raw).z_offset_mm = "1")],
    ["z shown", (v) => void (settings(v).z_offset_shown = 1)],
  ];
  for (const [name, change] of wrong) {
    const broken = structuredClone(good);
    change(broken);
    assert.equal(decodeOperationsView(broken), null, name);
  }
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
