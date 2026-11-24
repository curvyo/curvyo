// Tester acceptance cases for `specs/0150-operation-always-has-numbers` criterion 27 (the row):
// every unfolded or folded row of a laser page's operations shows numbers, never a dash and never
// "No settings". The fixtures are the Rust states of `curvyo-editor-wasm/tests/fixtures/
// operations_view/`. The fixture jobs keep "No settings" (`specs/0103`, Frage 339 A): checked in the
// component source.
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync, readdirSync } from "node:fs";
import { decodeOperationsView } from "../src/lib/operationsView.ts";

const DIR = new URL("../../curvyo-editor-wasm/tests/fixtures/operations_view/", import.meta.url);

const NAMES = readdirSync(DIR)
  .filter((file) => file.endsWith(".json") && !file.startsWith("settings_"))
  .map((file) => file.slice(0, -".json".length));

function fixture(name: string): unknown {
  return JSON.parse(readFileSync(new URL(`${name}.json`, DIR), "utf8"));
}

test("criterion 27: a laser row of a known kind never shows a dash or No settings", () => {
  let checked = 0;
  for (const name of NAMES) {
    const view = decodeOperationsView(fixture(name));
    assert.ok(view, `${name} decodes`);
    for (const row of view.rows) {
      const state = (row.settings as unknown as { state?: string } | null)?.state;
      if (row.kind === "unknown" || row.settings === null || state === "damaged") continue;
      checked += 1;
      assert.ok(row.numbers !== null && row.numbers.length >= 1, `${name}: ${row.name} has line 3`);
      for (const line of row.numbers ?? []) {
        assert.ok(!/(^|[ ,])–($|[ ,])/.test(line.text), `${name}: ${row.name} number line "${line.text}"`);
        assert.ok(!line.text.includes("No settings"), `${name}: ${row.name}`);
      }
      assert.ok(row.run === null || row.run.time !== "–", `${name}: ${row.name} time`);
      assert.ok(!row.accessibleName.includes("No settings"), `${name}: ${row.name} name`);
    }
  }
  assert.ok(checked > 0, "the fixtures hold rows of known kinds");
});

test("criterion 27 and 339 A: the fixture job section keeps No settings without a pick", () => {
  const source = readFileSync(
    new URL("../src/components/panel/FixtureOutputPart.tsx", import.meta.url),
    "utf8",
  );
  assert.ok(source.includes("No settings"), "fixture jobs keep their No settings line");
});

test("criterion 27: the operation settings source no longer renders No settings for a laser row", () => {
  const source = readFileSync(
    new URL("../src/components/panel/OperationSettingsBlock.tsx", import.meta.url),
    "utf8",
  );
  assert.ok(!source.includes("No settings"), "the Settings block has no No settings line");
});
