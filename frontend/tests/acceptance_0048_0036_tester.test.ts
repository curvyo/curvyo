// Run with `npm test`. Independent check of the Path card's state logic and of the sentences
// quoted in `specs/0048-split-compound-path` (criteria 1, 2, 7 to 9, 12, 13, the States table)
// and `specs/0036-cut-at-crossings` (criteria 1, 7 to 10, the States table), written from the
// specifications.

import assert from "node:assert/strict";
import { test } from "node:test";

import {
  PATH_COMMANDS,
  breakApartRefusalText,
  commandDimmed,
  commandName,
  commandNote,
  commandRule,
  cutOutlinesObjects,
  cutRefusalText,
  noticeMs,
  splitRefusalText,
  type PathAvailabilityState,
} from "../src/lib/pathText.ts";

const avail = (patch: Partial<PathAvailabilityState>): PathAvailabilityState => ({
  needsTwo: true,
  hasCompound: false,
  canCut: false,
  ...patch,
});
const split = (kind: string, compounds = 0, pieces = 0, withHoles = false, leftAlone = 0) => ({
  kind,
  compounds,
  pieces,
  withHoles,
  leftAlone,
});
const brk = (kind: string, compounds = 0, pieces = 0, withHoles = false) => ({
  kind,
  compounds,
  pieces,
  withHoles,
  singleNodesRemoved: 0,
});
const cut = (kind: string, count = 0, of = 0, pieces = 0) => ({ kind, count, of, pieces });

test("card order: Combine, Split, Break apart, Cut; names are the plain words", () => {
  assert.deepEqual([...PATH_COMMANDS], ["combine", "split", "break_apart", "cut"]);
  assert.deepEqual(
    PATH_COMMANDS.map(commandName),
    ["Combine", "Split", "Break apart", "Cut"],
  );
});

test("0048 c2 / 0036 c1: rule lines verbatim and at most 45 characters", () => {
  assert.equal(commandRule("split"), "Separates unconnected parts. Holes stay.");
  assert.equal(commandRule("break_apart"), "Every outline becomes its own object.");
  assert.equal(commandRule("cut"), "Cuts paths open where they cross or touch.");
  for (const c of PATH_COMMANDS) {
    assert.ok(commandRule(c).length <= 45, `${c}: ${commandRule(c).length}`);
  }
});

test("States: no compound path dims Split and Break apart, never Cut when something is selected", () => {
  const ordinary = avail({ needsTwo: true, hasCompound: false, canCut: true });
  assert.equal(commandDimmed("split", ordinary), true);
  assert.equal(commandDimmed("break_apart", ordinary), true);
  assert.equal(commandDimmed("cut", ordinary), false);
  assert.equal(commandNote("split", ordinary, true), "Select a compound path.");
  assert.equal(commandNote("break_apart", ordinary, true), "Select a compound path.");
  assert.equal(commandNote("cut", ordinary, true), "Pieces get no fill. Replaces the selection.");
});

test("States: a compound path enables Split and Break apart with their notes", () => {
  const comp = avail({ hasCompound: true, canCut: true });
  assert.equal(commandDimmed("split", comp), false);
  assert.equal(commandDimmed("break_apart", comp), false);
  assert.equal(commandDimmed("cut", comp), false, "pressing refuses, the button is enabled");
  assert.equal(commandNote("split", comp, true), "Replaces the selection.");
  assert.equal(
    commandNote("break_apart", comp, true),
    "Holes become filled shapes. Use Split to keep them.",
  );
});

test("States: Split and Break apart are dimmed in exactly the same cases", () => {
  for (const hasCompound of [false, true]) {
    for (const canCut of [false, true]) {
      for (const needsTwo of [false, true]) {
        const a = avail({ hasCompound, canCut, needsTwo });
        assert.equal(commandDimmed("split", a), commandDimmed("break_apart", a));
      }
    }
  }
});

test("States: nothing selected dims Cut with its note", () => {
  const none = avail({});
  assert.equal(commandDimmed("cut", none), true);
  assert.equal(commandNote("cut", none, true), "Select one or more paths.");
});

// Amended after the UX review of 2026-10-12 (lead decision): with another tool the note is "Use the
// Select tool." alone; the session reports nothing as selected then, so a selection sentence would
// be false.
test("States: another tool than Select says 'Use the Select tool.'", () => {
  const off = avail({}); // the session reports nothing for another tool
  assert.equal(commandNote("split", off, false), "Use the Select tool.");
  assert.equal(
    commandNote("break_apart", off, false),
    "Use the Select tool.",
  );
  assert.equal(commandNote("cut", off, false), "Use the Select tool.");
  for (const c of ["split", "break_apart", "cut"] as const) {
    assert.equal(commandDimmed(c, off), true);
  }
});

test("0048 c7, c9: Split refusals", () => {
  // A compound path with a hole (0054 criterion 17 keeps this wording for it).
  assert.equal(
    splitRefusalText(split("one_piece", 1, 0, true)),
    "Nothing to split. The compound path is one piece with its holes. Use Break apart to " +
      "release the holes. Nothing was changed.",
  );
  assert.equal(
    splitRefusalText(split("one_piece", 2, 0, true)),
    "Nothing to split. The 2 selected compound paths are one piece each, with their holes. " +
      "Use Break apart to release the holes. Nothing was changed.",
  );
  assert.equal(
    splitRefusalText(split("no_compound")),
    "Split needs a compound path. Nothing was changed.",
  );
  assert.equal(
    splitRefusalText(split("groups", 3)),
    "Split does not work on groups. Ungroup first. Nothing was changed.",
  );
  assert.equal(splitRefusalText(split("applied", 1, 2)), null);
  assert.equal(splitRefusalText(split("ignored")), null);
});

// Success notices of Split, Break apart and Cut are gone (customer rule of 2026-10-31); their
// refusals stay.
test("0048 c13: Break apart refusals", () => {
  assert.equal(
    breakApartRefusalText(brk("no_compound")),
    "Break apart needs a compound path. Nothing was changed.",
  );
  assert.equal(
    breakApartRefusalText(brk("groups", 2)),
    "Break apart does not work on groups. Ungroup first. Nothing was changed.",
  );
  assert.equal(breakApartRefusalText(brk("applied", 1, 2)), null);
});

test("0036 c8 to c10: Cut refusals", () => {
  assert.equal(
    cutRefusalText(cut("no_crossing")),
    "The selected paths do not cross each other or themselves. Nothing was changed.",
  );
  assert.equal(
    cutRefusalText(cut("compound", 1, 2)),
    "Cut does not work on compound paths. Split or Break apart first. Nothing was changed.",
  );
  assert.ok(
    (cutRefusalText(cut("groups", 2, 3)) ?? "").startsWith(
      "Cut does not work on groups. Ungroup first.",
    ),
  );
  assert.equal(
    cutRefusalText(cut("too_many", 0, 0, 10003)),
    "Cut would make more than 10000 pieces. Nothing was changed.",
  );
  assert.ok((cutRefusalText(cut("out_of_range", 1, 3)) ?? "").includes("Nothing was changed."));
  assert.equal(cutRefusalText(cut("applied", 2, 0, 4)), null);
  assert.equal(cutOutlinesObjects(cut("compound", 1, 2)), true);
  assert.equal(cutOutlinesObjects(cut("groups", 2, 3)), true);
  assert.equal(cutOutlinesObjects(cut("no_crossing")), false);
  assert.equal(cutOutlinesObjects(cut("too_many", 0, 0, 10003)), false);
});
