// The source line of a derived operation row (`specs/0112` criterion 12, UX 6) in the Operations
// view decoder, against the shared fixture's shape.
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { decodeOperationsText } from "../src/lib/operationsView.ts";

const FILE = new URL("../../curvyo-editor-wasm/tests/fixtures/operations_view/five_kinds_cuts_last.json", import.meta.url);

function withSource(source: unknown): string {
  const view = JSON.parse(readFileSync(FILE, "utf8")) as { rows: Record<string, unknown>[] };
  view.rows[0].source = source;
  return JSON.stringify(view);
}

test("rows of the shared fixture have no source", () => {
  const view = decodeOperationsText(readFileSync(FILE, "utf8"));
  assert.ok(view);
  assert.ok(view.rows.every((row) => row.source === null));
});

test("a derived row carries its grid, whether it is a cell row, and its text", () => {
  const view = decodeOperationsText(
    withSource({ grid: { peer: "5", counter: 3 }, cell: true, text: "Birch 3 mm · K7Q2M column 3, row 2" }),
  );
  assert.ok(view);
  assert.deepEqual(view.rows[0].source, {
    grid: { peer: "5", counter: 3 },
    cell: true,
    text: "Birch 3 mm · K7Q2M column 3, row 2",
  });
});

test("a source with a missing or mistyped field refuses the whole view", () => {
  assert.equal(decodeOperationsText(withSource({ grid: { peer: "5", counter: 3 }, cell: true })), null);
  assert.equal(decodeOperationsText(withSource({ grid: { peer: "5" }, cell: true, text: "x" })), null);
  assert.equal(decodeOperationsText(withSource({ grid: { peer: "5", counter: 3 }, cell: "yes", text: "x" })), null);
  assert.equal(decodeOperationsText(withSource("K7Q2M")), null);
});
