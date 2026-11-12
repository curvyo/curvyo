// The texts of the Settings block against the Rust fixtures (`specs/0028` ux-notes 8).
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { decodePick, decodeSettings, type OperationSettings } from "../src/lib/operationSettingsView.ts";
import {
  PICK_FILTER_ABOVE,
  blockHead,
  blockSummary,
  canEditMaterial,
  disabledReason,
  fieldName,
  filterPick,
  kindsText,
  materialDescription,
  materialLine,
  pickValue,
  resetStatus,
  stateLines,
  thicknessText,
} from "../src/lib/operationSettingsRows.ts";

const DIR = new URL("../../curvyo-editor-wasm/tests/fixtures/operations_view/", import.meta.url);

function rows(name: string): { name: string; settings: OperationSettings }[] {
  const json = JSON.parse(readFileSync(new URL(`${name}.json`, DIR), "utf8")) as { rows: { name: string; settings: unknown }[]; pick: unknown };
  return json.rows.map((row) => ({ name: row.name, settings: decodeSettings(row.settings) as OperationSettings }));
}

function pick(name: string) {
  const json = JSON.parse(readFileSync(new URL(`${name}.json`, DIR), "utf8")) as { pick: unknown };
  return decodePick(json.pick) ?? [];
}

test("Differs shows both lines in the order of the UX table, and the typed field gets its Material line", () => {
  const [differs] = rows("settings_live");
  assert.deepEqual(stateLines(differs.settings), [{ glyph: "differs", text: "Differs from Birch plywood" }]);
  const main = differs.settings.sets[0];
  assert.equal(materialLine(main, "power"), "Material 35");
  assert.equal(materialDescription(main, "power"), "Material 35 %");
  assert.equal(materialLine(main, "speed"), null);
  assert.equal(materialLine(differs.settings.sets[1], "power"), null);
});

test("stored states show the not-found line first, then Differs", () => {
  const stored = rows("settings_stored");
  assert.deepEqual(stateLines(stored[0].settings), [
    { glyph: "not-found", text: "Material has no Score set, using stored numbers" },
  ]);
  assert.deepEqual(
    stateLines(stored[1].settings).map((line) => line.glyph),
    ["not-found", "differs"],
  );
  assert.equal(materialLine(stored[1].settings.sets[0], "speed"), "Material 300");
});

test("Edit material shows only when the material is readable in the pick", () => {
  const live = rows("settings_live");
  const list = pick("settings_live");
  assert.equal(canEditMaterial(live[0].settings, list), true);
  assert.equal(canEditMaterial(live[1].settings, list), false);
  const stored = rows("settings_stored");
  assert.equal(canEditMaterial(stored[1].settings, pick("settings_stored")), false);
  assert.equal(canEditMaterial(stored[0].settings, pick("settings_stored")), true);
});

test("the pick disables a material without the needed set, with the words", () => {
  const [engrave] = rows("settings_live");
  assert.equal(disabledReason(engrave.settings, "11111111-1111-4111-8111-111111111111"), "No Engrave set");
  assert.equal(disabledReason(engrave.settings, "22222222-2222-4222-8222-222222222222"), null);
  assert.equal(disabledReason(null, "x"), null);
});

test("the source value is the name, Own numbers, or Mixed", () => {
  const live = rows("settings_live");
  assert.deepEqual(pickValue([live[0]]), { text: "Birch plywood", mixed: false });
  assert.deepEqual(pickValue([live[1]]), { text: "Own numbers", mixed: false });
  assert.deepEqual(pickValue([live[0], live[2]]), { text: "Mixed", mixed: true });
  assert.deepEqual(pickValue([live[1], live[1]]), { text: "Own numbers", mixed: false });
});

test("the head, the block summary and the reset status read as the notes say", () => {
  assert.equal(blockHead(["Cut 1"]), "Cut 1");
  assert.equal(blockHead(["a", "b", "c"]), "3 operations");
  const [differs] = rows("settings_live");
  assert.equal(blockSummary(differs.settings), "Power 37.5 %, speed 6000 mm/min, 1 pass, from Birch plywood, differs");
  assert.equal(blockSummary(rows("settings_live")[1].settings), "No settings");
  assert.equal(blockSummary(rows("settings_own_none_damaged")[1].settings), "Damaged settings");
  assert.equal(resetStatus("Birch 3 mm"), "Numbers reset to Birch 3 mm.");
});

test("the filter narrows by name and the threshold is above 8", () => {
  const list = pick("settings_live");
  assert.deepEqual(filterPick(list, " BIRCH ").map((entry) => entry.name), ["Birch plywood"]);
  assert.equal(filterPick(list, "").length, 2);
  assert.equal(PICK_FILTER_ABOVE, 8);
});

test("thickness and kinds read as in the rows", () => {
  assert.equal(thicknessText(3), "3.0 mm");
  assert.equal(thicknessText(0.25), "0.25 mm");
  assert.equal(kindsText(["cut", "engrave"]), "Cut and Engrave sets");
  assert.equal(kindsText(["cut", "score", "engrave"]), "Cut, Score and Engrave sets");
  assert.equal(kindsText(["cut"]), "Cut set");
  assert.equal(kindsText([]), "no sets");
});

test("field names carry the unit; two sets say hatch or outline", () => {
  assert.equal(fieldName("power", "main", false), "Power, percent");
  assert.equal(fieldName("speed", "outline", true), "Speed, millimetres per minute, outline");
});
