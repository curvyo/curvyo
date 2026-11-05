// Pick mask paths (`specs/0094-masked-composite-trace` slice 1b, criteria 35, 38, 43; UX notes
// §11): the names, the polite status and the canvas cursor Rust names "pick-mask".

import assert from "node:assert/strict";
import test from "node:test";

import { cursorForHint } from "../src/lib/cursors.ts";
import { CLEAR_MASK, PICK_MASK_PATHS, TRACE_MIXED, maskPickStatus } from "../src/lib/traceText.ts";

test("the pick-mask hint is the arrow with the mask glyph and crosshair as its fallback", () => {
  const cursor = cursorForHint("pick-mask");
  assert.ok(cursor?.startsWith('url("data:image/svg+xml,'), cursor);
  assert.ok(cursor?.endsWith(" 3 3, crosshair"), "hotspot at the arrow tip");
  assert.notEqual(cursor, cursorForHint("pick-carrier"), "its own glyph, not On path's");
});

test("the names and the polite status state the control and the mode, no instruction", () => {
  assert.equal(PICK_MASK_PATHS, "Pick mask paths");
  assert.equal(CLEAR_MASK, "Clear mask");
  assert.equal(maskPickStatus(true), "Pick mask paths on");
  assert.equal(maskPickStatus(false), "Pick mask paths off");
  assert.equal(TRACE_MIXED, "Use Object to path for text; one image at a time.", "the refused row reuses criterion 14's string");
});
