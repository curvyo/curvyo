import assert from "node:assert/strict";
import { test } from "node:test";

import {
  ALIGN_PAGE_RULES,
  DISTRIBUTE_PAGE_RULES,
  alignRule,
  distributeRule,
  ALIGN_ORDER,
  ALIGN_TEXT,
  DISTRIBUTE_ORDER,
  DISTRIBUTE_TEXT,
  GAP_INVALID_NUMBER,
  GAP_NEGATIVE,
  pageCueFor,
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

// Criteria 14 to 16 and the UX notes "Grid and page reference": the rule
// line names what the buttons act on — the page's objects, the selected
// objects relative to the page, or the selection.
test("the rule lines follow the reference", () => {
  assert.equal(
    alignRule("left", "page-objects"),
    "Nothing selected: every object of the page onto its left edge.",
  );
  assert.equal(alignRule("left", "page"), "Its left edge onto the page's left edge.");
  assert.equal(alignRule("centre-h", "page"), "Its centre on the page's horizontal centre.");
  assert.equal(alignRule("centre-v", "page"), "Its centre on the page's vertical centre.");
  assert.equal(alignRule("left", "selection"), ALIGN_TEXT.left.rule);
  assert.equal(
    distributeRule("h-gaps", "page-objects", 0),
    "Nothing selected: the page's objects evenly across the page, equal gaps.",
  );
  assert.equal(
    distributeRule("h-gaps", "page", 1),
    "Centred across the page, equal gaps to both edges.",
  );
  assert.equal(distributeRule("v-centres", "page", 1), "Centred down the page.");
  assert.equal(
    distributeRule("h-gaps", "page", 2),
    "Across the page, equal gaps between them and to both edges.",
  );
  assert.equal(distributeRule("v-centres", "selection", 3), DISTRIBUTE_TEXT["v-centres"].rule);
  // Every command has a line per reference, none of them empty.
  for (const command of ALIGN_ORDER) {
    for (const reference of ["page-objects", "page", "selection"] as const) {
      assert.ok(alignRule(command, reference).length > 0, command);
    }
  }
  for (const code of DISTRIBUTE_ORDER) {
    assert.ok(DISTRIBUTE_PAGE_RULES[code].length > 0, code);
    assert.ok(distributeRule(code, "page", 1).length > 0, code);
    assert.ok(distributeRule(code, "page", 2).length > 0, code);
  }
  assert.equal(ALIGN_PAGE_RULES.left, alignRule("left", "page-objects"));
});

test("only the page references draw the page cue", () => {
  assert.equal(pageCueFor("page-objects"), true);
  assert.equal(pageCueFor("page"), true);
  assert.equal(pageCueFor("selection"), false);
});
