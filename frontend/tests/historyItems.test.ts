import assert from "node:assert/strict";
import test from "node:test";

import {
  HEADER_HEIGHT_PX,
  type ListShape,
  MORE_HEIGHT_PX,
  forkPath,
  gutterWidth,
  itemAtY,
  itemHeight,
  itemRange,
  itemTop,
  laneX,
  listHeight,
  objectsHeight,
  parseShape,
  withRowHeight,
} from "../src/lib/historyItems.ts";
import { type HistoryRowData, ROW_HEIGHT_PX, rowAriaName, secondLine } from "../src/lib/historyRows.ts";

// Row 0, a header at 1, rows 2 and 3, the "more" row at 4, rows 5 to 9.
const shape: ListShape = {
  total: 10,
  lanes: 1,
  special: [
    [1, "header"],
    [4, "more"],
  ],
};

test("header rows are 24 px and the more row 28 px; step rows keep 44 px", () => {
  assert.equal(itemHeight(0, shape), ROW_HEIGHT_PX);
  assert.equal(itemHeight(1, shape), HEADER_HEIGHT_PX);
  assert.equal(itemHeight(4, shape), MORE_HEIGHT_PX);
  assert.equal(itemTop(0, shape), 0);
  assert.equal(itemTop(1, shape), 44);
  assert.equal(itemTop(2, shape), 68);
  assert.equal(itemTop(5, shape), 68 + 88 + 28);
  assert.equal(listHeight(shape), 8 * 44 + 24 + 28);
});

test("the item under a y is found across the shorter items", () => {
  assert.equal(itemAtY(0, shape), 0);
  assert.equal(itemAtY(50, shape), 1);
  assert.equal(itemAtY(68, shape), 2);
  assert.equal(itemAtY(160, shape), 4);
  assert.equal(itemAtY(184, shape), 5);
  assert.equal(itemAtY(1e9, shape), 9);
  assert.deepEqual(itemRange(0, 100, shape, 0), { first: 0, last: 2 });
  assert.deepEqual(itemRange(0, 100, { total: 0, lanes: 0, special: [] }), { first: 0, last: -1 });
});

test("the gutter is 16 px plus 12 px per branch lane; the fork turns up into the branch lane", () => {
  assert.equal(gutterWidth(0), 16);
  assert.equal(gutterWidth(3), 52);
  assert.equal(laneX(0), 8);
  assert.equal(laneX(2), 32);
  assert.equal(forkPath(0, 1, 44), "M 8 22 H 16 Q 20 22 20 18 V 0");
});

test("a shape is read with defaults", () => {
  assert.deepEqual(parseShape("{}"), { total: 0, lanes: 0, special: [] });
});

test("a row on a branch says Branch first on line 2 and names its lane to a screen reader", () => {
  const row: HistoryRowData = {
    id: "a1b2c3d",
    step: "7@3",
    label: "translate_objects",
    name: "Move",
    known: true,
    author: "you",
    time: new Date(2026, 9, 10, 12, 3, 41).getTime() / 1000,
    kind: "change",
    summary: "3 paths",
    status: "branch",
    lane: "on branch from 9f8e7d6, step 2 of 2",
    objects: [],
  };
  const today = new Date(2026, 9, 10, 15, 0, 0);
  assert.equal(secondLine(row, today), "Branch · 3 paths · You · 12:03");
  assert.equal(
    rowAriaName(row, today),
    "Move, 3 paths, You, 12:03, id a1b2c3d, on branch from 9f8e7d6, step 2 of 2",
  );
});

test("an open disclosure grows its row by 28 px per object, at most 20 and a more line", () => {
  const base = { id: "a", step: "1@1", label: "x", name: "x", known: true, author: "you" as const, time: 0, kind: "change" as const, summary: "", status: "applied" as const };
  const object = { id: 1, shape: "rectangle", role: "changed" as const, alive: true };
  assert.equal(objectsHeight({ ...base, objects: [object, object] }), 56);
  assert.equal(objectsHeight({ ...base, objects: Array(25).fill(object) }), 21 * 28);
  const grown = withRowHeight(shape, 2, 100);
  assert.equal(itemHeight(2, grown), 100);
  assert.equal(itemTop(3, grown), 68 + 100);
  assert.equal(itemAtY(68 + 99, grown), 2);
});
