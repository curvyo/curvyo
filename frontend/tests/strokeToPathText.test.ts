import { strict as assert } from "node:assert";
import { test } from "node:test";

import {
  isStrokeToPathChord,
  strokeToPathNotice,
  strokeToPathSuccessText,
} from "../src/lib/strokeToPathText.ts";

const base = { kind: "applied", count: 0, of: 1, strokes: 1, unstroked: 0, texts: 0, unlinked: 0 };

test("criterion 22: the success notice and its sentences in order", () => {
  assert.equal(strokeToPathSuccessText(base), "Stroke to path: 1 stroke became a filled outline.");
  assert.equal(
    strokeToPathSuccessText({ ...base, strokes: 3, unstroked: 2, texts: 1, unlinked: 2 }),
    "Stroke to path: 3 strokes became filled outlines. 2 objects have no stroke and were left as they are. 1 text was converted to paths. 2 clones were unlinked.",
  );
  assert.ok("Stroke to path: 3 strokes became filled outlines.".length <= 70);
});

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
