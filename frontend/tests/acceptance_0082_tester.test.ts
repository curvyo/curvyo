// Independent tester cases for `specs/0082-glyph-table` on the frontend's pure rules: criteria 3
// and 18 (the stored mode, search and Recent list, each bad field falling back alone), 4 and 16
// (subject line and no-match words), 15 (cell side and columns at the edges), 18 (Recent row),
// 20 (keys across block headers and at the ends) and 19 (detail bar meta).
// Written from the specification before the components were read.

import assert from "node:assert/strict";
import { test } from "node:test";

import {
  FONT_PREVIEW_KEY,
  MAX_GLYPH_SEARCH_CHARS,
  MAX_RECENT,
  isOneCharacter,
  readPreviewPrefs,
  withRecent,
  writePreviewPrefs,
  type PreviewStorage,
} from "../src/lib/fontPreviewState.ts";
import { cellSide, glyphColumns, glyphMove, layoutGlyphGrid, recentCells } from "../src/lib/glyphTableLayout.ts";
import {
  cellName,
  codePointText,
  detailMeta,
  glyphSubjectLine,
  missingFaceText,
  noCharactersText,
  noMatchText,
  typesIntoSearch,
} from "../src/lib/glyphTableText.ts";

function memory(initial: string | null): PreviewStorage & { value: string | null } {
  return {
    value: initial,
    getItem(key: string) {
      assert.equal(key, FONT_PREVIEW_KEY);
      return this.value;
    },
    setItem(key: string, value: string) {
      assert.equal(key, FONT_PREVIEW_KEY);
      this.value = value;
    },
  };
}

// --- Criteria 3, 16, 18: stored state --------------------------------------------------------

test("ac03: mode, search and Recent survive a write and read with the other preview state", () => {
  const store = memory(null);
  const prefs = { sample: "Hi", size: 40, full: true, mode: "glyphs" as const, glyphSearch: "heart", recent: ["♥", "😀", "ä"] };
  writePreviewPrefs(prefs, store);
  assert.deepEqual(readPreviewPrefs(store), prefs);
  // A store that throws on write is ignored.
  const broken: PreviewStorage = {
    getItem: () => null,
    setItem: () => {
      throw new Error("quota");
    },
  };
  assert.doesNotThrow(() => writePreviewPrefs(prefs, broken));
});

test("ac03/18: each bad new field falls back alone, the old fields stay", () => {
  const store = memory(
    JSON.stringify({
      sample: "Kept",
      size: 50,
      full: false,
      mode: "info",
      glyphSearch: "x".repeat(MAX_GLYPH_SEARCH_CHARS + 1),
      recent: "♥",
    }),
  );
  const read = readPreviewPrefs(store);
  assert.equal(read.sample, "Kept");
  assert.equal(read.size, 50);
  assert.equal(read.mode, undefined, "an unknown mode is Fonts");
  assert.equal(read.glyphSearch, undefined);
  assert.equal(read.recent, undefined);
  const okSearch = memory(JSON.stringify({ glyphSearch: "x".repeat(MAX_GLYPH_SEARCH_CHARS), mode: "glyphs" }));
  assert.equal(readPreviewPrefs(okSearch).glyphSearch?.length, MAX_GLYPH_SEARCH_CHARS);
  assert.equal(readPreviewPrefs(okSearch).mode, "glyphs");
  assert.deepEqual(readPreviewPrefs(memory("{not json")).recent, undefined);
});

test("ac18: a stored Recent list keeps only distinct single characters, at most 24", () => {
  const many = Array.from({ length: 40 }, (_, i) => String.fromCodePoint(0x41 + i));
  const store = memory(JSON.stringify({ recent: ["♥", "ab", "", "\u0007", "\uD800", 7, null, "♥", "😀", ...many] }));
  const recent = readPreviewPrefs(store).recent ?? [];
  assert.equal(recent.length, MAX_RECENT);
  assert.deepEqual(recent.slice(0, 3), ["♥", "😀", "A"]);
  assert.equal(new Set(recent).size, recent.length);
});

