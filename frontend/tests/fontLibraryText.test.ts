// The words of the font library (`specs/0075-font-library` criteria 3, 7, 8, 10, 22, 25, 30).
import assert from "node:assert/strict";
import { test } from "node:test";

import {
  addNotice,
  chipTooltip,
  armedTagLabel,
  emptyTagText,
  tagNameError,
  armedRemoveLabel,
  STORAGE_FULL_NOTICE,
} from "../src/lib/fontLibraryText.ts";

test("criterion 22: the Add notice counts, names the refused and present files, and picks its look", () => {
  assert.deepEqual(addNotice({ added: ["a", "b", "c"], present: [], notFonts: [] }, null), {
    text: "3 fonts added.",
    kind: "success",
  });
  assert.equal(
    addNotice({ added: ["a.ttf"], present: ["b.ttf", "c.ttf"], notFonts: [] }, null).text,
    "1 font added. 2 were already there. Already there: b.ttf, c.ttf.",
  );
  assert.deepEqual(addNotice({ added: [], present: [], notFonts: ["Lobster.ttf"] }, null), {
    text: "Lobster.ttf is not a font and was not added.",
    kind: "refusal",
  });
  assert.equal(
    addNotice({ added: [], present: [], notFonts: ["notes.pdf", "logo.svg"] }, null).text,
    "2 files are not fonts and were not added. Not fonts: notes.pdf, logo.svg.",
  );
  const many = Array.from({ length: 9 }, (_, i) => `a-rather-long-file-name-${i}.pdf`);
  assert.match(addNotice({ added: [], present: [], notFonts: many }, null).text, /and 7 more\.$/);
});

test("criterion 23: fonts added with an own tag chosen say so", () => {
  assert.equal(addNotice({ added: ["a", "b", "c"], present: [], notFonts: [] }, "Wedding").text, "3 fonts added to Wedding.");
});

test("criteria 7, 8, 25, 30: names, prompts and the storage refusal", () => {
  assert.equal(tagNameError("taken", "Script"), "A tag named Script exists.");
  assert.equal(tagNameError("reserved"), "All and Favourites are not tags.");
  assert.equal(tagNameError("empty"), "Enter a name.");
  assert.equal(tagNameError("long"), "Use at most 24 characters.");
  assert.equal(armedTagLabel("Wedding", 12), "Press again to delete tag Wedding. Its 12 fonts stay in Curvyo.");
  assert.equal(armedRemoveLabel("Lobster", 2), "Press again to remove Lobster from Curvyo fonts. 2 texts here use it.");
  assert.equal(STORAGE_FULL_NOTICE, "The browser has no room for this font. Nothing was added.");
});

test("criteria 3 and 10: empty strip cells and tag sources", () => {
  assert.equal(emptyTagText("Cuttable", "", false), "No font in Cuttable.");
  assert.equal(emptyTagText("Cuttable", " vibs ", false), 'No font in Cuttable matches "vibs".');
  assert.equal(emptyTagText("Cuttable", "", true), "No font in Cuttable yet.");
  assert.equal(chipTooltip("detected:1"), "Detected: one piece");
  assert.equal(chipTooltip("declared:script"), "Declared by the font: script");
  assert.equal(chipTooltip("shipped"), "Set for the included fonts");
  assert.equal(chipTooltip("you"), "Added by you");
  assert.equal(chipTooltip(""), null);
});
