import assert from "node:assert/strict";
import { test } from "node:test";

import {
  GRID_NOT_ENOUGH_CELLS,
  GRID_TOO_SMALL,
  GRID_WHOLE_NUMBER,
  gridPlan,
  nearestSquareGrid,
  parseCount,
} from "../src/lib/gridText.ts";

// `specs/0061-align-and-distribute/` criterion 19 and the UX notes "Field
// behaviour": the nearest-to-square grid, the linked counts and the refusals.
test("criterion 19: the initial grid is the nearest to square", () => {
  assert.deepEqual(nearestSquareGrid(2), { rows: 1, columns: 2 });
  assert.deepEqual(nearestSquareGrid(6), { rows: 2, columns: 3 });
  assert.deepEqual(nearestSquareGrid(7), { rows: 3, columns: 3 });
  assert.deepEqual(nearestSquareGrid(9), { rows: 3, columns: 3 });
  assert.deepEqual(nearestSquareGrid(10), { rows: 3, columns: 4 });
});

test("criterion 19: the typed count drives, the other holds all objects", () => {
  const empty = { rows: "", columns: "", gap: "" };
  assert.deepEqual(gridPlan(6, empty), {
    ok: true,
    rows: 2,
    columns: 3,
    gapMm: 0,
    derived: { rows: 2, columns: 3 },
  });
  const columns = gridPlan(7, { rows: "", columns: "2", gap: "5" });
  assert.deepEqual(columns, { ok: true, rows: 4, columns: 2, gapMm: 5, derived: { rows: 4, columns: 2 } });
  const rows = gridPlan(7, { rows: "2", columns: "", gap: "" });
  assert.deepEqual(rows, { ok: true, rows: 2, columns: 4, gapMm: 0, derived: { rows: 2, columns: 4 } });
  // Both typed with more cells than objects: the last row partly empty.
  const both = gridPlan(5, { rows: "3", columns: "3", gap: "" });
  assert.equal(both.ok, true);
});

test("criterion 19: counts and gaps that cannot work are refused per field", () => {
  assert.deepEqual(parseCount("3"), { ok: true, value: 3 });
  assert.deepEqual(parseCount(" 4 "), { ok: true, value: 4 });
  assert.deepEqual(parseCount("1.5"), { ok: false, message: GRID_WHOLE_NUMBER });
  assert.deepEqual(parseCount("abc"), { ok: false, message: GRID_WHOLE_NUMBER });
  assert.deepEqual(parseCount("-"), { ok: false, message: GRID_WHOLE_NUMBER });
  assert.deepEqual(parseCount("0"), { ok: false, message: GRID_TOO_SMALL });
  assert.deepEqual(parseCount("-2"), { ok: false, message: GRID_TOO_SMALL });
  assert.deepEqual(gridPlan(4, { rows: "x", columns: "", gap: "" }), {
    ok: false,
    errors: { rows: GRID_WHOLE_NUMBER },
  });
  assert.deepEqual(gridPlan(4, { rows: "", columns: "", gap: "-1" }), {
    ok: false,
    errors: { gap: "Must be 0 or more." },
  });
  // Both typed, too few cells: Columns says why, Rows is marked too.
  assert.deepEqual(gridPlan(7, { rows: "2", columns: "3", gap: "" }), {
    ok: false,
    errors: { columns: GRID_NOT_ENOUGH_CELLS(7), rows: "" },
  });
  assert.equal(GRID_NOT_ENOUGH_CELLS(7), "Not enough cells for 7 objects.");
  assert.equal(GRID_WHOLE_NUMBER, "Enter a whole number");
  assert.equal(GRID_TOO_SMALL, "Must be 1 or more.");
});
