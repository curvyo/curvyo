// The value field's bar edge breaks where it would cross the label or the value text (UI review of
// `specs/0071-pattern-fill`, item 7).
import assert from "node:assert/strict";
import { test } from "node:test";

import { edgeCrossesText } from "../src/lib/valueEdge.ts";

test("an edge through the value text crosses it", () => {
  assert.equal(edgeCrossesText(190, [[8, 48], [180, 214]]), true);
});

test("an edge through the label crosses it", () => {
  assert.equal(edgeCrossesText(40, [[8, 48], [180, 214]]), true);
});

test("an edge clear of both texts does not", () => {
  assert.equal(edgeCrossesText(120, [[8, 48], [180, 214]]), false);
});

test("an edge just beside a text still counts, so the 2 px line never touches a glyph", () => {
  assert.equal(edgeCrossesText(216, [[180, 214]]), true);
  assert.equal(edgeCrossesText(220, [[180, 214]]), false);
});
