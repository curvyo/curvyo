// The Select bar's kind buttons (`specs/0111-laser-operation-tools` criteria 1, 2, 11,
// `ux-notes.md` 3.2, 4.2): names are the kind word alone; inside an entered group the bar holds
// only this group and is absent with no member selected.
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { enteredBarShown, kindButtonName, leavesEnteredGroup } from "../src/lib/kindButtons.ts";
import { decodeOperationsView } from "../src/lib/operationsView.ts";

const DIR = new URL("../../curvyo-editor-wasm/tests/fixtures/operations_view/", import.meta.url);

test("each button is named by its kind word and nothing else (AC 2)", () => {
  const view = decodeOperationsView(JSON.parse(readFileSync(new URL("select_bar_kinds.json", DIR), "utf8")));
  assert.deepEqual(
    view?.selectBarKinds?.map((button) => kindButtonName(button.kind)),
    ["Cut", "Score", "Engrave", "Engrave + Outline", "Cut-out"],
  );
});

test("the bar shows the group only while the session lists buttons (AC 1, 11)", () => {
  assert.equal(enteredBarShown(null), false);
  assert.equal(enteredBarShown([]), false);
  assert.equal(enteredBarShown([{}]), true);
});

test("the kind button source names no tip text and no Shift line in its tooltip (M1)", () => {
  const source = readFileSync(new URL("../src/components/OperationKindGroup.tsx", import.meta.url), "utf8");
  assert.equal(/Shift:\s*new operation/i.test(source), false);
  assert.equal(/content=\{name\}/.test(source), true);
});

test("Escape on a kind button leaves an entered group, nothing else does (AC 11)", () => {
  assert.equal(leavesEnteredGroup("Escape", true), true);
  assert.equal(leavesEnteredGroup("Escape", false), false);
  assert.equal(leavesEnteredGroup("Enter", true), false);
});
