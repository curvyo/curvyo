// `specs/0082-glyph-table`: the glyph table's stored state, Recent list, geometry, keys and words.

import assert from "node:assert/strict";
import { test } from "node:test";

import {
  FONT_PREVIEW_KEY,
  MAX_RECENT,
  isOneCharacter,
  readPreviewPrefs,
  withRecent,
  writePreviewPrefs,
} from "../src/lib/fontPreviewState.ts";
import {
  GLYPH_HEADER_PX,
  GLYPH_PAD_PX,
  cellSide,
  glyphColumns,
  glyphMove,
  glyphRowsInPage,
  focusStaysOnCell,
  glyphPageItems,
  layoutGlyphGrid,
  recentCells,
  scrollToCell,
  stickyHeader,
} from "../src/lib/glyphTableLayout.ts";
import {
  cellName,
  codePointText,
  detailMeta,
  emptyCellLabel,
  glyphSubjectLine,
  missingFaceText,
  needsNames,
  noCharactersText,
  noMatchText,
  typesIntoSearch,
} from "../src/lib/glyphTableText.ts";

function memory() {
  const values = new Map<string, string>();
  return { values, getItem: (key: string) => values.get(key) ?? null, setItem: (key: string, value: string) => void values.set(key, value) };
}

// Criteria 3, 16, 18 and adrs.md decision 5: three optional fields of the same key.
test("the mode, the search and the Recent list are kept with the preview state", () => {
  const storage = memory();
  writePreviewPrefs({ sample: "a", size: 32, full: false, mode: "glyphs", glyphSearch: "heart", recent: ["♥", "ä"] }, storage);
  assert.deepEqual(readPreviewPrefs(storage), { sample: "a", size: 32, full: false, mode: "glyphs", glyphSearch: "heart", recent: ["♥", "ä"] });
  // Each unreadable field falls back alone; the old fields stay.
  storage.values.set(
    FONT_PREVIEW_KEY,
    JSON.stringify({ sample: "a", size: 32, full: false, mode: "table", glyphSearch: "x".repeat(65), recent: ["ab", "\u0007", "♥", "♥", 3] }),
  );
  assert.deepEqual(readPreviewPrefs(storage), { sample: "a", size: 32, full: false, recent: ["♥"] });
});

test("Recent keeps the last 24 distinct characters, most recent first", () => {
  let recent: string[] = [];
  for (let i = 0; i < 30; i += 1) {
    recent = withRecent(recent, String.fromCodePoint(0x41 + i));
  }
  assert.equal(recent.length, MAX_RECENT);
  assert.equal(recent[0], String.fromCodePoint(0x41 + 29));
  recent = withRecent(recent, String.fromCodePoint(0x41 + 10));
  assert.equal(recent[0], "K");
  assert.equal(new Set(recent).size, recent.length);
  assert.equal(isOneCharacter("😀"), true);
  assert.equal(isOneCharacter("\uD800"), false);
  assert.equal(isOneCharacter("\n"), false);
});

// Criterion 15, UX notes P2 and §7.
test("cells are squares of max(40px, 2 x size) in columns from the left", () => {
  assert.equal(cellSide(16), 40);
  assert.equal(cellSide(48), 96);
  // The UX notes' fit table: grid areas 297 and 447 px (padding included as the scroll area).
  assert.equal(glyphColumns(297 + 0, 48), 2);
  assert.equal(glyphColumns(447, 48), 4);
  assert.equal(glyphColumns(927, 48), 9);
  assert.equal(glyphColumns(927, 24), 17);
  assert.equal(glyphColumns(10, 160), 1, "at least one column");
});

const GROUPS = [
  { name: "Basic Latin", cells: [0x41, 0x42, 0x43, 0x44, 0x45] },
  { name: "Latin-1 Supplement", cells: [0xe4, 0xf6] },
  { name: "Arrows", cells: [0x2190, 0x2191, 0x2192, 0x2193, 0x2194, 0x2195, 0x2196] },
];

