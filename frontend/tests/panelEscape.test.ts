import assert from "node:assert/strict";
import { test } from "node:test";

import { panelHandlesEscape } from "../src/lib/panelEscape.ts";

test("an Escape nobody handled belongs to the panel", () => {
  assert.equal(panelHandlesEscape({ key: "Escape", defaultPrevented: false }), true);
});

test("an Escape a field already handled (hex field back to its slider) is not the panel's", () => {
  assert.equal(panelHandlesEscape({ key: "Escape", defaultPrevented: true }), false);
});

test("other keys are never the panel's Escape", () => {
  assert.equal(panelHandlesEscape({ key: "Enter", defaultPrevented: false }), false);
});
