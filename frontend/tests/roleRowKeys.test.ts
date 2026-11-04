import assert from "node:assert/strict";
import test from "node:test";

import {
  roleAfterPress,
  roleKey,
  roleRowHeight,
  roleRowWindow,
  roleTabStop,
} from "../src/lib/roleRowKeys.ts";

test("Left and Right move inside a row and never press (0027 UX notes §7)", () => {
  assert.deepEqual(roleKey("ArrowRight", 2, 0, 5), { kind: "focus", row: 2, column: 1 });
  assert.deepEqual(roleKey("ArrowRight", 2, 2, 5), { kind: "focus", row: 2, column: 2 });
  assert.deepEqual(roleKey("ArrowLeft", 2, 0, 5), { kind: "focus", row: 2, column: 0 });
});

test("Up, Down, Home, End and the page keys move to the same button of another row, no wrap", () => {
  assert.deepEqual(roleKey("ArrowDown", 4, 1, 5), { kind: "focus", row: 4, column: 1 });
  assert.deepEqual(roleKey("ArrowUp", 0, 2, 5), { kind: "focus", row: 0, column: 2 });
  assert.deepEqual(roleKey("ArrowDown", 1, 2, 5), { kind: "focus", row: 2, column: 2 });
  assert.deepEqual(roleKey("Home", 3, 1, 5), { kind: "focus", row: 0, column: 1 });
  assert.deepEqual(roleKey("End", 0, 1, 5), { kind: "focus", row: 4, column: 1 });
  assert.deepEqual(roleKey("PageDown", 0, 0, 20), { kind: "focus", row: 5, column: 0 });
  assert.deepEqual(roleKey("PageUp", 3, 0, 20), { kind: "focus", row: 0, column: 0 });
});

test("Space and Enter press, Delete and Backspace clear, Escape leaves, other keys stay", () => {
  assert.deepEqual(roleKey(" ", 0, 0, 1), { kind: "press" });
  assert.deepEqual(roleKey("Enter", 0, 0, 1), { kind: "press" });
  assert.deepEqual(roleKey("Delete", 0, 0, 1), { kind: "clear" });
  assert.deepEqual(roleKey("Backspace", 0, 0, 1), { kind: "clear" });
  assert.deepEqual(roleKey("Escape", 0, 0, 1), { kind: "leave" });
  assert.equal(roleKey("Tab", 0, 0, 1), null);
  assert.equal(roleKey("v", 0, 0, 1), null);
  assert.equal(roleKey("ArrowDown", 0, 0, 0), null);
});

test("pressing the pressed button clears the role, another sets it (criterion 2)", () => {
  assert.equal(roleAfterPress("cut", "cut"), null);
  assert.equal(roleAfterPress("cut", "engrave"), "engrave");
  assert.equal(roleAfterPress(null, "ignore"), "ignore");
});

test("the Tab stop is the last focused button, else the first row's pressed one, else its first", () => {
  assert.deepEqual(roleTabStop(["a", "b"], [null, "cut"], { key: "b", column: 2 }), { row: 1, column: 2 });
  assert.deepEqual(roleTabStop(["a", "b"], ["engrave", null], { key: "gone", column: 2 }), { row: 0, column: 1 });
  assert.deepEqual(roleTabStop(["a"], [null], null), { row: 0, column: 0 });
  assert.equal(roleTabStop([], [], null), null);
});

test("row heights follow the captions and the pointer (UX notes §3)", () => {
  assert.equal(roleRowHeight(0, false), 64);
  assert.equal(roleRowHeight(1, false), 80);
  assert.equal(roleRowHeight(2, false), 96);
  assert.equal(roleRowHeight(0, true), 72);
});

test("up to 60 rows are all in the DOM; more are windowed with the focused row kept (UX notes §8)", () => {
  const few = Array.from({ length: 60 }, () => 72);
  assert.deepEqual(roleRowWindow(few, 0, 600, 0), { first: 0, last: 59, before: 0, after: 0, extra: null });
  const many = Array.from({ length: 200 }, () => 72);
  const top = roleRowWindow(many, 0, 720, 0);
  assert.equal(top.first, 0);
  assert.equal(top.last, 9);
  assert.equal(top.before, 0);
  assert.equal(top.after, 190 * 72);
  const middle = roleRowWindow(many, 72 * 100 + 10, 720, 3);
  assert.equal(middle.first, 100);
  assert.equal(middle.before, 72 * 100);
  assert.equal(middle.extra, 3);
  assert.ok(middle.last - middle.first + 1 <= 60);
  assert.equal(middle.before + (middle.last - middle.first + 1) * 72 + middle.after, 200 * 72);
});
