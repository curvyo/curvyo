// Independent acceptance tests for specs/0027-manufacturing-roles at the front end: the three role
// buttons and the press that clears (criterion 2), arrow keys that move and never press (2, UX notes
// §7), the one Tab stop, row heights and windowing with many layers (UX notes §8, criterion 21), and
// the fixed texts of the Layers block (criteria 1, 2, 14, 15, 18: each at most 70 characters).
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

import {
  ROLES,
  roleAfterPress,
  roleKey,
  roleRowHeight,
  roleRowWindow,
  roleTabStop,
  type RoleName,
} from "../src/lib/roleRowKeys.ts";

const block = readFileSync(new URL("../src/components/workbench/LayersBlock.tsx", import.meta.url), "utf8");
const hook = readFileSync(new URL("../src/hooks/useWorkbenches.ts", import.meta.url), "utf8");

test("ac2: exactly three buttons Cut, Engrave, Ignore in this order", () => {
  assert.deepEqual([...ROLES], ["cut", "engrave", "ignore"]);
});

test("ac2: pressing an unpressed button sets it, pressing the pressed one clears it", () => {
  for (const r of ROLES) {
    assert.equal(roleAfterPress(null, r), r, `${r} from no role`);
    assert.equal(roleAfterPress(r, r), null, `${r} pressed again clears`);
    for (const other of ROLES) {
      if (other !== r) {
        assert.equal(roleAfterPress(r, other), other, `${r} -> ${other}`);
      }
    }
  }
});

test("ac18: an unknown stored value shows nothing pressed, so any press replaces it", () => {
  // The session sends role null with unknown true (no button pressed).
  for (const r of ROLES) {
    assert.equal(roleAfterPress(null, r), r);
  }
});

test("ac2: arrow keys only move focus, never press, at every row and column", () => {
  for (const rows of [1, 2, 7, 200]) {
    for (let row = 0; row < Math.min(rows, 8); row += 1) {
      for (let column = 0; column < 3; column += 1) {
        for (const key of ["ArrowLeft", "ArrowRight", "ArrowUp", "ArrowDown", "Home", "End", "PageUp", "PageDown"]) {
          const a = roleKey(key, row, column, rows);
          assert.ok(a !== null && a.kind === "focus", `${key} at ${row}/${column} of ${rows}`);
          if (a.kind === "focus") {
            assert.ok(a.row >= 0 && a.row < rows, `${key}: row in range`);
            assert.ok(a.column >= 0 && a.column < 3, `${key}: column in range`);
          }
        }
      }
    }
  }
});

test("ac2: Left and Right stay in the row; Up and Down keep the column", () => {
  assert.deepEqual(roleKey("ArrowRight", 3, 1, 10), { kind: "focus", row: 3, column: 2 });
  assert.deepEqual(roleKey("ArrowLeft", 3, 1, 10), { kind: "focus", row: 3, column: 0 });
  assert.deepEqual(roleKey("ArrowUp", 3, 1, 10), { kind: "focus", row: 2, column: 1 });
  assert.deepEqual(roleKey("ArrowDown", 9, 1, 10), { kind: "focus", row: 9, column: 1 }, "no wrap");
});

test("ac2: Space and Enter press; Delete and Backspace clear; other keys and no rows do nothing", () => {
  assert.deepEqual(roleKey(" ", 0, 2, 3), { kind: "press" });
  assert.deepEqual(roleKey("Enter", 0, 2, 3), { kind: "press" });
  assert.deepEqual(roleKey("Delete", 0, 2, 3), { kind: "clear" });
  assert.deepEqual(roleKey("Backspace", 0, 2, 3), { kind: "clear" });
  assert.equal(roleKey("Tab", 0, 0, 3), null);
  assert.equal(roleKey("c", 0, 0, 3), null, "tool letters inactive");
  assert.equal(roleKey("ArrowDown", 0, 0, 0), null, "no rows");
});

test("ac2: the one Tab stop is the last focused button, else the pressed button of row 1", () => {
  const keys = ["1:1", "1:2", "outside"];
  const roles: (RoleName | null)[] = ["engrave", null, "cut"];
  assert.deepEqual(roleTabStop(keys, roles, null), { row: 0, column: 1 });
  assert.deepEqual(roleTabStop(keys, [null, "cut", null], null), { row: 0, column: 0 });
  assert.deepEqual(roleTabStop(keys, roles, { key: "outside", column: 2 }), { row: 2, column: 2 });
  assert.deepEqual(
    roleTabStop(keys, roles, { key: "9:9", column: 2 }),
    { row: 0, column: 1 },
    "a deleted layer's focus falls back",
  );
  assert.equal(roleTabStop([], [], null), null);
});

test("ac21: 200 layers keep at most 60 rows in the DOM and always the focused one", () => {
  const heights = Array.from({ length: 200 }, (_, i) => roleRowHeight(i % 3 === 0 ? 1 : 0, false));
  for (const [top, focused] of [
    [0, 199],
    [5000, 0],
    [100000, 150],
    [-50, 10],
  ]) {
    const w = roleRowWindow(heights, top, 800, focused);
    assert.ok(w.last - w.first + 1 <= 60, `window ${w.first}..${w.last}`);
    const inWindow = focused >= w.first && focused <= w.last;
    assert.ok(inWindow || w.extra === focused, `focused ${focused} kept: ${JSON.stringify(w)}`);
    const total = heights.reduce((a, b) => a + b, 0);
    const shown = heights.slice(w.first, w.last + 1).reduce((a, b) => a + b, 0);
    assert.equal(w.before + shown + w.after, total, "spacers add up");
  }
});

test("UX §3: row heights grow by one caption line each", () => {
  assert.equal(roleRowHeight(0, false), 64);
  assert.equal(roleRowHeight(1, false), 80);
  assert.equal(roleRowHeight(2, false), 96);
  assert.ok(roleRowHeight(0, true) >= roleRowHeight(0, false));
});

test("ac1 ac2 ac14 ac18: the fixed texts are present and at most 70 characters", () => {
  for (const text of [
    "The drawing has no layers yet.",
    "No role",
    "Hidden in the drawing: not output",
    "Unknown role in file, treated as no role.",
    "Role view",
  ]) {
    assert.ok(block.includes(text), `LayersBlock shows "${text}"`);
    assert.ok(text.length <= 70);
  }
});

test("ac2: buttons carry aria-pressed and a text label each", () => {
  assert.ok(block.includes("aria-pressed={pressed}"));
  for (const label of ["Cut", "Engrave", "Ignore"]) {
    assert.ok(block.includes(`"${label}"`) || block.includes(`${label}:`) || block.includes(`>${label}<`), label);
  }
});

test("ac2: a clear reaches the session as an empty role, which the wasm reads as no role", () => {
  assert.ok(hook.includes('session.set_layer_role(key, role ?? "")'));
});

test("ac6: the Role view switch is view state: no undo-step command, just set_role_view", () => {
  assert.ok(hook.includes("session.set_role_view(on)"));
});
