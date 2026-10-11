// Run with `npm test`. The sentences of `src/lib/pathText.ts` against the wording of
// `specs/0035-combine-and-break-apart/specification.md` (criteria 2, 7, 9 to 11) and
// `specs/0048-split-compound-path/specification.md` (criteria 2, 7 to 9, 12, 13).

import assert from "node:assert/strict";
import { test } from "node:test";

import {
  breakApartRefusalText,
  breakApartSuccessText,
  combineOutlinesObjects,
  combineRefusalText,
  combineSuccessText,
  commandDimmed,
  commandName,
  commandNote,
  commandRule,
  cutOutlinesObjects,
  cutRefusalText,
  cutSuccessText,
  noticeMs,
  PATH_COMMANDS,
  splitRefusalText,
  splitSuccessText,
  type BreakApartResult,
  type CombineResult,
  type CutResult,
  type PathAvailabilityState,
  type SplitResult,
} from "../src/lib/pathText.ts";

const available = (patch: Partial<PathAvailabilityState>): PathAvailabilityState => ({
  needsTwo: false,
  open: 0,
  of: 3,
  hasCompound: true,
  canCut: true,
  ...patch,
});

const combined = (patch: Partial<CombineResult>): CombineResult => ({
  kind: "applied",
  count: 4,
  of: 4,
  holes: 3,
  stylesDiffer: false,
  ...patch,
});

const split = (patch: Partial<SplitResult>): SplitResult => ({
  kind: "applied",
  compounds: 1,
  pieces: 3,
  withHoles: false,
  leftAlone: 0,
  ...patch,
});

const broken = (patch: Partial<BreakApartResult>): BreakApartResult => ({
  kind: "applied",
  compounds: 1,
  pieces: 3,
  withHoles: false,
  ...patch,
});

const cut = (patch: Partial<CutResult>): CutResult => ({
  kind: "applied",
  count: 2,
  of: 2,
  pieces: 9,
  ...patch,
});

test("names and rules are those of 0048 criterion 2, and no rule wraps", () => {
  assert.deepEqual(PATH_COMMANDS.map(commandName), ["Combine", "Split", "Break apart", "Cut"]);
  assert.equal(commandRule("cut"), "Cuts paths open where they cross or touch.");
  assert.equal(commandRule("combine"), "One object. Inner shapes become holes.");
  assert.equal(commandRule("split"), "Separates unconnected parts. Holes stay.");
  assert.equal(commandRule("break_apart"), "Every outline becomes its own object.");
  for (const command of PATH_COMMANDS) {
    assert.ok(commandRule(command).length <= 45, command);
  }
});

test("the Combine note: fewer than two, another tool, an open path, else the effect", () => {
  const none = available({ needsTwo: true, of: 0 });
  assert.equal(commandNote("combine", none, true), "Select two or more closed shapes.");
  assert.equal(
    commandNote("combine", none, false),
    "Select two or more closed shapes. Use the Select tool.",
  );
  assert.equal(
    commandNote("combine", available({ open: 1 }), true),
    "Needs closed paths: 1 of 3 selected is open.",
  );
  assert.equal(
    commandNote("combine", available({ open: 2 }), true),
    "Needs closed paths: 2 of 3 selected are open.",
  );
  assert.equal(
    commandNote("combine", available({}), true),
    "Replaces the selection.",
  );
});

test("the Split and Break apart notes: no compound path, another tool, else the effect", () => {
  const none = available({ hasCompound: false });
  for (const command of ["split", "break_apart"] as const) {
    assert.equal(commandNote(command, none, true), "Select a compound path.");
    assert.equal(
      commandNote(command, none, false),
      "Select a compound path. Use the Select tool.",
    );
  }
  assert.equal(commandNote("split", available({}), true), "Replaces the selection.");
  assert.equal(
    commandNote("break_apart", available({}), true),
    "Holes become filled shapes. Use Split to keep them.",
  );
});

test("dimming follows the session: an open path or touching shapes do not dim Combine", () => {
  assert.equal(commandDimmed("combine", available({ needsTwo: true })), true);
  assert.equal(commandDimmed("combine", available({ open: 1 })), false);
  for (const command of ["split", "break_apart"] as const) {
    assert.equal(commandDimmed(command, available({ hasCompound: false })), true, command);
    assert.equal(commandDimmed(command, available({})), false, command);
  }
});

