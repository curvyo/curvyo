// The Fixtures tab's set, placement and Apply logic (`specs/0113-fixture-sets-and-pins`
// `ux-notes.md` sections 3 to 5, 9, 10). Pure functions: the components only draw what they return.
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { decodeFixturesView } from "../src/lib/fixturesView.ts";
import {
  applyKeyTarget,
  applyLabel,
  applyReplacesText,
  candidateCountText,
  cleanName,
  dragStarts,
  fixtureArmStatus,
  noticeMs,
  noticeInTab,
  noticeUnder,
  onPageName,
  placementNudge,
  placingStatus,
  removedStatus,
  renamedText,
  setArmStatus,
  setKeyAction,
  setLanding,
  setsCountText,
} from "../src/lib/fixtureSets.ts";

const DIR = new URL("../../curvyo-editor-wasm/tests/fixtures/fixtures_view/", import.meta.url);

function view(name: string) {
  const decoded = decodeFixturesView(JSON.parse(readFileSync(new URL(`${name}.json`, DIR), "utf8")));
  assert.ok(decoded, name);
  return decoded;
}

test("the Machine sets heading counts the sets on this page, and only while the page has a set", () => {
  assert.equal(setsCountText(view("two_sets_one_off")), "1 of 2 on this page");
  assert.equal(setsCountText(view("empty_page")), null);
});

test("arming a set's trash says what goes with it; a fixture's says its name", () => {
  const [first, second] = view("two_sets_one_off").sets;
  assert.equal(setArmStatus(first), "Press again to delete Set 1. Its 2 fixtures go with it.");
  assert.equal(setArmStatus(second), "Press again to delete Coaster jig. Its 1 fixture goes with it.");
  assert.equal(setArmStatus({ name: "Set 3", fixtures: [] }), "Press again to delete Set 3.");
  assert.equal(fixtureArmStatus("Pins 1"), "Press again to delete Pins 1.");
  assert.equal(removedStatus("Pins 1"), "Pins 1 removed.");
  assert.equal(onPageName("Set 1"), "Set 1 on this page");
});

test("placement says 'Placing Pin.' once on arming", () => {
  assert.equal(placingStatus("pin"), "Placing Pin.");
  assert.equal(placingStatus("screw"), "Placing Screw.");
});

test("a name is trimmed, 1 to 40 characters", () => {
  assert.equal(cleanName("  Coaster jig "), "Coaster jig");
  assert.equal(cleanName("   "), null);
  assert.equal(cleanName("x".repeat(60))?.length, 40);
});

test("Up, Down, Home and End move the focus without wrap", () => {
  assert.deepEqual(setKeyAction("ArrowDown", false, 0, 3, false), { kind: "focus", index: 1 });
  assert.deepEqual(setKeyAction("ArrowDown", false, 2, 3, false), { kind: "focus", index: 2 });
  assert.deepEqual(setKeyAction("ArrowUp", false, 0, 3, false), { kind: "focus", index: 0 });
  assert.deepEqual(setKeyAction("Home", false, 2, 3, false), { kind: "focus", index: 0 });
  assert.deepEqual(setKeyAction("End", false, 0, 3, false), { kind: "focus", index: 2 });
  assert.equal(setKeyAction("ArrowDown", false, 0, 0, false), null);
});

test("Enter and Space fold, F2 renames, Delete arms and a second Delete or Enter removes", () => {
  assert.deepEqual(setKeyAction("Enter", false, 0, 2, false), { kind: "toggle" });
  assert.deepEqual(setKeyAction(" ", false, 0, 2, false), { kind: "toggle" });
  assert.deepEqual(setKeyAction("F2", false, 0, 2, false), { kind: "rename" });
  assert.deepEqual(setKeyAction("Delete", false, 0, 2, false), { kind: "arm" });
  assert.deepEqual(setKeyAction("Delete", false, 0, 2, true), { kind: "remove" });
  assert.deepEqual(setKeyAction("Enter", false, 0, 2, true), { kind: "remove" });
  assert.equal(setKeyAction("x", false, 0, 2, false), null);
});

