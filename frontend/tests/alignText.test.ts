import assert from "node:assert/strict";
import { test } from "node:test";

import {
  ALIGN_ORDER,
  ALIGN_TEXT,
  DISTRIBUTE_ORDER,
  DISTRIBUTE_TEXT,
  GAP_INVALID_NUMBER,
  GAP_NEGATIVE,
  parseGap,
} from "../src/lib/alignText.ts";

test("criterion 2: the six align buttons, their names and rules", () => {
  assert.deepEqual(ALIGN_ORDER, ["left", "centre-h", "right", "top", "centre-v", "bottom"]);
  assert.equal(ALIGN_TEXT.left.name, "Align left edges");
  assert.equal(ALIGN_TEXT["centre-h"].name, "Align horizontal centres");
  assert.equal(ALIGN_TEXT["centre-v"].name, "Align vertical centres");
  assert.equal(ALIGN_TEXT.right.rule, "Right edges land on the reference's right edge.");
});

test("criterion 6: the four distribute buttons, their names and rules", () => {
  assert.deepEqual(DISTRIBUTE_ORDER, ["h-gaps", "h-centres", "v-gaps", "v-centres"]);
  assert.equal(DISTRIBUTE_TEXT["h-gaps"].name, "Distribute horizontal gaps");
  assert.equal(DISTRIBUTE_TEXT["v-centres"].name, "Distribute vertical centres");
  assert.equal(
    DISTRIBUTE_TEXT["h-gaps"].rule,
    "Equal gaps between the objects, horizontally. The outermost stay.",
  );
  // Every name is distinct, so the four are told apart by ear.
  const names = DISTRIBUTE_ORDER.map((code) => DISTRIBUTE_TEXT[code].name);
  assert.equal(new Set(names).size, names.length);
});

test("criterion 8: the Gap field reads empty, a value, and refuses the rest", () => {
  assert.deepEqual(parseGap(""), { ok: true, mm: null });
  assert.deepEqual(parseGap("   "), { ok: true, mm: null });
  assert.deepEqual(parseGap("2.5"), { ok: true, mm: 2.5 });
  assert.deepEqual(parseGap("2,5"), { ok: true, mm: 2.5 });
  assert.deepEqual(parseGap("0"), { ok: true, mm: 0 });
  assert.deepEqual(parseGap("−1"), { ok: false, message: GAP_NEGATIVE });
  assert.deepEqual(parseGap("-1"), { ok: false, message: GAP_NEGATIVE });
  assert.deepEqual(parseGap("abc"), { ok: false, message: GAP_INVALID_NUMBER });
  assert.equal(GAP_NEGATIVE, "Must be 0 or more.");
  assert.equal(GAP_INVALID_NUMBER, "Enter a number");
});
