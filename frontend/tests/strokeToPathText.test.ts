import { strict as assert } from "node:assert";
import { test } from "node:test";

import {
  isStrokeToPathChord,
  strokeToPathNotice,
} from "../src/lib/strokeToPathText.ts";

const base = { kind: "applied", count: 0, of: 1, strokes: 1, unstroked: 0, texts: 0, unlinked: 0 };

test("criterion 26: out of range uses the shared sentence", () => {
  const notice = strokeToPathNotice({ ...base, kind: "out_of_range", count: 1, of: 1 });
  assert.deepEqual(notice, {
    text: "Stroke to path works only within 10 km of the point 0, 0. The selected object reaches further. Nothing was changed.",
    alert: true,
  });
  assert.equal(strokeToPathNotice({ ...base, kind: "ignored" }), null);
});

test("Ctrl+Alt+C, not AltGr, not with Shift", () => {
  const key = { key: "c", code: "KeyC", ctrlKey: true, metaKey: false, altKey: true, shiftKey: false };
  assert.ok(isStrokeToPathChord(key));
  assert.ok(!isStrokeToPathChord({ ...key, getModifierState: (k: string) => k === "AltGraph" }));
  assert.ok(!isStrokeToPathChord({ ...key, shiftKey: true }));
  assert.ok(!isStrokeToPathChord({ ...key, altKey: false }));
  assert.ok(isStrokeToPathChord({ ...key, key: "ç", ctrlKey: false, metaKey: true }));
});
