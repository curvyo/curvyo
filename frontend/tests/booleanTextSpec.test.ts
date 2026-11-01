// Independent check of the sentences in `specs/0016-boolean-operations/specification.md`
// (criteria 1, 2, 15 to 17, 29; 38 is withdrawn by `specs/0055-compound-path-node-editing/`), written from the specification before reading the
// implementation. Every expected string is copied from the specification, not from the code.

import assert from "node:assert/strict";
import { test } from "node:test";

import {
  BOOLEAN_OPS,
  operationName,
  operationRule,
  refusalOutlinesObjects,
  refusalText,
  tooltipNote,
} from "../src/lib/booleanText.ts";

const NAMES = ["Union", "Difference", "Intersection", "Exclusion", "Reverse difference"];

test("criterion 1: five operations in the rail order with exact names", () => {
  assert.deepEqual(BOOLEAN_OPS.map(operationName), NAMES);
});

test("criterion 2: rule lines", () => {
  assert.equal(operationRule("difference"), "The lowest selected object minus the others.");
  assert.equal(operationRule("union"), "Everything covered by any selected object.");
  assert.equal(operationRule("intersection"), "Only what every selected object covers.");
  assert.equal(operationRule("exclusion"), "Areas covered an odd number of times.");
  assert.equal(operationRule("reverse_difference"), "The top selected object minus the others.");
  for (const op of BOOLEAN_OPS) {
    assert.doesNotMatch(operationRule(op), /ctrl|shift|cmd|\+/i, "no shortcut text");
  }
});

test("criteria 1 and 2: tooltip notes", () => {
  const dim = { needsTwo: true, open: 0 };
  assert.equal(tooltipNote(dim, true), "Select two or more objects.");
  assert.equal(tooltipNote(dim, false), "Select two or more objects. Use the Select tool.");
  // 0054 criterion 23 replaces the open-path note of criterion 2.
  assert.equal(
    tooltipNote({ needsTwo: false, open: 2 }, true),
    "Closes 2 open outlines with a straight line.",
  );
  assert.equal(
    tooltipNote({ needsTwo: false, open: 0 }, true),
    "Replaces the selection.",
  );
  // another tool wins over an open path
  assert.match(tooltipNote({ needsTwo: true, open: 1 }, false), /Use the Select tool\.$/);
});

test("criterion 15 is withdrawn by 0054 criterion 21: no open-path refusal is left", () => {
  assert.equal(
    refusalText("union", { kind: "open_paths", count: 1, of: 3, compound: false, closedOpen: 0, removedOpen: 0 }),
    null,
  );
});

test("criterion 16: no-area refusal", () => {
  assert.equal(
    refusalText("exclusion", { kind: "no_area", count: 1, of: 3, compound: false }),
    "Exclusion needs shapes that enclose an area. 1 of 3 selected objects has no area. Nothing was changed.",
  );
});

test("criterion 17: empty refusals, each followed by Nothing was changed.", () => {
  const empty = { kind: "empty", count: 0, of: 0, compound: false };
  assert.equal(
    refusalText("intersection", empty),
    "Intersection is empty: the selected objects share no area. Nothing was changed.",
  );
  assert.equal(
    refusalText("difference", empty),
    "Difference is empty: the lowest object is covered completely. Nothing was changed.",
  );
  assert.equal(
    refusalText("reverse_difference", empty),
    "Reverse difference is empty: the top object is covered completely. Nothing was changed.",
  );
  assert.equal(
    refusalText("exclusion", empty),
    "Exclusion is empty: no area is covered an odd number of times. Nothing was changed.",
  );
});

test("criterion 16: which refusals outline objects", () => {
  const r = (kind: string) => refusalOutlinesObjects({ kind, count: 1, of: 2, compound: false });
  assert.equal(r("open_paths"), false, "0054: no open-path refusal");
  assert.equal(r("no_area"), true);
  assert.equal(r("empty"), false);
  assert.equal(r("applied"), false);
  assert.equal(r("needs_two"), false);
});

test("every refusal text ends with Nothing was changed. and has no placeholder left", () => {
  for (const op of BOOLEAN_OPS) {
    for (const kind of ["no_area", "empty", "out_of_range"]) {
      const text = refusalText(op, { kind, count: 2, of: 4, compound: false });
      assert.ok(text !== null, `${op}/${kind}`);
      assert.ok(text.endsWith("Nothing was changed."), text);
      assert.doesNotMatch(text, /undefined|NaN|\$\{|<Op>|<Operation>/);
    }
  }
});

// No success text since the customer rule of 2026-10-31: a result that is no refusal says nothing.
test("ignored, needs_two and applied results have no refusal text", () => {
  for (const kind of ["ignored", "needs_two", "applied"]) {
    assert.equal(refusalText("union", { kind, count: 2, of: 2, compound: false }), null, kind);
  }
});
