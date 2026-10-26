// The words of the font info view and the archive notices (`specs/0083-font-info-and-zip`
// criteria 2, 9, 13 and 14).
import assert from "node:assert/strict";
import { test } from "node:test";

import { addNoticeWithArchives, freeFolderName } from "../src/lib/fontArchiveText.ts";
import {
  blockText,
  documentsGoing,
  embeddingText,
  fileLine,
  formatText,
  licenceName,
  sizeText,
} from "../src/lib/fontInfoText.ts";
import { armedRemoveLabel, type ArchiveResult } from "../src/lib/fontLibraryText.ts";

test("criterion 2: fsType in words, the least restrictive usage bit, the extra bits", () => {
  assert.equal(embeddingText(0, false), "Installable");
  assert.equal(embeddingText(0x0002, false), "Restricted: may not be embedded");
  assert.equal(embeddingText(0x0004, false), "Preview and print");
  assert.equal(embeddingText(0x0008, false), "Editable");
  assert.equal(embeddingText(0x0108, false), "Editable, no subsetting");
  assert.equal(embeddingText(0x0304, false), "Preview and print, no subsetting, bitmap only");
  assert.equal(embeddingText(0x000c, false), "Editable");
  assert.equal(embeddingText(null, false), "Not stated");
  assert.equal(embeddingText(null, true), "Not stated (SVG font)");
});

test("criterion 2: formats, file line, blocks, licence names", () => {
  assert.equal(formatText({ kind: "truetype" }), "TrueType (TTF)");
  assert.equal(formatText({ kind: "cff" }), "OpenType CFF (OTF)");
  assert.equal(formatText({ kind: "cff2" }), "OpenType CFF2");
  assert.equal(formatText({ kind: "collection", face: 2, faces: 4 }), "TrueType collection (TTC), face 2 of 4");
  assert.equal(formatText({ kind: "svg" }), "SVG font, single-line");
  assert.equal(fileLine("Lobster-Regular.ttf", 412 * 1024), "Lobster-Regular.ttf · 412 KB");
  assert.equal(sizeText(812), "812 bytes");
  assert.equal(sizeText(1.25 * 1024 * 1024), "1.3 MB");
  assert.deepEqual(
    [blockText("curvyo", true), blockText("curvyo", false), blockText("included", true), blockText("system", true)],
    ["Curvyo fonts", "Browser storage", "Included", "This computer"],
  );
  assert.equal(licenceName("OFL-1.1"), "SIL Open Font License 1.1");
  assert.equal(licenceName("Apache-2.0"), "Apache License 2.0");
  assert.equal(licenceName("Hershey"), "Hershey notice");
});

const archive = (over: Partial<ArchiveResult>): ArchiveResult => ({
  name: "Lobster.zip",
  added: 0,
  present: 0,
  documents: 0,
  unsafeEntries: 0,
  refused: null,
  ...over,
});
const only = (a: ArchiveResult) => addNoticeWithArchives({ added: [], present: [], notFonts: [], archives: [a] }, null).text;

test("criterion 13: one archive's notices", () => {
  assert.equal(only(archive({ added: 2, documents: 3 })), "Lobster.zip: 2 fonts added, 3 documents kept.");
  assert.equal(only(archive({ present: 2 })), "Lobster.zip: 2 fonts were already there.");
  assert.equal(only(archive({})), "Lobster.zip holds no font. Nothing was added.");
  assert.equal(only(archive({ name: "Pack.zip", refused: "unreadable" })), "Pack.zip could not be read. Nothing was added.");
  assert.equal(only(archive({ name: "Pack.zip", refused: "over_limit" })), "Pack.zip is over the size limit. Nothing was added.");
  assert.equal(
    only(archive({ added: 1, documents: 1, unsafeEntries: 2 })),
    "Lobster.zip: 1 font added, 1 document kept. 2 files were skipped as unsafe.",
  );
});

test("criterion 13: several archives and loose files are summed", () => {
  const notice = addNoticeWithArchives(
    {
      added: ["a.ttf"],
      present: [],
      notFonts: [],
      archives: [archive({ added: 2, documents: 3 }), archive({ name: "B.zip", added: 2, documents: 1 }), archive({ name: "Pack.zip", refused: "unreadable" })],
    },
    null,
  );
  assert.equal(notice.text, "5 fonts added. 4 documents kept. Pack.zip could not be read.");
  assert.equal(notice.kind, "success");
  for (const part of notice.text.split(". ")) {
    assert.ok(part.length <= 70);
  }
});

test("criterion 18: an archive the store could not write gets the failure notice", () => {
  assert.equal(only(archive({ refused: "not_written" })), "Lobster.zip could not be added.");
  const two = addNoticeWithArchives(
    { added: [], present: [], notFonts: [], archives: [archive({ refused: "not_written" }), archive({ name: "B.zip", refused: "not_written" })] },
    null,
  );
  assert.equal(two.text, "Lobster.zip and 1 more could not be added.");
  assert.equal(two.kind, "refusal");
});

test("criterion 9: a taken folder name gets -2, by lower case (the Rust table)", () => {
  assert.equal(freeFolderName("Lobster", ["lobster"]), "Lobster-2");
  assert.equal(freeFolderName("Pack.v2", ["pack.V2"]), "Pack.v2-2");
  assert.equal(freeFolderName("New", ["Old"]), "New");
});

test("criterion 14: the documents sentence when the folder's last font goes", () => {
  const faces = [
    { family: "Lobster", folder: "Lobster" },
    { family: "Lobster", folder: "Lobster" },
    { family: "Pacifico", folder: "Pack" },
    { family: "Lobster Two", folder: "Pack" },
    { family: "Loose", folder: null },
  ];
  const folders = [
    { name: "Lobster", documents: [1, 2, 3] },
    { name: "Pack", documents: [1] },
  ];
  assert.equal(documentsGoing("Lobster", faces, folders), 3);
  assert.equal(documentsGoing("Pacifico", faces, folders), 0);
  assert.equal(documentsGoing("Loose", faces, folders), 0);
  assert.equal(
    armedRemoveLabel("Lobster", 2, 3),
    "Press again to remove Lobster from Curvyo fonts. Its 3 documents go too. 2 texts here use it.",
  );
  assert.equal(armedRemoveLabel("Lobster", 0, 1), "Press again to remove Lobster from Curvyo fonts. Its 1 document goes too.");
  assert.equal(armedRemoveLabel("Lobster", 0), "Press again to remove Lobster from Curvyo fonts.");
});

test("criterion 1: the pane reopens in Info mode", async () => {
  const { readPreviewPrefs } = await import("../src/lib/fontPreviewState.ts");
  const store = { getItem: () => JSON.stringify({ sample: "a", size: 32, full: false, mode: "info" }), setItem: () => {} };
  assert.equal(readPreviewPrefs(store).mode, "info");
});
