import assert from "node:assert/strict";
import { test } from "node:test";

import {
  EMPTY_FONTS,
  blockCount,
  fontBlocks,
  listsOnToolChange,
  parseSize,
  sizeNotice,
  sizeText,
  textRefusalText,
} from "../src/lib/textText.ts";

test("criteria 24 and 29: refusal sentences for texts", () => {
  assert.equal(
    textRefusalText({ kind: "missing_fonts", count: 1, face: "Source Serif 4 Bold" }),
    "Font Source Serif 4 Bold is missing. Nothing was changed.",
  );
  assert.equal(
    textRefusalText({ kind: "missing_fonts", count: 2 }),
    "2 texts use missing fonts. Nothing was changed.",
  );
  assert.equal(textRefusalText({ kind: "groups", count: 1 }), null);
  const long = textRefusalText({ kind: "missing_fonts", count: 1, face: "A".repeat(40) }) ?? "";
  assert.ok(long.length <= 70);
});

test("criterion 12: size notices, typed units and the shown value", () => {
  assert.equal(sizeNotice("max"), "Size set to 2000 mm, the largest size.");
  assert.equal(sizeNotice("min"), "Size set to 0.1 mm, the smallest size.");
  assert.equal(parseSize("2 cm"), 20);
  assert.ok(Math.abs((parseSize("72pt") ?? 0) - 25.4) < 1e-9);
  assert.equal(parseSize("abc"), null);
  assert.equal(sizeText(12.5), "12.5");
  assert.equal(sizeText(20), "20");
  assert.equal(sizeText(null), "");
});

test("criteria 16 and 17: blocks in order, A to Z, filtered by a substring", () => {
  const face = (index: number, block: "curvyo" | "included" | "system", family: string, style: string) => ({
    index,
    block,
    family,
    style,
    id: String(index),
    faceIndex: 0,
    loaded: false,
  });
  const data = {
    ...EMPTY_FONTS,
    faces: [
      face(0, "included", "source serif 4", "Bold"),
      face(1, "included", "Great Vibes", "Regular"),
      face(2, "system", "Arial", "Regular"),
      face(3, "included", "source serif 4", "Regular"),
      face(4, "curvyo", "Great Vibes", "Regular"),
    ],
    systemRead: true,
  };
  const blocks = fontBlocks(data, "", true);
  assert.deepEqual(
    blocks.map((b) => b.title),
    ["Curvyo fonts", "Included", "This computer"],
  );
  assert.deepEqual(
    blocks[1].families.map((f) => f.family),
    ["Great Vibes", "source serif 4"],
  );
  assert.equal(blocks[1].families[1].faces[0].style, "Regular");
  const found = fontBlocks(data, "VIBES", true);
  assert.deepEqual(found.map((b) => b.title), ["Curvyo fonts", "Included"]);
  assert.equal(blockCount(found[1], true, 0), "1 of 2");
  assert.equal(blockCount(blocks[0], false, 2), "1 · 2 skipped");
  assert.deepEqual(
    fontBlocks({ ...data, faces: [], systemRead: false }, "", true).map((b) => b.title),
    ["This computer"],
  );
  assert.equal(fontBlocks({ ...data, faces: [] }, "", false).length, 0);
});

test("criterion 17: the search filters 2,000 families in under 100 ms", () => {
  const faces = Array.from({ length: 2000 }, (_, i) => ({
    index: i,
    block: "system" as const,
    family: `Family ${i}`,
    style: "Regular",
    id: String(i),
    faceIndex: 0,
    loaded: false,
  }));
  const started = performance.now();
  const blocks = fontBlocks({ ...EMPTY_FONTS, faces, systemRead: true }, "ily 19", true);
  assert.ok(performance.now() - started < 100);
  assert.ok(blocks[0].families.length > 0);
});

test("criteria 19 and 20: the host fonts are listed on entering the Text tool, not on each sync", () => {
  assert.equal(listsOnToolChange("select", "text"), true);
  assert.equal(listsOnToolChange(null, "text"), true);
  assert.equal(listsOnToolChange("text", "text"), false);
  assert.equal(listsOnToolChange("text", "select"), false);
});
