// Local files a paste or a drop names (`specs/0087-image-clipboard-and-drop` criteria 6 and 21,
// lead decision 2026-10-29): the host's answer becomes the import's files.

import assert from "node:assert/strict";
import { test } from "node:test";

import { importFilesOf } from "../src/lib/localFiles.ts";

test("the host's answer: bytes, a length over the limit, refused and skipped as no bytes", () => {
  const files = importFilesOf([
    { kind: "bytes", name: "a.png", bytes: [1, 2, 3] },
    { kind: "too_large", name: "huge.png", bytes: 146_800_640 },
    { kind: "refused", name: "folder" },
    { kind: "skipped", name: "21.png" },
  ]);
  assert.deepEqual(files, [
    { name: "a.png", bytes: new Uint8Array([1, 2, 3]) },
    { name: "huge.png", bytes: new Uint8Array(), tooLarge: 146_800_640 },
    { name: "folder", bytes: new Uint8Array() },
    { name: "21.png", bytes: new Uint8Array() },
  ]);
});
