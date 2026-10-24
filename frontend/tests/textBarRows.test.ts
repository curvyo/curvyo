// `specs/0084-text-bar-second-row`: the Font jump button's words (criteria 2, 3), the expand
// button's words and changed list (criteria 11, 13) and the stored row 2 state (criterion 14).

import assert from "node:assert/strict";
import { test } from "node:test";

import {
  type BarStorage,
  changedRow2Controls,
  changedText,
  expandLabel,
  fontButtonName,
  fontTooltipLines,
  readRow2Open,
  TEXT_BAR_ROW2_KEY,
  writeRow2Open,
} from "../src/lib/textBarRows.ts";
import { noticeShift } from "../src/lib/barLayout.ts";

function memory(): BarStorage & { values: Map<string, string> } {
  const values = new Map<string, string>();
  return {
    values,
    getItem: (key) => values.get(key) ?? null,
    setItem: (key, value) => void values.set(key, value),
  };
}

const throwing: BarStorage = {
  getItem: () => {
    throw new Error("blocked");
  },
  setItem: () => {
    throw new Error("blocked");
  },
};

test("the key is curvyo.text-bar-row2 and a new user starts closed", () => {
  assert.equal(TEXT_BAR_ROW2_KEY, "curvyo.text-bar-row2");
  assert.equal(readRow2Open(memory()), false);
});

test("open and closed are read back; open is the JSON boolean true", () => {
  const storage = memory();
  writeRow2Open(true, storage);
  assert.equal(storage.values.get(TEXT_BAR_ROW2_KEY), "true");
  assert.equal(readRow2Open(storage), true);
  writeRow2Open(false, storage);
  assert.equal(readRow2Open(storage), false);
});

test("an unknown value, throwing storage or no storage mean closed and never throw", () => {
  const storage = memory();
  storage.values.set(TEXT_BAR_ROW2_KEY, "yes");
  assert.equal(readRow2Open(storage), false);
  assert.equal(readRow2Open(throwing), false);
  assert.equal(readRow2Open(undefined), false);
  assert.doesNotThrow(() => writeRow2Open(true, throwing));
  assert.doesNotThrow(() => writeRow2Open(true, undefined));
});

test("the Font button is named after the face, Mixed, and missing", () => {
  assert.equal(fontButtonName({ font: "Source Sans 3 Regular", missing: false }), "Font: Source Sans 3 Regular");
  assert.equal(fontButtonName({ font: "", missing: false }), "Font: Mixed");
  assert.equal(fontButtonName({ font: "Lobster Regular", missing: true }), "Font: Lobster Regular, missing");
});

test("the Font tooltip names the key on the desktop only, then the face, then the missing line", () => {
  assert.deepEqual(
    fontTooltipLines({ font: "Source Sans 3 Regular", missing: false, substitute: "Source Sans 3", desktop: true }),
    ["Font (Shift+Ctrl+T)", "Source Sans 3 Regular"],
  );
  assert.deepEqual(fontTooltipLines({ font: "", missing: false, substitute: "Source Sans 3", desktop: false }), [
    "Font",
    "Mixed",
  ]);
  assert.deepEqual(
    fontTooltipLines({ font: "Lobster Regular", missing: true, substitute: "Source Sans 3", desktop: false }),
    ["Font", "Lobster Regular", "Missing on this computer. Shown in Source Sans 3."],
  );
});

test("Mixed fonts with a missing one count the missing faces (UX notes §0.4)", () => {
  const bar = { font: "", missing: true, faces: 3, missingFaces: 1 };
  assert.equal(fontButtonName(bar), "Font: Mixed, missing");
  assert.deepEqual(fontTooltipLines({ ...bar, substitute: "Source Sans 3", desktop: true }), [
    "Font (Shift+Ctrl+T)",
    "Mixed",
    "1 of 3 fonts missing on this computer.",
  ]);
});

test("the expand button says what a press does", () => {
  assert.equal(expandLabel(false), "More text options");
  assert.equal(expandLabel(true), "Fewer text options");
});

test("default row 2 values change nothing", () => {
  assert.deepEqual(changedRow2Controls({ writingMode: "horizontal", orientation: "auto" }), []);
  // The orientation of a horizontal text does not act, so it does not count.
  assert.deepEqual(changedRow2Controls({ writingMode: "horizontal", orientation: "upright" }), []);
  assert.equal(changedText([]), null);
});

test("a vertical or mixed writing mode counts, and its orientation when not Auto", () => {
  assert.deepEqual(changedRow2Controls({ writingMode: "vertical-rl", orientation: "auto" }), ["Writing mode"]);
  assert.deepEqual(changedRow2Controls({ writingMode: null, orientation: "auto" }), ["Writing mode"]);
  assert.deepEqual(changedRow2Controls({ writingMode: "vertical-lr", orientation: null }), [
    "Writing mode",
    "Glyph orientation",
  ]);
  assert.equal(changedText(["Writing mode", "Glyph orientation"]), "Changed: Writing mode, Glyph orientation");
});

test("a bar notice is shifted to stay inside the bars' area (finding c17)", () => {
  // The Size notice, 216px right-aligned to the field near the bar's left edge.
  assert.equal(noticeShift(128, 344, 152, 476), 24);
  assert.equal(noticeShift(200, 416, 152, 476), 0);
  assert.equal(noticeShift(400, 600, 152, 476), -124);
  // Wider than the area: the left edge wins.
  assert.equal(noticeShift(100, 900, 152, 476), 52);
});
