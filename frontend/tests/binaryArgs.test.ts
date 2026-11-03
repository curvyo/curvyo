import assert from "node:assert/strict";
import { test } from "node:test";

import { answerBytes, asciiJson } from "../src/lib/binaryArgs.ts";

test("arguments of a binary call are ASCII JSON that reads back unchanged", () => {
  const args = { path: ["Deckel", "Grüße 😀.curvyo"], note: "a\nb" };
  const header = asciiJson(args);
  assert.match(header, /^[\x20-\x7e]*$/);
  assert.deepEqual(JSON.parse(header), args);
});

test("a raw answer becomes bytes without a number array", () => {
  const buffer = new Uint8Array([1, 2, 255]).buffer;
  assert.deepEqual(Array.from(answerBytes(buffer)), [1, 2, 255]);
  assert.deepEqual(Array.from(answerBytes([4, 5])), [4, 5]);
});
