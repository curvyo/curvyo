import assert from "node:assert/strict";
import { test } from "node:test";

import {
  badgeWord,
  deleteLabel,
  selectClonesLabel,
  sketchPageReading,
  stateEmphasised,
  stateGlyph,
  stateWord,
  usageLine,
  usedByTail,
} from "../src/lib/sketchPages.ts";

const row = (clones: number) => ({ state: { state: "full" as const }, clones, pages: 1, drawable: true });

test("the row state reads short on screen and spelled out for a screen reader (criterion 14)", () => {
  assert.equal(stateWord({ state: "full" }), "Fully constrained");
  assert.equal(stateWord({ state: "free", dof: 3 }), "3 DOF");
  assert.equal(stateWord({ state: "empty" }), "Empty");
  assert.equal(stateWord({ state: "failed" }), "Solver failed");
  assert.equal(sketchPageReading("Bracket", { state: "full" }), "Bracket, Sketch page, fully constrained");
  assert.equal(sketchPageReading("Bracket", { state: "free", dof: 3 }), "Bracket, Sketch page, 3 degrees of freedom");
  assert.equal(stateGlyph({ state: "conflict" }), "TriangleAlert");
  assert.ok(stateEmphasised({ state: "conflict" }) && stateEmphasised({ state: "failed" }));
  assert.ok(!stateEmphasised({ state: "free", dof: 2 }));
});

test("the trash says what becomes of the clones (criterion 16)", () => {
  assert.equal(deleteLabel("Bracket", row(3)), "Delete Bracket and its sketch; 3 clones become paths");
  assert.equal(deleteLabel("Bracket", row(1)), "Delete Bracket and its sketch; 1 clone becomes a path");
  assert.equal(deleteLabel("Bracket", row(0)), "Delete Bracket and its sketch");
  assert.equal(deleteLabel("Page 1", null), "Delete page: Page 1");
});

test("the usage line and the clone button count clones and pages (criterion 12)", () => {
  assert.equal(usageLine(0, 0), "Not used on other pages");
  assert.equal(usageLine(1, 1), "Used by 1 linked clone on 1 page");
  assert.equal(usageLine(3, 2), "Used by 3 linked clones on 2 pages");
  assert.equal(usedByTail(3), " · used by 3 clones");
  assert.equal(usedByTail(0), "");
  assert.equal(selectClonesLabel(1), "Select clone");
  assert.equal(selectClonesLabel(4), "Select clones");
});

test("the drop badge words (criteria 8 to 10)", () => {
  assert.equal(badgeWord("clone", "Bracket"), "Linked clone");
  assert.equal(badgeWord("not_on_sketch_page", "Bracket"), "Not on a sketch page");
  assert.equal(badgeWord("no_sketch", "Bracket"), "Bracket has no sketch yet");
  assert.equal(badgeWord("no_sketch", "A very long page name indeed here"), "A very long page name i… has no sketch yet");
  assert.equal(badgeWord("none", "Bracket"), null);
});
