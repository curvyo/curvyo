// Run with `npm test`. The Join, Split and Close path sentences on compound paths against the
// wording of `specs/0055-compound-path-node-editing/specification.md` (criteria 5 to 7 and the UX
// notes "Node bar, notices and texts").

import assert from "node:assert/strict";
import { test } from "node:test";

import { joinNoticeText, splitNoticeText } from "../src/lib/nodeText.ts";
import { closeDimmed, closeNoticeText, closeTooltipNote } from "../src/lib/penText.ts";

test("criterion 5 c: the Join notice counts the moved outlines", () => {
  assert.equal(joinNoticeText(2), "Join: 2 more outlines moved into this compound path.");
  assert.equal(joinNoticeText(1), "Join: 1 more outline moved into this compound path.");
  assert.equal(joinNoticeText(0), null);
});

test("criterion 6: the Split notice of a compound path", () => {
  assert.equal(
    splitNoticeText("outline_in_two"),
    "Split: 1 outline became 2. They stay in this compound path.",
  );
  assert.equal(splitNoticeText("outline_opened"), "Split: 1 closed outline was opened.");
  assert.equal(splitNoticeText("path"), null);
  assert.equal(splitNoticeText(""), null);
});

test("criterion 7: the Close path notice counts paths and outlines apart", () => {
  assert.equal(
    closeNoticeText({ closed: 1, skipped: 0, closedOutlines: 3 }),
    "Closed 1 path and 3 outlines.",
  );
  assert.equal(closeNoticeText({ closed: 0, skipped: 0, closedOutlines: 3 }), "Closed 3 outlines.");
  assert.equal(
    closeNoticeText({ closed: 0, skipped: 0, closedOutlines: 3, skippedOutlines: 1 }),
    "Closed 3 outlines. 1 outline has fewer than 3 nodes and was not closed.",
  );
  assert.equal(closeNoticeText({ closed: 0, skipped: 0, skippedOutlines: 1 }), null);
});

test("criterion 7: the Close path notes and the dimmed state", () => {
  const none = { closable: 0, skipped: 0, closableOutlines: 0, skippedOutlines: 2 };
  assert.equal(closeDimmed(none), true);
  assert.equal(closeTooltipNote(none), "Select an open path or outline with 3 or more nodes.");
  assert.equal(
    closeTooltipNote({ closable: 0, skipped: 0, closableOutlines: 3 }),
    "Closes 3 outlines.",
  );
  assert.equal(
    closeTooltipNote({ closable: 1, skipped: 0, closableOutlines: 3 }),
    "Closes 1 path and 3 outlines.",
  );
  assert.equal(
    closeTooltipNote({ closable: 0, skipped: 0, closableOutlines: 3, skippedOutlines: 1 }),
    "Closes 3 of 4 open outlines. 1 has fewer than 3 nodes.",
  );
  assert.equal(
    closeTooltipNote({ closable: 1, skipped: 0, closableOutlines: 3, skippedOutlines: 1 }),
    "Closes 1 path and 3 outlines. 1 outline has fewer than 3 nodes.",
  );
  assert.equal(closeDimmed({ closable: 0, skipped: 0, closableOutlines: 1 }), false);
});