test("blocks lay out as a header and rows of cells", () => {
  const layout = layoutGlyphGrid(GROUPS, 3, 40);
  assert.deepEqual(
    layout.rows.map((r) => (r.kind === "header" ? `h${r.group}` : `${r.first}+${r.count}`)),
    ["h0", "0+3", "3+2", "h1", "5+2", "h2", "7+3", "10+3", "13+1"],
  );
  assert.equal(layout.rows[0].top, GLYPH_PAD_PX);
  assert.equal(layout.rows[1].top, GLYPH_PAD_PX + GLYPH_HEADER_PX);
  assert.equal(layout.rows[2].top, GLYPH_PAD_PX + GLYPH_HEADER_PX + 44);
  assert.equal(layout.rows[3].top, GLYPH_PAD_PX + GLYPH_HEADER_PX + 44 + 40 + 12);
  assert.deepEqual(layout.groupStart, [0, 5, 7]);
  assert.equal(layout.cells.length, 14);
});

test("the page holds the rows in view and one screen below", () => {
  const many = [{ name: "CJK", cells: Array.from({ length: 20_000 }, (_, i) => 0x4e00 + i) }];
  const layout = layoutGlyphGrid(many, 10, 40);
  const page = glyphRowsInPage(layout, 44 * 1000, 400);
  assert.ok(page.first > 990 && page.first <= 1000, String(page.first));
  assert.ok(page.last - page.first <= 22, String(page.last - page.first));
});

test("the header of the block at the top sticks and the next one pushes it", () => {
  const layout = layoutGlyphGrid(GROUPS, 3, 40);
  assert.equal(stickyHeader(layout, 0), null);
  assert.deepEqual(stickyHeader(layout, 20), { group: 0, offset: 0 });
  const second = layout.rows[3].top;
  assert.deepEqual(stickyHeader(layout, second - 10), { group: 0, offset: -18 });
  assert.deepEqual(stickyHeader(layout, second), { group: 1, offset: 0 });
});

// Criterion 20 and UX notes P5.
test("arrows, Home, End, pages and the block jump move the focus", () => {
  const layout = layoutGlyphGrid(GROUPS, 3, 40);
  assert.equal(glyphMove("ArrowRight", false, 4, layout, 100), 5, "across a header");
  assert.equal(glyphMove("ArrowLeft", false, 0, layout, 100), 0);
  assert.equal(glyphMove("ArrowDown", false, 2, layout, 100), 4, "to a shorter row: its last cell");
  assert.equal(glyphMove("ArrowDown", false, 4, layout, 100), 6, "across a header, same column");
  assert.equal(glyphMove("ArrowUp", false, 5, layout, 100), 3);
  assert.equal(glyphMove("ArrowUp", false, 1, layout, 100), 1, "the first row stays");
  assert.equal(glyphMove("Home", false, 9, layout, 100), 7);
  assert.equal(glyphMove("End", false, 7, layout, 100), 9);
  assert.equal(glyphMove("Home", true, 9, layout, 100), 0);
  assert.equal(glyphMove("End", true, 0, layout, 100), 13);
  assert.equal(glyphMove("ArrowDown", true, 1, layout, 100), 5);
  assert.equal(glyphMove("ArrowUp", true, 8, layout, 100), 5);
  assert.equal(glyphMove("ArrowUp", true, 1, layout, 100), 1);
  assert.equal(glyphMove("PageDown", false, 0, layout, 150), 5, "the row nearest one page down");
  assert.equal(glyphMove("Enter", false, 0, layout, 100), null);
  assert.equal(typesIntoSearch("a", false, false, false), true);
  assert.equal(typesIntoSearch("Backspace", false, false, false), true);
  assert.equal(typesIntoSearch("a", true, false, false), false);
});

test("a focused cell is scrolled into view below the sticky header", () => {
  const layout = layoutGlyphGrid(GROUPS, 3, 40);
  assert.equal(scrollToCell(layout, 0, 0, 400), null);
  const last = layout.rows[layout.rowOf[13]];
  assert.equal(scrollToCell(layout, 13, 0, 100), last.top + 40 + GLYPH_PAD_PX - 100);
  assert.equal(scrollToCell(layout, 0, 200, 100), GLYPH_PAD_PX + GLYPH_HEADER_PX - GLYPH_HEADER_PX);
});

