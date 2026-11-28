// Run with `npm test`. Texts of specs/0055-compound-path-node-editing criteria 5, 6, 7 and the
// removed Node bar pill (criterion 1), written from the specification.

import assert from "node:assert/strict";
import { test } from "node:test";

import { joinNoticeText, splitNoticeText } from "../src/lib/nodeText.ts";
import { closeDimmed, closeNoticeText, closeTooltipNote } from "../src/lib/penText.ts";
import { readFileSync } from "node:fs";

test("c5c: join notice counts outlines in digits, singular for one", () => {
  // 0164 S7 (customer 2026-11-27): result notices carry no counts of what came out.
  assert.equal(joinNoticeText(2), "Join: the other outlines moved into this compound path.");
  assert.equal(joinNoticeText(1), "Join: the other outlines moved into this compound path.");
  assert.equal(joinNoticeText(0), null);
});

test("c6: split notices", () => {
  assert.equal(
    splitNoticeText("outline_in_two"),
    "Split: 1 outline became 2. They stay in this compound path.",
  );
  assert.equal(splitNoticeText("outline_opened"), "Split: 1 closed outline was opened.");
  assert.equal(splitNoticeText("path_in_two"), null);
});

test("c7: close notice counts paths and outlines apart", () => {
  // 0164 S7 (customer 2026-11-27): result notices carry no counts of what came out.
  assert.equal(closeNoticeText({ closed: 1, skipped: 0, closedOutlines: 3, skippedOutlines: 0 }), null);
  assert.equal(closeNoticeText({ closed: 0, skipped: 0, closedOutlines: 3, skippedOutlines: 0 }), null);
  const skipped = closeNoticeText({ closed: 1, skipped: 0, closedOutlines: 3, skippedOutlines: 1 });
  assert.equal(skipped, "1 outline has fewer than 3 nodes and was not closed.");
  assert.equal(closeNoticeText({ closed: 0, skipped: 2 }), null);
});

test("c7: buttons are dimmed only without any closable path or outline", () => {
  assert.equal(closeDimmed({ closable: 0, skipped: 1 }), true);
  assert.equal(closeDimmed({ closable: 0, skipped: 0, closableOutlines: 1 }), false);
  assert.equal(closeDimmed({ closable: 1, skipped: 0 }), false);
  assert.equal(
    closeTooltipNote({ closable: 0, skipped: 0 }),
    "Select an open path or outline with 3 or more nodes.",
  );
  assert.equal(
    closeTooltipNote({ closable: 0, skipped: 0, closableOutlines: 3 }),
    "Closes 3 outlines.",
  );
  assert.equal(
    closeTooltipNote({ closable: 1, skipped: 0, closableOutlines: 3 }),
    "Closes 1 path and 3 outlines.",
  );
});

test("c1: the 'cannot be edited yet' pill is gone from the frontend sources", () => {
  for (const file of ["../src/components/NodeToolbar.tsx", "../src/lib/nodeText.ts", "../src/lib/booleanText.ts"]) {
    const text = readFileSync(new URL(file, import.meta.url), "utf8");
    assert.ok(!text.includes("cannot be edited yet"), file);
  }
});
