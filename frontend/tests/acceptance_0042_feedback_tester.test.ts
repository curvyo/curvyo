// Tester acceptance tests for the customer feedback of 2026-10-14 on
// specs/0042-history-branches/specification.md, criterion 14 (one way of working, hints only in
// each row's tooltip and accessible name), written from the spec text.

import assert from "node:assert/strict";
import { existsSync, readFileSync } from "node:fs";
import test from "node:test";

import * as preview from "../src/lib/historyPreview.ts";
import * as rows from "../src/lib/historyRows.ts";

const HINT = "Click to preview. Double-click or Enter to apply this version.";

test("c14 a row's tooltip ends with the hint, said once", () => {
  const t = preview.rowHintTooltip("Draw rectangle, You, 12:03");
  assert.ok(t.endsWith(HINT), t);
  assert.equal(t.split(HINT).length - 1, 1);
  assert.ok(t.startsWith("Draw rectangle, You, 12:03"));
});

test("c14 a row's accessible name ends with 'Enter applies this version'", () => {
  const n = preview.rowHintAriaName("Draw rectangle, applied, on branch from a1b2c3d, step 2 of 2");
  assert.ok(n.endsWith("Enter applies this version"), n);
  assert.ok(n.includes("step 2 of 2"), "the lane part stays in the name");
});

test("c14 the Mode switch, its captions and the footer line are gone from the modules", () => {
  const p = preview as Record<string, unknown>;
  assert.equal(p.MODE_TOOLTIP, undefined);
  assert.equal(p.modeCaption, undefined);
  assert.equal((rows as Record<string, unknown>).SAVED_IN_FILE_LINE, undefined);
  assert.equal(existsSync(new URL("../src/components/panel/HistoryModeRow.tsx", import.meta.url)), false);
});

test("c14 the History tab source has no Mode row, switch, caption or help line", () => {
  const tab = readFileSync(new URL("../src/components/panel/HistoryTab.tsx", import.meta.url), "utf8");
  const row = readFileSync(new URL("../src/components/panel/HistoryRow.tsx", import.meta.url), "utf8");
  for (const gone of [
    "HistoryModeRow",
    "Restore previous state",
    "A click shows the version",
    "saved in the project file",
    "history_live",
    "history_go_to_item",
    "history_exploring",
    "Apply version",
  ]) {
    assert.ok(!tab.includes(gone) && !row.includes(gone), `still mentions: ${gone}`);
  }
});

test("c14 Enter and double-click apply, Space and click preview, a double-click on the id applies nothing", () => {
  const row = readFileSync(new URL("../src/components/panel/HistoryRow.tsx", import.meta.url), "utf8");
  assert.match(row, /event\.key === "Enter"[\s\S]{0,80}onApply\(\)/);
  assert.match(row, /event\.key === " "[\s\S]{0,80}onPress\(\)/);
  assert.match(row, /onClick=\{onPress\}/);
  assert.match(row, /onDoubleClick/);
  assert.match(row, /closest\("\[data-step-id\]"\)/);
  assert.match(row, /data-step-id/);
});