test("ac18: Recent puts the newest first, moves a repeat to the front, drops the 25th", () => {
  let recent: string[] = [];
  for (let i = 0; i < 30; i += 1) {
    recent = withRecent(recent, String.fromCodePoint(0x61 + (i % 26)));
  }
  assert.equal(recent.length, MAX_RECENT);
  assert.equal(recent[0], "d");
  recent = withRecent(recent, "m");
  assert.equal(recent[0], "m");
  assert.equal(recent.filter((c) => c === "m").length, 1);
  assert.equal(recent.length, MAX_RECENT);
});

test("ac18: one character is one scalar that is not Cc (astral yes, combining pair no)", () => {
  assert.ok(isOneCharacter("😀"));
  assert.ok(isOneCharacter("\u{E000}"));
  assert.ok(isOneCharacter(" "));
  assert.ok(!isOneCharacter("é"));
  assert.ok(!isOneCharacter("\u009F"));
  assert.ok(!isOneCharacter("\uDC00"));
  assert.ok(!isOneCharacter(""));
});

// --- Criterion 18: the Recent row ------------------------------------------------------------

test("ac18: the Recent row hides characters the face does not list, keeps order, two rows max", () => {
  const listed = new Set([0x41, 0x2665, 0x1f600]);
  assert.deepEqual(recentCells(["😀", "x", "♥", "A"], listed, 8, false), [0x1f600, 0x2665, 0x41]);
  assert.deepEqual(recentCells(["😀", "♥", "A"], listed, 1, false), [0x1f600, 0x2665], "two rows of one column");
  assert.deepEqual(recentCells(["♥"], listed, 8, true), [], "not while searching");
  assert.deepEqual(recentCells([], listed, 8, false), [], "no row for an empty list");
});

// --- Criterion 15: cell geometry -------------------------------------------------------------

test("ac15: S = max(40, 2 x size); columns of S with 4px gaps; a narrow pane still has one", () => {
  assert.equal(cellSide(16), 40);
  assert.equal(cellSide(20), 40);
  assert.equal(cellSide(21), 42);
  assert.equal(cellSide(160), 320);
  const side = cellSide(32);
  const cols = glyphColumns(1000, 32);
  // The columns and their gaps fit; one more would not.
  assert.ok(cols * side + (cols - 1) * 4 <= 1000);
  assert.ok((cols + 1) * side + cols * 4 > 1000 - 24, `${cols}`);
  assert.equal(glyphColumns(10, 160), 1);
  assert.equal(glyphColumns(0, 32), 1);
});

// --- Criterion 20: keys ----------------------------------------------------------------------

