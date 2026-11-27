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
    "options_image",
    "options_image_gradient",
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
  // The fixture has the five older kinds; Image has its own fixture (`options_image`).
  assert.deepEqual(new Set(kinds), new Set(OPERATION_KINDS.filter((kind) => kind !== "image")));
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

test("the view has no summary counts (they are the output group's checks, `0029` P28) and names no missing item", () => {
  const view = decodeOperationsView(fixture("five_kinds_cuts_last"));
  assert.ok(view);
  assert.equal(view.rows.length, 6);
  assert.ok(!("summary" in view) && !("items" in view));
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
  const brokenRun = structuredClone(good);
  brokenRun.rows[0].run = { digit: "1", time: "–" };
  assert.equal(decodeOperationsView(brokenRun), null);
  const brokenNumbers = structuredClone(good);
  brokenNumbers.rows[0].numbers = [{ text: "35 %", typed: ["colour"] }];
  assert.equal(decodeOperationsView(brokenNumbers), null);
  const missingName = structuredClone(good) as Record<string, unknown> & { rows: Array<Record<string, unknown>> };
  delete missingName.rows[0].accessible_name;
  assert.equal(decodeOperationsView(missingName), null);
});

test("a row that runs carries its ring digit, time, numbers and accessible name; one that does not has none (`0029` UX 6.1)", () => {
  const engrave = decodeOperationsView(fixture("options_engrave"));
  assert.ok(engrave);
  const row = engrave.rows[0];
  assert.deepEqual(row.run, { digit: 1, time: "27 s" });
  assert.deepEqual(row.numbers, [{ text: "40 %, 3,000 mm/min, 1 ×, 0.25 mm", typed: [] }]);
  assert.equal(row.accessibleName, "Engrave 1, run 1, 27 s");
  const mixed = decodeOperationsView(fixture("five_kinds_cuts_last"));
  assert.ok(mixed);
  const off = mixed.rows.find((r) => r.name === "Cut 2");
  assert.ok(off);
  assert.equal(off.run, null);
  assert.equal(off.accessibleName, "Cut 2");
  const damaged = decodeOperationsView(fixture("options_damaged"));
  assert.equal(damaged?.rows[0].numbers, null, "damaged settings have no numbers line");
});

// `specs/0111-laser-operation-tools` Part C1: the new fields, each checked (AC 1, 10, 17 to 21).
type Raw = Record<string, unknown> & { rows: Array<Record<string, unknown>> };

