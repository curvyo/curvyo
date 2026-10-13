// Independent tester cases for `specs/0042-history-branches/specification.md`, page side, without
// a window: the words of criteria 14, 17 and 19, the gutter numbers of the UX notes (lane pitch 12 px,
// main lane at 8 px, gutter 16 px + 12 px per lane, headers 24 px, "more" row 28 px) and the
// virtualised height model of a list with branches (criterion 8: 50 branches and 10,000 steps).
// Focus, keys, the DOM and the look of the lanes in a real webview are NOT covered here.

import assert from "node:assert/strict";
import test from "node:test";

import {
  HEADER_HEIGHT_PX,
  LANE_PITCH_PX,
  MAIN_LANE_X,
  MORE_HEIGHT_PX,
  gutterWidth,
  itemAtY,
  itemHeight,
  itemRange,
  itemTop,
  laneX,
  listHeight,
  parseItems,
  parseShape,
  type ListShape,
  type SpecialItem,
} from "../src/lib/historyItems.ts";
import {
  modeCaption,
  parsePreview,
  previewBanner,
  previewReadout,
  sinceText,
} from "../src/lib/historyPreview.ts";

const now = new Date(2026, 9, 12, 15, 0, 0);
const at = new Date(2026, 9, 12, 12, 3, 0).getTime() / 1000;

test("c17 and c19 words are exactly those of the specification", () => {
  const object = parsePreview(
    JSON.stringify({ kind: "object", id: "a1b2c3d", name: "Resize", author: "you", time: at }),
  );
  assert.equal(previewReadout(object, now), "Preview a1b2c3d, Resize, You, 12:03");
  const other = parsePreview(
    JSON.stringify({ kind: "object", id: "a1b2c3d", name: "Resize", author: "earlier", time: at }),
  );
  assert.match(previewReadout(other, now), /^Preview a1b2c3d, Resize, Earlier session, 12:03$/);
  const doc = parsePreview(JSON.stringify({ kind: "document", id: "a1b2c3d", time: at, since: 3 }));
  assert.equal(previewBanner(doc, now), "Previewing a1b2c3d, 12:03. Editing is paused.");
  assert.equal(sinceText(doc), "(3 new steps since)");
});

test("a preview with an unknown kind or broken data reads as no preview", () => {
  assert.equal(parsePreview(JSON.stringify({ kind: "weird" })).kind, "");
  assert.equal(parsePreview("{}").kind, "");
});

test("c14 captions", () => {
  assert.equal(modeCaption(false), "A click shows the version. Nothing changes.");
  assert.equal(modeCaption(true), "A click puts the object into that version. Ctrl+Z undoes it.");
});

test("the gutter numbers of the UX notes", () => {
  assert.equal(MAIN_LANE_X, 8);
  assert.equal(LANE_PITCH_PX, 12);
  assert.equal(laneX(0), 8);
  assert.deepEqual([1, 2, 3].map(laneX), [20, 32, 44]);
  assert.equal(gutterWidth(0), 16);
  assert.equal(gutterWidth(3), 16 + 36);
  assert.equal(HEADER_HEIGHT_PX, 24);
  assert.equal(MORE_HEIGHT_PX, 28);
});

function bigShape(): ListShape {
  // 10,000 rows, 3 headers and one "more" row spread out, 47 further headers inside no list (folded).
  const special: SpecialItem[] = [];
  const total = 10_004;
  for (const i of [0, 3, 500, 9000]) {
    special.push([i, i === 9000 ? "more" : "header"]);
  }
  return { total, lanes: 3, special };
}

test("c8 the height model is consistent: itemAtY inverts itemTop for every item", () => {
  const shape = bigShape();
  for (let i = 0; i < shape.total; i += 1) {
    const top = itemTop(i, shape);
    assert.equal(itemAtY(top, shape), i, `item ${i}`);
    assert.equal(itemAtY(top + itemHeight(i, shape) - 1, shape), i, `end of item ${i}`);
  }
  const sum = Array.from({ length: shape.total }, (_, i) => itemHeight(i, shape)).reduce((a, b) => a + b, 0);
  assert.equal(listHeight(shape), sum);
});

test("c8 the visible range at any scroll position covers the viewport and stays small", () => {
  const shape = bigShape();
  const started = performance.now();
  for (let y = 0; y < listHeight(shape); y += 997) {
    const { first, last } = itemRange(y, 600, shape);
    assert.ok(first <= last);
    assert.ok(last - first < 40, `range ${first}..${last}`);
    assert.ok(first >= 0 && last < shape.total);
  }
  assert.ok(performance.now() - started < 800, "the range math is cheap");
});

test("an empty list and a one-item list do not break the math", () => {
  assert.deepEqual(itemRange(0, 600, { total: 0, lanes: 0, special: [] }), { first: 0, last: -1 });
  const one: ListShape = { total: 1, lanes: 0, special: [] };
  assert.equal(itemAtY(10_000, one), 0);
  assert.equal(itemAtY(-50, one), 0);
});

test("parsing hostile input does not throw and gives an empty list", () => {
  assert.deepEqual(parseShape("{}"), { total: 0, lanes: 0, special: [] });
  assert.deepEqual(parseItems("null"), []);
  assert.deepEqual(parseItems("{}"), []);
});
