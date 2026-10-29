// What a drag over the canvas carries (`specs/0087-image-clipboard-and-drop` criteria 20, 21, 23).

import assert from "node:assert/strict";
import { test } from "node:test";

import { dragKind, dropContent } from "../src/lib/dropData.ts";

const file = (name: string, type: string) => new File([new Uint8Array([1, 2, 3])], name, { type });

test("files win, a link without files is a link, plain text is nothing", () => {
  assert.equal(dragKind(["Files", "text/uri-list"]), "files");
  assert.equal(dragKind(["text/uri-list", "text/plain"]), "link");
  assert.equal(dragKind(["text/html"]), "link");
  assert.equal(dragKind(["text/plain"]), null);
  assert.equal(dragKind([]), null);
});

test("dropped image data without a file name comes as a file item; the file list is the fallback", () => {
  const dragged = file("", "image/png");
  const items = [{ kind: "string", getAsFile: () => null }, { kind: "file", getAsFile: () => dragged }];
  const content = dropContent({ types: ["Files", "text/html"], files: [], items });
  assert.deepEqual(content, { kind: "files", files: [dragged] });
  const listed = file("a.png", "image/png");
  assert.deepEqual(dropContent({ types: ["Files"], files: [listed] }), { kind: "files", files: [listed] });
});

test("a drop of only a link is a link and fetches nothing", () => {
  const original = globalThis.fetch;
  let fetched = 0;
  globalThis.fetch = (() => {
    fetched += 1;
    return Promise.reject(new Error("no network"));
  }) as typeof fetch;
  try {
    assert.deepEqual(dropContent({ types: ["text/uri-list", "text/plain"], files: [] }), { kind: "link" });
    assert.deepEqual(dropContent({ types: ["text/html"], files: [] }), { kind: "link" });
    assert.deepEqual(dropContent({ types: ["Files", "text/uri-list"], files: [] }), { kind: "link" });
    assert.equal(dropContent({ types: ["text/plain"], files: [] }), null);
  } finally {
    globalThis.fetch = original;
  }
  assert.equal(fetched, 0);
});

test("on the desktop plain text may name local files, so it is taken like a link", () => {
  assert.equal(dragKind(["text/plain"], true), "link");
  assert.equal(dragKind(["text/plain"], false), null);
  assert.equal(dragKind(["Files", "text/plain"], true), "files");
});
