// The lower part of an unfolded row (`specs/0111-laser-operation-tools` `ux-notes.md` 6.2, 6.3,
// 6.4, 11): which controls a row shows, their names, ranges and Up/Down steps. Pure functions.
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { decodeOperationsView } from "../src/lib/operationsView.ts";
import {
  INTERVAL_FIELD,
  lowerPart,
  optionStep,
  optionText,
  OPTION_FIELDS,
  toggleNames,
} from "../src/lib/operationOptionRows.ts";

const DIR = new URL("../../curvyo-editor-wasm/tests/fixtures/operations_view/", import.meta.url);
function settingsOf(name: string, kind?: string) {
  const view = decodeOperationsView(JSON.parse(readFileSync(new URL(`${name}.json`, DIR), "utf8")));
  assert.ok(view);
  const row = kind ? view.rows.find((r) => r.kind === kind) : view.rows[0];
  assert.ok(row?.settings, `${name} ${kind}`);
  return { row, settings: row.settings };
}

test("an Engrave row shows the hatch toggles and fields, Z offset only with the capability (AC 20, 21)", () => {
  const { settings } = settingsOf("options_engrave");
  const part = lowerPart("engrave", settings);
  assert.ok(part);
  assert.deepEqual(part.toggles.map((t) => [t.option, t.pressed]), [["crosshatch", true], ["bidirectional", false]]);
  assert.deepEqual(part.fields.map((f) => f.key), ["scan_angle", "overscan", "z_offset"]);
  assert.deepEqual(part.fields.map((f) => f.shown), ["15", "5", "1.5"]);
  const hidden = { ...settings, zOffsetShown: false };
  assert.deepEqual(lowerPart("engrave", hidden)?.fields.map((f) => f.key), ["scan_angle", "overscan"]);
  assert.deepEqual(lowerPart("engrave_outline", hidden)?.toggles.length, 2);
});

test("a Cut shows Z offset alone with the capability, and no lower part without it (AC 20, 21)", () => {
  const { settings } = settingsOf("options_cut_z_hidden");
  assert.equal(lowerPart("cut", settings), null);
  const part = lowerPart("score", { ...settings, zOffsetShown: true });
  assert.deepEqual(part?.fields.map((f) => f.key), ["z_offset"]);
  assert.deepEqual(part?.toggles, []);
  assert.deepEqual(lowerPart("cut_out", { ...settings, zOffsetShown: true })?.fields.length, 1);
});

test("Interval shows 0.10 for Engrave and Engrave + Outline also with No settings, never for a Cut (AC 19)", () => {
  for (const kind of ["engrave", "engrave_outline"]) {
    const { settings } = settingsOf("five_kinds_cuts_last", kind);
    assert.equal(settings.state, "no_settings");
    assert.equal(settings.intervalMm, 0.1, kind);
  }
  for (const kind of ["score", "cut", "cut_out"]) {
    assert.equal(settingsOf("five_kinds_cuts_last", kind).settings.intervalMm, null, kind);
  }
});

test("damaged settings and an unknown kind show no lower part", () => {
  const { settings } = settingsOf("options_damaged");
  assert.equal(lowerPart("engrave", settings), null);
  const engrave = settingsOf("options_engrave").settings;
  assert.equal(lowerPart("unknown", engrave), null);
});

test("names, units and ranges follow the notes (6.3, 11)", () => {
  const byKey = Object.fromEntries(OPTION_FIELDS.map((f) => [f.key, f]));
  assert.equal(byKey.scan_angle.name, "Scan angle, degrees");
  assert.equal(byKey.overscan.name, "Overscanning, percent");
  assert.equal(byKey.z_offset.name, "Z offset, millimetres");
  assert.equal(INTERVAL_FIELD.name, "Interval, millimetres");
  assert.equal(byKey.scan_angle.label, "Scan angle");
  assert.deepEqual(toggleNames("crosshatch"), "Crosshatch");
  assert.deepEqual(toggleNames("bidirectional"), "Bidirectional fill");
});

test("Up and Down step by the notes: 1 (Shift 10), 1 (10), 0.1 (1), Interval 0.01 (0.1)", () => {
  const scan = optionStep(OPTION_FIELDS.find((f) => f.key === "scan_angle")!);
  assert.equal(scan("15", "ArrowUp", false), "16");
  assert.equal(scan("15", "ArrowDown", true), "5");
  assert.equal(scan("x", "ArrowUp", false), null);
  assert.equal(scan("15", "a", false), null);
  const z = optionStep(OPTION_FIELDS.find((f) => f.key === "z_offset")!);
  assert.equal(z("1.5", "ArrowUp", false), "1.6");
  assert.equal(z("0.3", "ArrowDown", false), "0.2");
  assert.equal(z("1", "ArrowUp", true), "2");
  const over = optionStep(OPTION_FIELDS.find((f) => f.key === "overscan")!);
  assert.equal(over("2", "ArrowDown", true), "-8");
  const interval = optionStep(INTERVAL_FIELD);
  assert.equal(interval("0.1", "ArrowUp", false), "0.11");
  assert.equal(interval("0.1", "ArrowUp", true), "0.2");
});

test("numbers show without trailing zeros", () => {
  assert.equal(optionText(0), "0");
  assert.equal(optionText(0.25), "0.25");
  assert.equal(optionText(-90), "-90");
  assert.equal(optionText(null), "");
});
