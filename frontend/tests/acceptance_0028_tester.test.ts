// Tester acceptance cases of `specs/0028-material-test-library` that `specs/0112` keeps: the
// damaged row's title and reason (29), the keyboard routes of the order (22) and the decoded
// Materials view of the Rust fixtures carrying no colour (19). The Worked cell, the Save as
// material form and the operation use count of `0028` are superseded by `0112` (UX notes 15); the
// replacing cases are in `materialRows.test.ts` and `photoSize.test.ts`.
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync, readdirSync } from "node:fs";
import { materialKeyAction, materialRowName, moveTarget, rowTitle } from "../src/lib/materialRows.ts";
import { decodeMaterialsText } from "../src/lib/materialsView.ts";

const MATERIALS = new URL("../../curvyo-editor-wasm/tests/fixtures/materials_view/", import.meta.url);

test("c29: a damaged record is listed as Damaged material with its reason", () => {
  const row = { state: "damaged", id: "x", reason: "Unknown key colour." } as const;
  assert.equal(rowTitle(row), "Damaged material");
  assert.equal(materialRowName(row), "Damaged material, Unknown key colour.");
});

test("c22: Alt+Up and Alt+Down move a row; plain arrows do not", () => {
  assert.equal(materialKeyAction("ArrowUp", true, false), "move");
  assert.equal(materialKeyAction("ArrowDown", true, false), "move");
  assert.equal(materialKeyAction("ArrowUp", false, false), "none");
  assert.equal(moveTarget(3, 0, "ArrowUp"), null);
  assert.equal(moveTarget(3, 0, "ArrowDown"), 1);
  assert.equal(moveTarget(3, 2, "ArrowDown"), null);
  assert.equal(moveTarget(1, 0, "ArrowDown"), null);
});

test("c19: the decoded Materials view carries no colour", () => {
  const names = readdirSync(MATERIALS)
    .filter((file) => file.endsWith(".json"))
    .map((file) => file.slice(0, -".json".length));
  assert.ok(names.length > 0);
  for (const name of names) {
    const text = readFileSync(new URL(`${name}.json`, MATERIALS), "utf8");
    const view = decodeMaterialsText(text);
    assert.ok(view, name);
    const json = JSON.stringify(view);
    assert.ok(!/colou?r|#[0-9a-f]{3,6}\b|rgb\(/i.test(json.replaceAll("Unknown key colour.", "")), name);
  }
});
