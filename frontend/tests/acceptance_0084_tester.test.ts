// Independent tester cases for `specs/0084-text-bar-second-row` on the frontend's pure rules:
// criteria 2 and 3 (Font button name and tooltip edges, Mixed with a missing face, UX notes §0.4),
// 11 (expand label), 13 (the changed list with Mixed values) and 14 (stored row 2 state edges).
// Written from the specification before the components were read; no case repeats
// `textBarRows.test.ts`.

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

function memory(): BarStorage & { values: Map<string, string> } {
  const values = new Map<string, string>();
  return {
    values,
    getItem: (key) => values.get(key) ?? null,
    setItem: (key, value) => void values.set(key, value),
  };
}

test("c14: a stored false, an empty string and a JSON non-boolean all read as closed", () => {
  const storage = memory();
  for (const raw of ["false", "", "1", "null", '"true"', "{}", "TRUE"]) {
    storage.values.set(TEXT_BAR_ROW2_KEY, raw);
    assert.equal(readRow2Open(storage), false, `raw ${JSON.stringify(raw)}`);
  }
});

test("c14: writing closed stores the JSON boolean false and touches no other key", () => {
  const storage = memory();
  storage.values.set("curvyo.panel-collapsed", "true");
  writeRow2Open(false, storage);
  assert.equal(storage.values.get(TEXT_BAR_ROW2_KEY), "false");
  assert.equal(storage.values.get("curvyo.panel-collapsed"), "true");
  assert.equal(storage.values.size, 2);
});

test("c14: toggling many times ends in the last state", () => {
  const storage = memory();
  let open = false;
  for (let i = 0; i < 51; i += 1) {
    open = !open;
    writeRow2Open(open, storage);
  }
  assert.equal(readRow2Open(storage), true);
});

test("c3 / UX §0.4: Mixed with a missing face is named 'Font: Mixed, missing'", () => {
  assert.equal(fontButtonName({ font: "", missing: true }), "Font: Mixed, missing");
});

test("c2: a face name with punctuation and non-Latin letters is kept verbatim", () => {
  assert.equal(fontButtonName({ font: "Noto Sans JP Bold", missing: false }), "Font: Noto Sans JP Bold");
  assert.equal(fontButtonName({ font: "Ünïcode, Font: Italic", missing: false }), "Font: Ünïcode, Font: Italic");
});

test("c2: the desktop Mixed tooltip names the key and Mixed and nothing else", () => {
  assert.deepEqual(fontTooltipLines({ font: "", missing: false, substitute: "Source Sans 3", desktop: true }), [
    "Font (Shift+Ctrl+T)",
    "Mixed",
  ]);
});

test("c2: the desktop missing tooltip has exactly three lines in order", () => {
  assert.deepEqual(
    fontTooltipLines({ font: "Lobster Regular", missing: true, substitute: "Source Sans 3", desktop: true }),
    ["Font (Shift+Ctrl+T)", "Lobster Regular", "Missing on this computer. Shown in Source Sans 3."],
  );
});

test("c11: the two labels differ and neither is empty", () => {
  assert.notEqual(expandLabel(false), expandLabel(true));
  assert.ok(expandLabel(false).length > 0 && expandLabel(true).length > 0);
});

test("c13: Mixed orientation on a horizontal text is not changed (orientation does not act)", () => {
  assert.deepEqual(changedRow2Controls({ writingMode: "horizontal", orientation: null }), []);
});

test("c13: Mixed writing mode with Mixed orientation counts both, in row 2 order", () => {
  assert.deepEqual(changedRow2Controls({ writingMode: null, orientation: null }), [
    "Writing mode",
    "Glyph orientation",
  ]);
});

test("c13: vertical with Upright counts both; vertical with Auto counts only Writing mode", () => {
  assert.deepEqual(changedRow2Controls({ writingMode: "vertical-rl", orientation: "upright" }), [
    "Writing mode",
    "Glyph orientation",
  ]);
  assert.deepEqual(changedRow2Controls({ writingMode: "vertical-lr", orientation: "auto" }), ["Writing mode"]);
});

test("c13: the description of one changed control has no trailing comma", () => {
  assert.equal(changedText(["Writing mode"]), "Changed: Writing mode");
  assert.equal(changedText(["Writing mode", "Taper"]), "Changed: Writing mode, Taper");
});