test("Alt+arrows, Alt+Home and Alt+End move the set; a move that changes nothing is no action", () => {
  assert.deepEqual(setKeyAction("ArrowUp", true, 2, 3, false), { kind: "move", to: 1 });
  assert.deepEqual(setKeyAction("ArrowDown", true, 0, 3, false), { kind: "move", to: 1 });
  assert.deepEqual(setKeyAction("Home", true, 2, 3, false), { kind: "move", to: 0 });
  assert.deepEqual(setKeyAction("End", true, 0, 3, false), { kind: "move", to: 2 });
  assert.equal(setKeyAction("ArrowUp", true, 0, 3, false), null);
  assert.equal(setKeyAction("End", true, 2, 3, false), null);
});

test("a drag starts after 4 px of travel, either way", () => {
  assert.equal(dragStarts(100, 103), false);
  assert.equal(dragStarts(100, 104), true);
  assert.equal(dragStarts(100, 95), true);
});

test("a drag lands in the gap under the last midpoint the pointer passed", () => {
  const middles = [20, 60, 100];
  assert.deepEqual(setLanding(0, middles, 10), { gap: 0, to: null });
  assert.deepEqual(setLanding(0, middles, 70), { gap: 2, to: 1 });
  assert.deepEqual(setLanding(0, middles, 500), { gap: 3, to: 2 });
  assert.deepEqual(setLanding(2, middles, 10), { gap: 0, to: 0 });
  assert.deepEqual(setLanding(2, middles, 70), { gap: 2, to: null });
  assert.deepEqual(setLanding(1, middles, 30), { gap: 1, to: null });
});

test("keyboard placement: arrows move 1 mm, Shift 10 mm, y grows downward; other keys are not a move", () => {
  assert.deepEqual(placementNudge("ArrowLeft", false), [-1, 0]);
  assert.deepEqual(placementNudge("ArrowRight", true), [10, 0]);
  assert.deepEqual(placementNudge("ArrowUp", false), [0, -1]);
  assert.deepEqual(placementNudge("ArrowDown", true), [0, 10]);
  assert.equal(placementNudge("Enter", false), null);
});

test("the Apply list's texts and keys", () => {
  assert.equal(applyLabel(4), "Apply 4 fixtures");
  assert.equal(applyLabel(1), "Apply 1 fixture");
  assert.equal(candidateCountText(4), "4 job fixtures");
  assert.equal(candidateCountText(1), "1 job fixture");
  const ids = ["a", "b", "c"];
  assert.equal(applyKeyTarget("ArrowDown", ids, null), "a");
  assert.equal(applyKeyTarget("ArrowDown", ids, "a"), "b");
  assert.equal(applyKeyTarget("ArrowDown", ids, "c"), "c");
  assert.equal(applyKeyTarget("ArrowUp", ids, "b"), "a");
  assert.equal(applyKeyTarget("ArrowUp", ids, "a"), "a");
  assert.equal(applyKeyTarget("x", ids, "a"), null);
  assert.equal(applyKeyTarget("ArrowDown", [], null), null);
});

test("the outside-the-work-area notice stays 5 s, a clamp notice 3 s, and none is over 70 characters", () => {
  assert.equal(noticeMs("4 fixtures lie outside the work area."), 5000);
  assert.equal(noticeMs("Columns are 1 to 50. Set to 50."), 3000);
  assert.ok("4 fixtures lie outside the work area.".length <= 70);
});

test("a rename is a change only when the trimmed text is not empty and differs", () => {
  assert.equal(renamedText(" Jig ", "Set 1"), "Jig");
  assert.equal(renamedText("Set 1", "Set 1"), null);
  assert.equal(renamedText("  ", "Set 1"), null);
});

test("a clamp notice shows under the field that was committed, any other notice at the tab's foot", () => {
  const clamp = { text: "Diameter is 0.5 to 1,000 mm. Set to 1,000 mm.", field: "f1" };
  assert.equal(noticeUnder(clamp, "f1"), clamp.text);
  assert.equal(noticeUnder(clamp, "f2"), null);
  assert.equal(noticeInTab(clamp), null);
  const apply = { text: "4 fixtures lie outside the work area.", field: null };
  assert.equal(noticeUnder(apply, "f1"), null);
  assert.equal(noticeInTab(apply), apply.text);
  assert.equal(noticeUnder(null, "f1"), null);
  assert.equal(noticeInTab(null), null);
});

test("Apply names how many job fixtures of this page it replaces (criterion 35)", () => {
  assert.equal(applyReplacesText(0), null);
  assert.equal(applyReplacesText(1), "Replaces 1 job fixture on this page.");
  assert.equal(applyReplacesText(12), "Replaces 12 job fixtures on this page.");
});
