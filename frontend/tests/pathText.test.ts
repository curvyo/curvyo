// Run with `npm test`. The sentences of `src/lib/pathText.ts` against the wording of
// `specs/0035-combine-and-break-apart/specification.md` (criteria 2, 7, 9 to 11) and
// `specs/0048-split-compound-path/specification.md` (criteria 2, 7 to 9, 12, 13).

import assert from "node:assert/strict";
import { test } from "node:test";

import {
  breakApartRefusalText,
  combineOutlinesObjects,
  combineRefusalText,
  combineWarning,
  commandDimmed,
  commandName,
  commandNote,
  commandRule,
  cutOutlinesObjects,
  cutRefusalText,
  noticeMs,
  PATH_COMMANDS,
  splitRefusalText,
  type BreakApartResult,
  type CombineResult,
  type CutResult,
  type PathAvailabilityState,
  type SplitResult,
} from "../src/lib/pathText.ts";

const available = (patch: Partial<PathAvailabilityState>): PathAvailabilityState => ({
  needsTwo: false,
  hasCompound: true,
  canCut: true,
  ...patch,
});

const combined = (patch: Partial<CombineResult>): CombineResult => ({
  kind: "applied",
  count: 4,
  of: 4,
  holes: 3,
  open: 0,
  touching: 0,
  touchingHoles: 0,
  singleNodesRemoved: 0,
  markers: false,
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
  singleNodesRemoved: 0,
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

test("the Combine note: fewer than two, another tool, else the effect (0054 criterion 13)", () => {
  const none = available({ needsTwo: true });
  assert.equal(commandNote("combine", none, true), "Select two or more paths or shapes.");
  assert.equal(
    commandNote("combine", none, false),
    "Use the Select tool.",
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
      "Use the Select tool.",
    );
  }
  assert.equal(commandNote("split", available({}), true), "Replaces the selection.");
  assert.equal(
    commandNote("break_apart", available({}), true),
    "Holes become filled shapes. Use Split to keep them.",
  );
});

test("dimming follows the session: only fewer than two objects dim Combine", () => {
  assert.equal(commandDimmed("combine", available({ needsTwo: true })), true);
  assert.equal(commandDimmed("combine", available({})), false);
  for (const command of ["split", "break_apart"] as const) {
    assert.equal(commandDimmed(command, available({ hasCompound: false })), true, command);
    assert.equal(commandDimmed(command, available({})), false, command);
  }
});

test("the Combine refusals: range (0035 criterion 11) and single nodes (0054 criterion 10)", () => {
  assert.equal(
    combineRefusalText(combined({ kind: "single_nodes_only" })),
    "Combine needs outlines with two or more nodes. Every selected outline has a single node. Nothing was changed.",
  );
  assert.equal(
    combineRefusalText(combined({ kind: "out_of_range", count: 2, of: 3 })),
    "Combine works only within 10 km of the point 0, 0. 2 of 3 selected objects reach further. Nothing was changed.",
  );
  assert.equal(combineRefusalText(combined({ kind: "applied" })), null);
  assert.equal(combineRefusalText(combined({ kind: "needs_two" })), null);
});

test("a Combine refusal outlines objects out of range and groups only", () => {
  for (const kind of ["out_of_range", "groups"]) {
    assert.equal(combineOutlinesObjects(combined({ kind })), true, kind);
  }
  assert.equal(combineOutlinesObjects(combined({ kind: "single_nodes_only" })), false);
  assert.equal(combineOutlinesObjects(combined({ kind: "applied" })), false);
  assert.equal(combineOutlinesObjects(combined({ kind: "needs_two" })), false);
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
    splitRefusalText(split({ kind: "one_piece", compounds: 1, withHoles: true })),
    "Nothing to split. The compound path is one piece with its holes. Use Break apart to release the holes. Nothing was changed.",
  );
  assert.equal(
    splitRefusalText(split({ kind: "one_piece", compounds: 2, withHoles: true })),
    "Nothing to split. The 2 selected compound paths are one piece each, with their holes. Use Break apart to release the holes. Nothing was changed.",
  );
  // 0054 criterion 17: no hole, no word about holes.
  assert.equal(
    splitRefusalText(split({ kind: "one_piece", compounds: 1 })),
    "Nothing to split. The compound path is one piece. Use Break apart to separate every outline. Nothing was changed.",
  );
  assert.equal(
    splitRefusalText(split({ kind: "one_piece", compounds: 2 })),
    "Nothing to split. The 2 selected compound paths are one piece each. Use Break apart to separate every outline. Nothing was changed.",
  );
  assert.equal(splitRefusalText(split({ kind: "applied" })), null);
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
  assert.equal(commandNote("cut", none, false), "Use the Select tool.");
  assert.equal(commandNote("cut", available({}), true), "Pieces get no fill. Replaces the selection.");
  assert.equal(commandDimmed("cut", none), true);
  assert.equal(commandDimmed("cut", available({ hasCompound: false })), false, "plain paths enable Cut");
  assert.equal(commandDimmed("split", available({ hasCompound: false })), true);
});

test("the Cut refusals are the sentences of criteria 8 to 10", () => {
  assert.equal(
    cutRefusalText(cut({ kind: "no_crossing" })),
    "The selected paths do not cross each other or themselves. Nothing was changed.",
  );
  assert.equal(
    cutRefusalText(cut({ kind: "no_crossing", of: 1 })),
    "The selected path does not cross itself. Nothing was changed.",
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

// No success notice for Combine, Split, Break apart or Cut (customer rule of 2026-10-31); a Combine
// warns only about holes that touch or cross.
test("a Combine warns only about holes that touch or cross", () => {
  assert.equal(combineWarning(combined({})), null);
  assert.equal(combineWarning(combined({ touchingHoles: 1 })), null);
  assert.equal(
    combineWarning(combined({ touchingHoles: 2 })),
    "2 holes touch or cross; where they overlap, it stays filled. Undo and Union them first for one hole.",
  );
  assert.equal(combineWarning(combined({ touchingHoles: 2, kind: "out_of_range" })), null);
});

test("a notice over 70 characters lasts 5 seconds, over 140 8 seconds, a shorter one 3", () => {
  assert.equal(noticeMs("x".repeat(70)), undefined);
  assert.equal(noticeMs("x".repeat(71)), 5000);
  assert.equal(noticeMs("x".repeat(140)), 5000);
  assert.equal(noticeMs("x".repeat(141)), 8000);
});