test("ac20: Up and Down cross block headers, Ctrl+Up/Down jump blocks, ends stay put", () => {
  // Block 0: 5 cells in rows of 3 (3 + 2); block 1: 1 cell; block 2: 4 cells (3 + 1).
  const layout = layoutGlyphGrid(
    [
      { name: "Basic Latin", cells: [0x41, 0x42, 0x43, 0x44, 0x45] },
      { name: "Arrows", cells: [0x2190] },
      { name: "Dingbats", cells: [0x2764, 0x2765, 0x2766, 0x2767] },
    ],
    3,
    40,
  );
  assert.equal(glyphMove("ArrowDown", false, 2, layout, 200), 4, "into the shorter row: its last cell");
  assert.equal(glyphMove("ArrowDown", false, 4, layout, 200), 5, "across the header");
  assert.equal(glyphMove("ArrowDown", false, 5, layout, 200), 6);
  assert.equal(glyphMove("ArrowUp", false, 6, layout, 200), 5);
  assert.equal(glyphMove("ArrowUp", false, 0, layout, 200), 0, "first row: stays");
  assert.equal(glyphMove("ArrowDown", false, 9, layout, 200), 9, "last row: stays");
  assert.equal(glyphMove("ArrowRight", false, 9, layout, 200), 9);
  assert.equal(glyphMove("ArrowLeft", false, 0, layout, 200), 0);
  assert.equal(glyphMove("ArrowDown", true, 1, layout, 200), 5, "Ctrl+Down: next block");
  assert.equal(glyphMove("ArrowUp", true, 7, layout, 200), 5, "Ctrl+Up: previous block's first cell");
  assert.equal(glyphMove("ArrowDown", true, 7, layout, 200), 7, "no next block");
  assert.equal(glyphMove("Home", false, 4, layout, 200), 3);
  assert.equal(glyphMove("End", false, 6, layout, 200), 8);
  assert.equal(glyphMove("Home", true, 7, layout, 200), 0);
  assert.equal(glyphMove("End", true, 0, layout, 200), 9);
  assert.equal(glyphMove("PageDown", false, 0, layout, 10_000), 9, "a page past the end: last row");
  assert.equal(glyphMove("PageUp", false, 9, layout, 10_000), 0);
  assert.equal(glyphMove("Tab", false, 0, layout, 200), null);
  assert.equal(glyphMove("ArrowDown", false, 0, layoutGlyphGrid([], 3, 40), 200), null, "empty grid");
});

test("ac20: printable keys and Backspace go to the search; Ctrl/Alt chords do not", () => {
  assert.ok(typesIntoSearch("a", false, false, false));
  assert.ok(typesIntoSearch("ä", false, false, false));
  assert.ok(typesIntoSearch("Backspace", false, false, false));
  assert.ok(!typesIntoSearch("a", true, false, false));
  assert.ok(!typesIntoSearch("a", false, true, false));
  assert.ok(!typesIntoSearch("ArrowDown", false, false, false));
  assert.ok(!typesIntoSearch("Escape", false, false, false));
});

// --- Criteria 4, 11, 16, 19, 20: words ------------------------------------------------------

test("ac04/11/16: the spec's exact words", () => {
  assert.equal(glyphSubjectLine("Great Vibes Regular", 412, 412, false, false), "Great Vibes Regular · 412 characters");
  assert.equal(glyphSubjectLine("Great Vibes Regular", 412, 3, true, false), "Great Vibes Regular · 3 of 412 characters");
  assert.equal(glyphSubjectLine("Lobster Regular", 0, 0, false, true), "Lobster Regular · missing");
  assert.equal(glyphSubjectLine("Big Face", 20_000, 20_000, false, false), "Big Face · 20,000 characters");
  assert.equal(noCharactersText("Mylines Regular"), "Mylines Regular has no characters to insert.");
  assert.equal(missingFaceText("Lobster"), "Lobster is missing. Its characters cannot be shown.");
  assert.equal(noMatchText("Great Vibes", "xyz"), 'No character in Great Vibes matches "xyz".');
  for (const text of [noMatchText("Great Vibes", "x".repeat(64)), missingFaceText("Lobster"), noCharactersText("Mylines Regular")]) {
    assert.ok(text.length <= 70 || text.includes("x".repeat(64)), `notice of at most 70: ${text}`);
  }
});

test("ac19/20: code point, accessible name and detail meta", () => {
  assert.equal(codePointText(0xe4), "U+00E4");
  assert.equal(codePointText(0x41), "U+0041");
  assert.equal(codePointText(0x1f600), "U+1F600");
  assert.equal(codePointText(0x10ffff), "U+10FFFF");
  assert.equal(cellName("LATIN SMALL LETTER A WITH DIAERESIS", 0xe4), "Latin small letter a with diaeresis, U+00E4");
  assert.equal(cellName(null, 0xe000), "U+E000");
  assert.equal(detailMeta(0xe4, "adieresis", "Latin-1 Supplement"), "U+00E4 · adieresis · Latin-1 Supplement");
  assert.equal(detailMeta(0xe000, null, "Private Use Area"), "U+E000 · Private Use Area");
});