test("the kind buttons decode in bar order with their descriptions (AC 1, 2)", () => {
  const view = decodeOperationsView(fixture("select_bar_kinds"));
  assert.ok(view?.selectBarKinds);
  assert.deepEqual(
    view.selectBarKinds.map((button) => button.kind),
    ["cut", "score", "engrave", "engrave_outline", "cut_out", "image"],
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
    image: null,
    tabs: null,
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
import { lowerPart } from "../src/lib/operationOptionRows.ts";

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
  assert.deepEqual([...states].sort(), ["damaged", "live", "own", "stored"]);
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

test("a stored row keeps the reason; a row without op_settings has the default numbers", () => {
  const stored = decodeSettings(settingsRows("settings_stored")[0].settings);
  assert.equal(stored?.reason, "Material has no Score set, using stored numbers");
  const derived = decodeSettings(settingsRows("settings_live")[1].settings);
  assert.equal(derived?.state, "own");
  assert.deepEqual(derived?.sets[0].run, { powerPct: 10, speedMmMin: 1000, passes: 1 });
  const broken = structuredClone(settingsRows("settings_live")[1].settings) as { sets: { run: { power_pct: unknown } }[] };
  broken.sets[0].run.power_pct = null;
  assert.equal(decodeSettings(broken), undefined, "a field is never empty (specs/0150 criterion 27)");
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

test("an Image row shows the Image settings group in its options (specs/0123 AC 11)", () => {
  const view = decodeOperationsView(fixture("options_image"));
  assert.ok(view);
  const image = view.rows[0]?.settings?.options?.image;
  assert.ok(image);
  // Superseded by the Dither on the picture (2026-11-20): a picture-only row has no Dither of its own; Lowest power shows while
  // its picture burns Grey (Frage 284 B); "Dither for gradient fills" only with a gradient fill.
  assert.equal(image.dither, null);
  assert.equal(image.lowestPower?.field, "lowest_power");
  assert.equal(image.lowestPower?.value, 12.5);
  const gradient = decodeOperationsView(fixture("options_image_gradient"))?.rows[0]?.settings?.options?.image;
  assert.ok(gradient?.dither);
  assert.equal(gradient.dither.dither, "grey");
  assert.equal(gradient.dither.modes.length, 9, "the two screens of specs/0135 last");
  assert.deepEqual(gradient.dither.screenFields, [], "no screen fields for Grey");
  assert.equal(gradient.dither.modeField, null, "no Lowest power in the Dither block");
  assert.deepEqual(
    image.fields.map((f) => [f.field, f.value, f.unit]),
    [
      ["brightness", -10, "%"],
      ["contrast", 20, "%"],
      ["gamma", 1.5, ""],
    ],
  );
  assert.equal(image.invert, true);
  const plain = decodeOperationsView(fixture("options_engrave"));
  assert.equal(plain?.rows[0]?.settings?.options?.image, null, "an Engrave: never the group");
});

test("an Image row has no Crosshatch toggle, and a damaged dither decodes to no pressed cell (specs/0123 P40, P46)", () => {
  const raw = fixture("options_image") as Raw;
  const view = decodeOperationsView(raw);
  const row = view?.rows[0];
  assert.ok(row?.settings);
  assert.equal(row.settings.options?.hatch?.crosshatch, null);
  assert.deepEqual(
    lowerPart(row.kind, row.settings)?.toggles.map((toggle) => toggle.option),
    ["bidirectional"],
  );
  const damaged = structuredClone(fixture("options_image_gradient")) as Raw;
  const settings = damaged.rows[0].settings as { options: { image: { dither: Record<string, unknown> } } };
  settings.options.image.dither.dither = null;
  assert.equal(decodeOperationsView(damaged)?.rows[0].settings?.options?.image?.dither?.dither, null);
});

test("specs/0135 criteria 42, 43: the screen modes decode with Cell size or Line pitch and Angle", () => {
  const raw = fixture("options_image_gradient") as {
    rows: Array<{ settings: { options: { image: { dither: Record<string, unknown> } } } }>;
  };
  const image = raw.rows[0].settings.options.image.dither;
  image.dither = "line_screen";
  image.mode_field = null;
  image.screen_fields = [
    { field: "screen_size", label: "Line pitch", value: 0.8, unit: "mm", range: "Line pitch is 0.20 to 5.00 mm." },
    { field: "screen_angle", label: "Angle", value: 45, unit: "°", range: "Angle is 0 to 179°." },
  ];
  const decoded = decodeOperationsView(raw)?.rows[0]?.settings?.options?.image?.dither;
  assert.ok(decoded);
  assert.equal(decoded.dither, "line_screen");
  assert.deepEqual(
    decoded.screenFields.map((f) => [f.field, f.label, f.value, f.unit]),
    [
      ["screen_size", "Line pitch", 0.8, "mm"],
      ["screen_angle", "Angle", 45, "°"],
    ],
  );
  image.screen_fields = [{ field: "screen_pitch", label: "x", value: 1, unit: "", range: "x" }];
  assert.equal(decodeOperationsView(raw), null, "an unknown screen field is refused");
  delete image.screen_fields;
  assert.equal(decodeOperationsView(raw), null, "the list is required");
});

test("a Cut row carries the Tabs group and an Engrave row none (specs/0139 criterion 1)", () => {
  const cut = decodeOperationsView(fixture("options_cut_z_hidden"));
  assert.ok(cut);
  assert.deepEqual(cut.rows[0]?.settings?.options?.tabs, { count: 0, widthMm: 0.5, skipInner: false, placed: null });
  assert.equal(cut.rows[0]?.itemTabs, null);
  assert.equal(cut.tabsTool, null, "the Tabs toggle is hidden without a selected cut item");
  assert.equal(cut.tabChip, null);
  const engrave = decodeOperationsView(fixture("options_engrave"));
  assert.ok(engrave);
  assert.ok(engrave.rows.every((row) => row.settings?.options?.tabs == null));
});
