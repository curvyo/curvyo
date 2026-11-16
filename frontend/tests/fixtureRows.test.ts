// The Fixtures block's row logic (`specs/0102-laser-fixtures` `ux-notes.md` sections 3, 4, 7.2,
// 11): accessible names, the second line, the add kinds at the limits, the slot count and the fit
// words. Pure functions over the decoded view (`fixturesView.ts`).
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { decodeFixturesView, type FixtureRow, type FixturesView } from "../src/lib/fixturesView.ts";
import {
  JOB_ADD_KINDS,
  KIND_WORDS,
  MAX_SLOTS,
  SET_ADD_KINDS,
  setAddKinds,
  decodeFitCue,
  fitWord,
  rowKey,
  rowKeyAction,
  rowName,
  slotCountText,
  slotName,
  stepValue,
  tabStopRow,
} from "../src/lib/fixtureRows.ts";

const DIR = new URL("../../curvyo-editor-wasm/tests/fixtures/fixtures_view/", import.meta.url);

function view(name: string): FixturesView {
  const decoded = decodeFixturesView(JSON.parse(readFileSync(new URL(`${name}.json`, DIR), "utf8")));
  assert.ok(decoded, name);
  return decoded;
}

function row(patch: Partial<FixtureRow>): FixtureRow {
  return {
    id: "id",
    name: "Holder 1",
    kind: "holder",
    active: true,
    onBed: true,
    visible: true,
    locked: false,
    lockedBySet: false,
    line2: "Holder",
    grid: null,
    round: null,
    slots: [],
    canAddSlot: true,
    ...patch,
  };
}

test("a set's add bar has six kinds in the order of the notes, Grid hidden while the set has one", () => {
  assert.deepEqual(SET_ADD_KINDS, ["grid", "holder", "clamp", "stop", "screw", "pin"]);
  const [first, second] = view("two_sets_one_off").sets;
  assert.deepEqual(setAddKinds(first), ["holder", "clamp", "stop", "screw", "pin"]);
  assert.deepEqual(setAddKinds(second), ["grid", "holder", "clamp", "stop", "screw", "pin"]);
});

test("the job fixtures' add bar has five kinds, Pin first", () => {
  assert.deepEqual(JOB_ADD_KINDS, ["pin", "screw", "holder", "clamp", "stop"]);
  assert.equal(KIND_WORDS.pin, "Pin");
});

test("the accessible name says the kind and whether the fixture is on the bed (its set counts)", () => {
  assert.equal(rowName(row({ name: "Holder 1" })), "Holder 1, Holder, on the bed");
  assert.equal(rowName(row({ name: "Clamp 1", kind: "clamp", active: false, onBed: false })), "Clamp 1, Clamp, not on the bed");
  assert.equal(rowName(row({ name: "Pin 1", kind: "pin", active: true, onBed: false })), "Pin 1, Pin, not on the bed");
  assert.equal(rowName(row({ name: "Pocket 1", kind: "unknown" })), "Pocket 1, unknown fixture");
});

test("a row key is the id, so a rename or a reorder keeps the same React key", () => {
  assert.equal(rowKey(row({ id: "abc" })), "abc");
});

test("the slot count reads '2 of 50' and the limit is 50", () => {
  assert.equal(MAX_SLOTS, 50);
  assert.equal(slotCountText(2), "2 of 50");
  assert.equal(slotCountText(0), "0 of 50");
});

test("a new slot takes the lowest free number, so a removed name comes back", () => {
  assert.equal(slotName([]), "Slot 1");
  assert.equal(slotName(["Slot 1", "Slot 3"]), "Slot 2");
  assert.equal(slotName(["Slot 1", "Slot 2"]), "Slot 3");
});

test("the fit words name the slot and say 'turned' only when the item is turned", () => {
  assert.equal(fitWord("Slot 2", false), "Fits Slot 2");
  assert.equal(fitWord("Slot 2", true), "Fits Slot 2, turned");
});

test("the arrow, Home, End and Page keys move the focus without wrap, five rows at a time", () => {
  assert.deepEqual(rowKeyAction("ArrowDown", 0, 3, false), { kind: "focus", index: 1 });
  assert.deepEqual(rowKeyAction("ArrowDown", 2, 3, false), { kind: "focus", index: 2 });
  assert.deepEqual(rowKeyAction("ArrowUp", 0, 3, false), { kind: "focus", index: 0 });
  assert.deepEqual(rowKeyAction("Home", 2, 3, false), { kind: "focus", index: 0 });
  assert.deepEqual(rowKeyAction("End", 0, 3, false), { kind: "focus", index: 2 });
  assert.deepEqual(rowKeyAction("PageDown", 1, 9, false), { kind: "focus", index: 6 });
  assert.deepEqual(rowKeyAction("PageUp", 1, 9, false), { kind: "focus", index: 0 });
});

test("F2 renames; Delete arms, and Delete or Enter on the armed row removes", () => {
  assert.deepEqual(rowKeyAction("F2", 0, 1, false), { kind: "rename" });
  assert.deepEqual(rowKeyAction("Delete", 0, 1, false), { kind: "arm" });
  assert.deepEqual(rowKeyAction("Delete", 0, 1, true), { kind: "remove" });
  assert.deepEqual(rowKeyAction("Enter", 0, 1, true), { kind: "remove" });
  assert.equal(rowKeyAction("Enter", 0, 1, false), null);
  assert.equal(rowKeyAction("x", 0, 1, false), null);
  assert.equal(rowKeyAction("ArrowDown", 0, 0, false), null, "an empty list has no focus move");
});

test("the one tab stop is the focused row, else the selected one, else the first", () => {
  assert.equal(tabStopRow(["a", "b"], "b", "a"), "a");
  assert.equal(tabStopRow(["a", "b"], "b", null), "b");
  assert.equal(tabStopRow(["a", "b"], null, "gone"), "a");
  assert.equal(tabStopRow([], null, null), null);
});

test("a fit cue decodes from the session's JSON; anything else is null", () => {
  assert.deepEqual(decodeFitCue('{"slot":"s1","name":"Slot 2","turned":true}'), {
    slot: "s1",
    name: "Slot 2",
    turned: true,
  });
  assert.equal(decodeFitCue("null"), null);
  assert.equal(decodeFitCue('{"slot":"s1","name":"Slot 2"}'), null);
  assert.equal(decodeFitCue("not json"), null);
});

test("Up and Down step a value by one unit, Shift by ten, with no wrap", () => {
  assert.equal(stepValue(30, "up", false), 31);
  assert.equal(stepValue(30, "down", false), 29);
  assert.equal(stepValue(30, "up", true), 40);
  assert.equal(stepValue(30, "down", true), 20);
  assert.equal(stepValue(-5, "down", false), -6, "an angle may be negative");
});
