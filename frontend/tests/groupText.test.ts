// Run with `npm test`. The words of `src/lib/groupText.ts` against `specs/0023-groups` (UX notes
// "Group and Ungroup" and "Entered group", criterion 28) and the refusal sentence of the
// operations that do not look through groups.

import assert from "node:assert/strict";
import { test } from "node:test";

import { refusalText, refusalOutlinesObjects } from "../src/lib/booleanText.ts";
import { breakApartRefusalText, combineRefusalText } from "../src/lib/pathText.ts";
import {
  collapsedName,
  crumbRow,
  groupChord,
  groupCommandDimmed,
  groupCommandName,
  groupCommandNote,
  groupCommandRule,
} from "../src/lib/groupText.ts";

const none = { group: false, ungroup: false };
const both = { group: true, ungroup: true };

test("the tooltip names the chord, with Cmd on macOS", () => {
  assert.equal(groupCommandName("group", false), "Group (Ctrl+G)");
  assert.equal(groupCommandName("ungroup", false), "Ungroup (Ctrl+Shift+G)");
  assert.equal(groupCommandName("group", true), "Group (Cmd+G)");
  assert.equal(groupChord("ungroup", true), "Cmd+Shift+G");
});

test("the rule and the note of an enabled button", () => {
  assert.equal(groupCommandRule("group"), "Binds the selection into one group.");
  assert.equal(groupCommandRule("ungroup"), "Releases the objects of a group.");
  assert.equal(groupCommandNote("group", both, true), "Objects in between move below the group.");
  assert.equal(
    groupCommandNote("ungroup", both, true),
    "One level only. Inner groups stay groups.",
  );
});

test("a dimmed button says why, and which tool to take", () => {
  assert.equal(groupCommandDimmed("group", none), true);
  assert.equal(groupCommandNote("group", none, true), "Select two or more objects.");
  assert.equal(
    groupCommandNote("group", none, false),
    "Select two or more objects. Use the Select tool.",
  );
  assert.equal(groupCommandNote("ungroup", none, true), "Select a group.");
  assert.equal(groupCommandNote("ungroup", none, false), "Select a group. Use the Select tool.");
  assert.equal(groupCommandDimmed("ungroup", { group: true, ungroup: false }), true);
});

test("the crumbs are Document and one Group per level, the last one current", () => {
  const row = crumbRow(2);
  assert.deepEqual(
    row.crumbs.map((c) => [c.label, c.level, c.current]),
    [
      ["Document", 0, false],
      ["Group", 1, false],
      ["Group", 2, true],
    ],
  );
  assert.equal(row.collapsed, 0);
});

test("more than three levels collapse the middle and keep Document", () => {
  const row = crumbRow(5);
  assert.deepEqual(
    row.crumbs.map((c) => c.level),
    [0, 4, 5],
  );
  assert.equal(row.collapsed, 3);
  assert.equal(row.collapsedAt, 1);
  assert.equal(collapsedName(4), "4 more levels");
  assert.equal(collapsedName(1), "1 more level");
});

test("the operations refuse a group in the style of the other refusals (criterion 28)", () => {
  const result = { kind: "groups", count: 2, of: 2, compound: false };
  assert.equal(
    refusalText("union", result),
    "Union does not work on groups. Ungroup first. Nothing was changed.",
  );
  assert.equal(refusalOutlinesObjects(result), true);
  assert.equal(
    combineRefusalText({ kind: "groups", count: 2, of: 2, holes: 0, stylesDiffer: false }),
    "Combine does not work on groups. Ungroup first. Nothing was changed.",
  );
  assert.equal(
    breakApartRefusalText({
      kind: "groups",
      compounds: 0,
      pieces: 0,
      withHoles: false,
      leftAlone: 0,
    }),
    "Break apart does not work on groups. Ungroup first. Nothing was changed.",
  );
});