test("the Combine notice counts the objects and the holes", () => {
  assert.equal(
    combineSuccessText(combined({})),
    "Combine: 4 objects became 1 compound path with 3 holes.",
  );
  assert.equal(
    combineSuccessText(combined({ holes: 1 })),
    "Combine: 4 objects became 1 compound path with 1 hole.",
  );
  assert.equal(
    combineSuccessText(combined({ holes: 0, count: 2 })),
    "Combine: 2 objects became 1 compound path.",
  );
  assert.equal(combineSuccessText(combined({ kind: "touching" })), null);
});

test("the Combine notice names the style that was used when the styles differed", () => {
  assert.equal(
    combineSuccessText(combined({ stylesDiffer: true })),
    "Combine: 4 objects became 1 compound path with 3 holes. It uses the style of the lowest object.",
  );
});

test("a notice over 70 characters lasts 5 seconds, a shorter one the default 3", () => {
  assert.equal(noticeMs(combineSuccessText(combined({ holes: 0, count: 2 })) ?? ""), undefined);
  assert.equal(noticeMs(combineSuccessText(combined({ stylesDiffer: true })) ?? ""), 5000);
  assert.equal(noticeMs("x".repeat(70)), undefined);
  assert.equal(noticeMs("x".repeat(71)), 5000);
});

test("the Combine refusals are the sentences of criteria 9 to 11", () => {
  assert.equal(
    combineRefusalText(combined({ kind: "open_paths", count: 1, of: 3 })),
    "Combine needs closed paths. 1 of 3 selected objects is open. Nothing was changed.",
  );
  assert.equal(
    combineRefusalText(combined({ kind: "touching", count: 2, of: 3 })),
    "Combine needs shapes that do not touch. 2 of 3 selected objects touch each other. Use Union to merge overlapping shapes. Nothing was changed.",
  );
  assert.equal(
    combineRefusalText(combined({ kind: "self_touching", count: 1, of: 3 })),
    "Combine needs shapes that do not cross themselves. 1 of 3 selected objects does. Nothing was changed.",
  );
  assert.equal(
    combineRefusalText(combined({ kind: "self_touching", count: 2, of: 3 })),
    "Combine needs shapes that do not cross themselves. 2 of 3 selected objects do. Nothing was changed.",
  );
  assert.equal(
    combineRefusalText(combined({ kind: "no_area", count: 1, of: 2 })),
    "Combine needs shapes that enclose an area. 1 of 2 selected objects has no area. Nothing was changed.",
  );
  assert.equal(
    combineRefusalText(combined({ kind: "out_of_range", count: 2, of: 3 })),
    "Combine works only within 10 km of the point 0, 0. 2 of 3 selected objects reach further. Nothing was changed.",
  );
  assert.equal(combineRefusalText(combined({ kind: "applied" })), null);
  assert.equal(combineRefusalText(combined({ kind: "needs_two" })), null);
});

test("every refusal of Combine outlines its offenders, a success none", () => {
  for (const kind of ["open_paths", "no_area", "out_of_range", "touching", "self_touching"]) {
    assert.equal(combineOutlinesObjects(combined({ kind })), true, kind);
  }
  assert.equal(combineOutlinesObjects(combined({ kind: "applied" })), false);
  assert.equal(combineOutlinesObjects(combined({ kind: "needs_two" })), false);
});

test("the Split notice counts the pieces and mentions holes and compounds left alone", () => {
  assert.equal(splitSuccessText(split({})), "Split: 1 compound path became 3 objects.");
  assert.equal(
    splitSuccessText(split({ compounds: 2, pieces: 5 })),
    "Split: 2 compound paths became 5 objects.",
  );
  assert.equal(
    splitSuccessText(split({ withHoles: true })),
    "Split: 1 compound path became 3 objects. Holes stayed with their piece.",
  );
  assert.equal(
    splitSuccessText(split({ leftAlone: 1 })),
    "Split: 1 compound path became 3 objects. 1 compound path is one piece and was left as it is.",
  );
  assert.equal(
    splitSuccessText(split({ withHoles: true, leftAlone: 2 })),
    "Split: 1 compound path became 3 objects. Holes stayed with their piece. 2 compound paths are one piece each and were left as they are.",
  );
  assert.equal(splitSuccessText(split({ kind: "one_piece" })), null);
});

