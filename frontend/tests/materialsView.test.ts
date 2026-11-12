// The Materials section decoder against the Rust fixtures of `curvyo-editor-wasm/tests/fixtures/
// materials_view/` (`specs/0028-material-test-library` `adrs.md` section 6). The Rust contract test
// builds the same states and compares them with these files, so a drift fails on both sides.
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync, readdirSync } from "node:fs";
import { decodeMaterialsText, decodeMaterialsView } from "../src/lib/materialsView.ts";

const DIR = new URL("../../curvyo-editor-wasm/tests/fixtures/materials_view/", import.meta.url);

function raw(name: string): string {
  return readFileSync(new URL(`${name}.json`, DIR), "utf8");
}

function fixture(name: string): Record<string, unknown> {
  return JSON.parse(raw(name)) as Record<string, unknown>;
}

test("every Rust state decodes, and nothing else is in the folder", () => {
  const names = readdirSync(DIR)
    .filter((file) => file.endsWith(".json"))
    .map((file) => file.slice(0, -".json".length))
    .sort();
  assert.deepEqual(names, ["damaged_newer_and_used", "empty", "selected_fields"]);
  for (const name of names) {
    assert.ok(decodeMaterialsText(raw(name)), name);
  }
});

test("rows keep their state, kinds and use count; damaged and newer rows carry no numbers", () => {
  const view = decodeMaterialsText(raw("damaged_newer_and_used"));
  assert.ok(view);
  assert.equal(view.profileName, "Generic laser");
  assert.equal(view.nextName, "Material 1");
  assert.deepEqual(
    view.rows.map((row) => row.state),
    ["loaded", "damaged", "newer", "loaded"],
  );
  const [birch, damaged, newer, cardboard] = view.rows;
  assert.deepEqual(birch.state === "loaded" && birch.kinds, ["cut", "score", "engrave"]);
  assert.equal(cardboard.state === "loaded" && cardboard.uses, 1);
  assert.equal(damaged.state === "damaged" && damaged.reason, "Unknown key colour.");
  assert.equal(newer.state === "newer" && newer.name, "Birch plywood");
  assert.equal(view.selected, null);
});

test("the selected material's sets decode with numbers and source lines", () => {
  const view = decodeMaterialsText(raw("selected_fields"));
  assert.ok(view?.selected);
  assert.deepEqual(
    view.selected.sets.map((set) => set.kind),
    ["cut", "score", "engrave"],
  );
  assert.deepEqual(view.selected.sets[0].numbers, { powerPct: 60, speedMmMin: 300, passes: 2 });
  assert.equal(view.selected.sets[1].source, "Edited by hand 2025-11-11");
});

test("the empty section decodes with no rows; another page (null) is null", () => {
  const view = decodeMaterialsText(raw("empty"));
  assert.ok(view);
  assert.deepEqual(view.rows, []);
  assert.equal(decodeMaterialsText("null"), null);
  assert.equal(decodeMaterialsText("not json"), null);
});

test("a missing field or a wrong type is refused, not guessed", () => {
  for (const key of ["profile_name", "rows", "selected", "next_name", "load_notices"]) {
    const broken = fixture("selected_fields");
    delete broken[key];
    assert.equal(decodeMaterialsView(broken), null, `missing ${key}`);
  }
  const wrongType = fixture("selected_fields");
  wrongType.profile_name = 3;
  assert.equal(decodeMaterialsView(wrongType), null);

  const badState = fixture("damaged_newer_and_used");
  (badState.rows as Record<string, unknown>[])[0].state = "corrupt";
  assert.equal(decodeMaterialsView(badState), null);

  const badKind = fixture("damaged_newer_and_used");
  (badKind.rows as Record<string, unknown>[])[0].kinds = ["cut", "laser"];
  assert.equal(decodeMaterialsView(badKind), null);

  const badUses = fixture("damaged_newer_and_used");
  delete (badUses.rows as Record<string, unknown>[])[0].uses;
  assert.equal(decodeMaterialsView(badUses), null);

  const badSet = fixture("selected_fields");
  const selected = badSet.selected as { sets: { numbers: Record<string, unknown> }[] };
  selected.sets[0].numbers.speed_mm_min = "fast";
  assert.equal(decodeMaterialsView(badSet), null);
});
