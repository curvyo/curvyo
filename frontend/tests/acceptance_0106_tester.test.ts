import assert from "node:assert/strict";
import { test } from "node:test";

import { isPlainRow, parseLayersView, pressedId, rowKey } from "../src/lib/layerRows.ts";

const ROW = (kind: string, id: number, extra = "") =>
  `{"index":0,"id":{"peer":"1","counter":${id}},"kind":"${kind}","level":2,"mark":"","name":"x","label":"x"${extra}}`;

test("0106 criterion 18: one source placed twice gives distinct keys and presses the placement", () => {
  const view = parseLayersView(
    `{"rows":[${ROW("placed-page", 1)},${ROW("rectangle", 7, ',"via":{"peer":"1","counter":1},"readOnly":true')},${ROW("placed-page", 2)},${ROW("rectangle", 7, ',"via":{"peer":"1","counter":2},"readOnly":true')}]}`,
  );
  const keys = view.rows.map(rowKey);
  assert.equal(new Set(keys).size, 4, keys.join(","));
  assert.deepEqual(pressedId(view.rows[1]), { peer: "1", counter: 1 });
  assert.deepEqual(pressedId(view.rows[3]), { peer: "1", counter: 2 });
  assert.deepEqual(pressedId(view.rows[0]), { peer: "1", counter: 1 }, "an own row presses itself");
});

test("0106 criteria 17 to 19: plain rows have no eye, lock or drag; own items keep them", () => {
  const view = parseLayersView(
    `{"rows":[${ROW("page-root", 3)},${ROW("placed-page", 1)},${ROW("fixtures", 3)},${ROW("note", 3)},${ROW("rectangle", 7, ',"readOnly":true')},${ROW("rectangle", 8)},${ROW("clone", 9)}]}`,
  );
  const plain = view.rows.map(isPlainRow);
  assert.deepEqual(plain, [true, true, true, true, true, false, false]);
  assert.notEqual(rowKey(view.rows[0]), rowKey(view.rows[2]), "root and Fixtures share the page id");
});