// Criterion 18, UX notes P3.
test("the Recent row shows listed recent characters, at most two rows, not while searching", () => {
  const listed = new Set([0x41, 0x42, 0x2665]);
  assert.deepEqual(recentCells(["♥", "ü", "A", "B"], listed, 1, false), [0x2665, 0x41]);
  assert.deepEqual(recentCells(["♥", "A", "B"], listed, 5, false), [0x2665, 0x41, 0x42]);
  assert.deepEqual(recentCells(["♥"], listed, 5, true), []);
});

// Criteria 4, 11, 16, 19, 20 and UX notes P7.
test("the subject line, the empty texts and the names", () => {
  assert.equal(glyphSubjectLine("Great Vibes Regular", 412, 412, false, false), "Great Vibes Regular · 412 characters");
  assert.equal(glyphSubjectLine("Great Vibes Regular", 412, 3, true, false), "Great Vibes Regular · 3 of 412 characters");
  assert.equal(glyphSubjectLine("Source Sans 3 Regular", 3000, 3000, false, false), "Source Sans 3 Regular · 3,000 characters");
  assert.equal(glyphSubjectLine("Lobster Regular", 0, 0, false, true), "Lobster Regular · missing");
  assert.equal(missingFaceText("Lobster"), "Lobster is missing. Its characters cannot be shown.");
  assert.equal(noCharactersText("Mylines Regular"), "Mylines Regular has no characters to insert.");
  assert.equal(noMatchText("Great Vibes", " xyz "), 'No character in Great Vibes matches "xyz".');
  assert.equal(codePointText(0xe4), "U+00E4");
  assert.equal(codePointText(0x1f600), "U+1F600");
  assert.equal(cellName("LATIN SMALL LETTER A WITH DIAERESIS", 0xe4), "Latin small letter a with diaeresis, U+00E4");
  assert.equal(cellName(null, 0xe000), "U+E000");
  assert.equal(detailMeta(0xe4, "adieresis", "Latin-1 Supplement"), "U+00E4 · adieresis · Latin-1 Supplement");
  assert.equal(detailMeta(0xe4, null, "Latin-1 Supplement"), "U+00E4 · Latin-1 Supplement");
  assert.equal(emptyCellLabel("NBSP", 0xa0), "NBSP");
  assert.equal(emptyCellLabel(null, 0x2002), "2002");
  assert.equal(needsNames("heart"), true);
  assert.equal(needsNames("ä"), false);
  assert.equal(needsNames("U+00E4"), false);
});

test("a cell keeps its key when the page scrolls, so the focused cell is not remounted (criterion 20)", () => {
  const groups = [
    { name: "Basic Latin", cells: Array.from({ length: 40 }, (_, i) => 0x41 + i) },
    { name: "Arrows", cells: Array.from({ length: 40 }, (_, i) => 0x2190 + i) },
  ];
  const layout = layoutGlyphGrid(groups, 4, 40);
  const keysOf = (scrollTop: number) => {
    const page = glyphRowsInPage(layout, scrollTop, 200);
    return glyphPageItems(layout, page).flatMap((group) => group.items.map((item) => `${group.group}/${item.key}:${item.kind === "cell" ? item.index : "h"}`));
  };
  const top = keysOf(0);
  const scrolled = keysOf(250);
  // Cell 20 is in both pages, under the same key, although the first rows left the page.
  const cell20 = (keys: string[]) => keys.find((k) => k.endsWith(":20"));
  assert.ok(cell20(top) && cell20(scrolled));
  assert.equal(cell20(top), cell20(scrolled));
  assert.ok(!scrolled.some((k) => k.endsWith(":0")), "the first row left the page");
  for (const keys of [top, scrolled]) {
    assert.equal(new Set(keys.map((k) => k.split(":")[0])).size, keys.length, "keys unique per group");
  }
});

test("a click keeps the focus on the cell in full view, returns it to the text in split view (criterion 12)", () => {
  assert.equal(focusStaysOnCell(true, false), true);
  assert.equal(focusStaysOnCell(true, true), true);
  assert.equal(focusStaysOnCell(false, true), true);
  assert.equal(focusStaysOnCell(false, false), false);
});
