// Run with `npm test`. Independent check of the sentences quoted verbatim in
// `specs/0034-pen-path-extension` (criteria 1, 7, 16, 20) and `specs/0035-combine-and-break-apart`
// (criteria 2, 7, 9, 10, 10a, 11), written from the specifications. The Break apart sentences of
// `0035` criteria 16 and 17 are the Split sentences of `specs/0048-split-compound-path` since the
// customer's decision of 2026-10-10.

import assert from "node:assert/strict";
import { test } from "node:test";

import {
  combineRefusalText,
  combineWarning,
  commandNote,
  noticeMs,
  splitRefusalText,
} from "../src/lib/pathText.ts";
import {
  CHIP_REST_MS,
  chipLines,
  closeNoticeText,
  closeNoticeMs,
  type PenCue,
} from "../src/lib/penText.ts";

const cue = (patch: Partial<PenCue>): PenCue => ({
  kind: "none",
  target: "t",
  join: "",
  asDrawn: "",
  shift: false,
  styleDiffers: false,
  ...patch,
});

test("0034 criterion 1: continue chip, with and without Shift", () => {
  assert.deepEqual(chipLines(cue({ kind: "continue" })), ["Continue path", "Shift: start a new path"]);
  assert.deepEqual(chipLines(cue({ kind: "new_path", shift: true })), [
    "Start a new path",
    "Release Shift: continue path",
  ]);
  assert.equal(CHIP_REST_MS, 600);
});

test("0034 criterion 7: join chip, the muted style line, and the Shift variant", () => {
  assert.deepEqual(chipLines(cue({ kind: "join" })), ["Join with path", "Shift: place a node"]);
  assert.deepEqual(chipLines(cue({ kind: "join", styleDiffers: true })), [
    "Join with path",
    "Shift: place a node",
    "The result keeps the style of the path you continue.",
  ]);
  assert.deepEqual(chipLines(cue({ kind: "place_over_end", shift: true })), [
    "Place a node",
    "Release Shift: join with path",
  ]);
});

test("0034 criterion 16: the four close chips", () => {
  assert.deepEqual(chipLines(cue({ kind: "close", join: "sharp", asDrawn: "sharp" })), [
    "Close path with a sharp corner",
    "Shift: smooth curve",
  ]);
  assert.deepEqual(chipLines(cue({ kind: "close", join: "smooth", asDrawn: "smooth" })), [
    "Close path with a smooth curve",
    "Shift: sharp corner",
  ]);
  assert.deepEqual(chipLines(cue({ kind: "close", join: "smooth", asDrawn: "sharp", shift: true })), [
    "Close path with a smooth curve",
    "Release Shift: sharp corner",
  ]);
  assert.deepEqual(chipLines(cue({ kind: "close", join: "sharp", asDrawn: "smooth", shift: true })), [
    "Close path with a sharp corner",
    "Release Shift: smooth curve",
  ]);
  assert.equal(chipLines(cue({ kind: "place" })), null);
  assert.equal(chipLines(cue({ kind: "none" })), null);
});

test("0034 criterion 20: the Close path notices and their durations", () => {
  const a = closeNoticeText({ closed: 3, skipped: 0 });
  assert.equal(a, "Closed 3 paths.");
  assert.equal(closeNoticeMs(a ?? ""), 3000);
  const b = closeNoticeText({ closed: 2, skipped: 1 });
  assert.equal(b, "Closed 2 paths. 1 path has fewer than 3 nodes and was not closed.");
  // The sentence "No undo yet." is gone (0020 criterion 52): the skipped variant has 65 characters,
  // so it keeps the 3 s; a longer sentence (above 70 characters) keeps 5 s.
  assert.equal((b ?? "").length, 65);
  assert.equal(closeNoticeMs(b ?? ""), 3000);
  assert.equal(closeNoticeMs("x".repeat(71)), 5000);
  assert.equal(closeNoticeText({ closed: 1, skipped: 0 }), "Closed 1 path.");
  assert.equal(closeNoticeText({ closed: 0, skipped: 0 }), null);
});

test("0035 criteria 9, 10, 10a withdrawn by 0054 criterion 8: those refusals have no text", () => {
  const r = (kind: string, count: number, of: number) =>
    combineRefusalText({
      kind,
      count,
      of,
      holes: 0,
      open: 0,
      touching: 0,
      touchingHoles: 0,
      singleNodesRemoved: 0,
      markers: false,
      stylesDiffer: false,
    });
  for (const kind of ["open_paths", "touching", "self_touching"]) {
    assert.equal(r(kind, 1, 3), null, kind);
  }
});

test("0035 criterion 2: tooltip notes, first match wins", () => {
  const st = (p: Partial<{ needsTwo: boolean; hasCompound: boolean; canCut: boolean }>) => ({
    needsTwo: false,
    hasCompound: true,
    canCut: true,
    ...p,
  });
  // 0054 criterion 13: the dimmed note names paths and shapes, the open-path note is gone.
  assert.equal(commandNote("combine", st({ needsTwo: true }), true), "Select two or more paths or shapes.");
  assert.equal(
    commandNote("combine", st({ needsTwo: true }), false),
    "Use the Select tool.",
  );
  assert.equal(commandNote("combine", st({}), true), "Replaces the selection.");
  assert.equal(commandNote("split", st({ hasCompound: false }), true), "Select a compound path.");
  assert.equal(
    commandNote("split", st({ hasCompound: false }), false),
    "Use the Select tool.",
  );
  assert.equal(commandNote("split", st({}), true), "Replaces the selection.");
});

// No Combine or Split success notice since the customer rule of 2026-10-31; refusals stay.
test("0035 criteria 16 and 17 as renamed by 0048: Split refusals", () => {
  const no = (kind: string, compounds: number) =>
    splitRefusalText({ kind, compounds, pieces: 0, withHoles: true, leftAlone: 0 });
  assert.equal(no("no_compound", 0), "Split needs a compound path. Nothing was changed.");
  assert.equal(
    no("one_piece", 1),
    "Nothing to split. The compound path is one piece with its holes. Use Break apart to release the holes. Nothing was changed.",
  );
  assert.equal(
    no("one_piece", 2),
    "Nothing to split. The 2 selected compound paths are one piece each, with their holes. Use Break apart to release the holes. Nothing was changed.",
  );
});

test("a Combine warns only about holes that touch or cross", () => {
  const base = {
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
  };
  assert.equal(combineWarning(base), null);
  assert.equal(combineWarning({ ...base, touchingHoles: 1 }), null);
  assert.equal(
    combineWarning({ ...base, touchingHoles: 2 }),
    "2 holes touch or cross; where they overlap, it stays filled. Undo and Union them first for one hole.",
  );
  assert.equal(combineWarning({ ...base, touchingHoles: 2, kind: "out_of_range" }), null);
});

test("a notice over 70 characters lasts 5 seconds", () => {
  assert.equal(noticeMs("x".repeat(70)), undefined);
  assert.equal(noticeMs("x".repeat(71)), 5000);
});
