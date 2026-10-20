// The tag line of `specs/0075-font-library` (UX tags amendment §2 and §3): chips that do not fit
// go into "+N" with room for Add tag; an expanded row wraps every chip, Add tag last.
import assert from "node:assert/strict";
import { test } from "node:test";

import { shownTags, tagLineRoom, tagLines } from "../src/lib/fontRows.ts";

/** 6 px per character, so a chip is 26 px plus 6 per letter. */
const measure = (text: string) => text.length * 6;
const tags = ["Cuttable", "Script", "Wedding", "Signs"].map((name) => ({ id: name, name, source: "" }));

test("a one-line tag line keeps room for +N and Add tag", () => {
  const room = tagLineRoom(244 - 8);
  assert.equal(room, 200);
  // Cuttable 74, Script 62, Wedding 68: two fit before "+2" (26 px) and Add tag (20 px).
  assert.equal(shownTags(tags, room, measure), 2);
  assert.equal(shownTags(tags.slice(0, 2), room, measure), 2);
});

test("an expanded row wraps every chip and Add tag", () => {
  assert.equal(tagLines(tags, 200, measure), 2);
  assert.equal(tagLines([], 200, measure), 1);
});
