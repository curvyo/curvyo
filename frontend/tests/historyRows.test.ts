import assert from "node:assert/strict";
import test from "node:test";

import {
  type HistoryRowData,
  ROW_HEIGHT_PX,
  formatFullTime,
  formatRowTime,
  nextFocus,
  pagesFor,
  parseRows,
  rowAriaName,
  rowTooltip,
  scrollAfterInsert,
  secondLine,
  visibleRange,
} from "../src/lib/historyRows.ts";

const row = (over: Partial<HistoryRowData> = {}): HistoryRowData => ({
  id: "a1b2c3d",
  step: "7@3",
  label: "translate_objects",
  name: "Move",
  known: true,
  author: "you",
  time: new Date(2026, 9, 10, 12, 3, 41).getTime() / 1000,
  kind: "change",
  summary: "3 paths",
  status: "applied",
  objects: [],
  ...over,
});

const today = new Date(2026, 9, 10, 15, 0, 0);

test("a time is the clock time today, the date and clock before, and unknown for zero", () => {
  assert.equal(formatRowTime(row().time, today), "12:03");
  const yesterday = new Date(2026, 9, 9, 12, 3).getTime() / 1000;
  assert.equal(formatRowTime(yesterday, today), "9 Oct 12:03");
  assert.equal(formatRowTime(0, today), "Unknown time");
  assert.equal(formatFullTime(row().time), "10 Oct 2026 12:03:41");
});

test("the second line holds status, summary, author and time, in that order", () => {
  assert.equal(secondLine(row(), today), "3 paths · You · 12:03");
  assert.equal(secondLine(row({ status: "undone" }), today), "Undone · 3 paths · You · 12:03");
  assert.equal(
    secondLine(row({ summary: "", author: "earlier", time: 0 }), today),
    "Earlier · Unknown time",
  );
});

test("the tooltip and the accessible name say what the row says", () => {
  assert.equal(rowTooltip(row()), "Move, 3 paths. You, 10 Oct 2026 12:03:41. Id a1b2c3d.");
  assert.equal(
    rowTooltip(row({ known: false, name: "Edit", label: "old_label", author: "earlier" })),
    "Edit, 3 paths. Earlier session, 10 Oct 2026 12:03:41. Id a1b2c3d. Label: old_label.",
  );
  assert.equal(
    rowAriaName(row({ status: "undone" }), today),
    "Move, 3 paths, You, 12:03, id a1b2c3d, undone",
  );
});

test("rows parse from the session's JSON", () => {
  const rows = parseRows(JSON.stringify([row()]));
  assert.equal(rows.length, 1);
  assert.equal(rows[0].name, "Move");
  assert.deepEqual(parseRows("{}"), []);
});

test("only the rows in view, and a few more, are drawn", () => {
  assert.deepEqual(visibleRange(0, 440, 10_000), { first: 0, last: 15 });
  assert.deepEqual(visibleRange(ROW_HEIGHT_PX * 100, 440, 10_000), { first: 95, last: 115 });
  assert.deepEqual(visibleRange(0, 440, 0), { first: 0, last: -1 });
  assert.deepEqual(visibleRange(1e9, 440, 20), { first: 5, last: 19 });
});

test("rows are requested in pages of fifty", () => {
  assert.deepEqual(pagesFor(0, 15), [0]);
  assert.deepEqual(pagesFor(45, 105), [0, 1, 2]);
  assert.deepEqual(pagesFor(3, 2), []);
});

test("the roving focus moves by arrows, Home, End and pages, and stays inside", () => {
  assert.equal(nextFocus("ArrowDown", 0, 5, 10), 1);
  assert.equal(nextFocus("ArrowDown", 4, 5, 10), 4);
  assert.equal(nextFocus("ArrowUp", 0, 5, 10), 0);
  assert.equal(nextFocus("Home", 3, 5, 10), 0);
  assert.equal(nextFocus("End", 0, 5, 10), 4);
  assert.equal(nextFocus("PageDown", 0, 100, 10), 10);
  assert.equal(nextFocus("PageUp", 5, 100, 10), 0);
  assert.equal(nextFocus("a", 0, 5, 10), null);
  assert.equal(nextFocus("ArrowDown", 0, 0, 10), null);
});

test("a list scrolled away keeps its place when a row arrives at the top", () => {
  assert.equal(scrollAfterInsert(0, 1), 0);
  assert.equal(scrollAfterInsert(440, 1), 440 + ROW_HEIGHT_PX);
  assert.equal(scrollAfterInsert(440, 3), 440 + 3 * ROW_HEIGHT_PX);
});

test("the wipe confirm and its notice use the specified words", async () => {
  const { wipeQuestion, wipedNotice, WIPE_ARM_MS } = await import("../src/lib/historyRows.ts");
  assert.equal(
    wipeQuestion(142),
    "Remove all 142 steps from this file? The drawing stays as it is. This cannot be undone.",
  );
  assert.equal(
    wipeQuestion(1),
    "Remove the 1 step from this file? The drawing stays as it is. This cannot be undone.",
  );
  assert.equal(wipedNotice(142), "History wiped. 142 steps removed.");
  assert.equal(wipedNotice(1), "History wiped. 1 step removed.");
  assert.equal(WIPE_ARM_MS, 400);
});
