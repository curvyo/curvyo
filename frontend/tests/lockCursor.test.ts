// The lock cursor over a locked object (`specs/0050-layers-and-objects-panel` criterion 20,
// customer feedback 2026-10-14): Rust says "locked", this turns it into a CSS cursor.

import assert from "node:assert/strict";
import test from "node:test";

import { cursorForHint } from "../src/lib/cursors.ts";

test("the locked hint is a custom padlock cursor with not-allowed as its fallback", () => {
  const cursor = cursorForHint("locked");
  assert.ok(cursor?.startsWith('url("data:image/svg+xml,'), cursor);
  assert.ok(cursor?.endsWith(" 3 3, not-allowed"), "hotspot at the arrow tip");
  assert.equal(cursorForHint("default"), undefined);
});