test("the Split refusals are the sentences of criteria 7 and 9", () => {
  assert.equal(
    splitRefusalText(split({ kind: "no_compound" })),
    "Split needs a compound path. Nothing was changed.",
  );
  assert.equal(
    splitRefusalText(split({ kind: "groups" })),
    "Split does not work on groups. Ungroup first. Nothing was changed.",
  );
  assert.equal(
    splitRefusalText(split({ kind: "one_piece", compounds: 1 })),
    "Nothing to split. The compound path is one piece with its holes. Use Break apart to release the holes. Nothing was changed.",
  );
  assert.equal(
    splitRefusalText(split({ kind: "one_piece", compounds: 2 })),
    "Nothing to split. The 2 selected compound paths are one piece each, with their holes. Use Break apart to release the holes. Nothing was changed.",
  );
  assert.equal(splitRefusalText(split({ kind: "applied" })), null);
});

test("the Break apart notice counts the objects and warns when holes became shapes", () => {
  assert.equal(
    breakApartSuccessText(broken({})),
    "Break apart: 1 compound path became 3 objects.",
  );
  assert.equal(
    breakApartSuccessText(broken({ compounds: 2, pieces: 5 })),
    "Break apart: 2 compound paths became 5 objects.",
  );
  assert.equal(
    breakApartSuccessText(broken({ withHoles: true })),
    "Break apart: 1 compound path became 3 objects. Holes became filled shapes.",
  );
  assert.equal(breakApartSuccessText(broken({ kind: "no_compound" })), null);
});

test("the Break apart refusals are the sentences of criterion 13", () => {
  assert.equal(
    breakApartRefusalText(broken({ kind: "no_compound" })),
    "Break apart needs a compound path. Nothing was changed.",
  );
  assert.equal(
    breakApartRefusalText(broken({ kind: "groups" })),
    "Break apart does not work on groups. Ungroup first. Nothing was changed.",
  );
  assert.equal(breakApartRefusalText(broken({ kind: "applied" })), null);
});

test("the Cut note: nothing selected, another tool, else the effect", () => {
  const none = available({ canCut: false });
  assert.equal(commandNote("cut", none, true), "Select one or more paths.");
  assert.equal(commandNote("cut", none, false), "Select one or more paths. Use the Select tool.");
  assert.equal(commandNote("cut", available({}), true), "Pieces get no fill. Replaces the selection.");
  assert.equal(commandDimmed("cut", none), true);
  assert.equal(commandDimmed("cut", available({ hasCompound: false })), false, "plain paths enable Cut");
  assert.equal(commandDimmed("split", available({ hasCompound: false })), true);
});

test("the Cut notice counts the paths and the pieces", () => {
  assert.equal(cutSuccessText(cut({})), "Cut: 2 paths became 9 pieces.");
  assert.equal(cutSuccessText(cut({ count: 1, pieces: 4 })), "Cut: 1 path became 4 pieces.");
  assert.equal(cutSuccessText(cut({ kind: "no_crossing" })), null);
});

test("the Cut refusals are the sentences of criteria 8 to 10", () => {
  assert.equal(
    cutRefusalText(cut({ kind: "no_crossing" })),
    "The selected paths do not cross each other or themselves. Nothing was changed.",
  );
  assert.equal(
    cutRefusalText(cut({ kind: "groups", count: 3 })),
    "Cut does not work on groups. Ungroup first. Nothing was changed.",
  );
  assert.equal(
    cutRefusalText(cut({ kind: "compound", count: 1 })),
    "Cut does not work on compound paths. Split or Break apart first. Nothing was changed.",
  );
  assert.equal(
    cutRefusalText(cut({ kind: "too_many", pieces: 12000 })),
    "Cut would make more than 10000 pieces. Nothing was changed.",
  );
  assert.equal(
    cutRefusalText(cut({ kind: "out_of_range", count: 1, of: 3 })),
    "Cut works only within 10 km of the point 0, 0. 1 of 3 selected objects reaches further. Nothing was changed.",
  );
  assert.equal(cutRefusalText(cut({ kind: "applied" })), null);
});

test("a refused Cut outlines groups, compound paths and objects out of range, not the missing crossing", () => {
  for (const kind of ["groups", "compound", "out_of_range"]) {
    assert.equal(cutOutlinesObjects(cut({ kind })), true, kind);
  }
  for (const kind of ["no_crossing", "too_many", "applied"]) {
    assert.equal(cutOutlinesObjects(cut({ kind })), false, kind);
  }
});
