import assert from "node:assert/strict";
import { test } from "node:test";

import { fontExtension, fontFileName } from "../src/lib/fontFileName.ts";

test("the extension follows the first bytes", () => {
  assert.equal(fontExtension(new TextEncoder().encode("OTTO....")), "otf");
  assert.equal(fontExtension(new TextEncoder().encode("ttcf....")), "ttc");
  assert.equal(fontExtension(new TextEncoder().encode("<svg ")), "svg");
  assert.equal(fontExtension(new Uint8Array([0, 1, 0, 0])), "ttf");
});

test("the file name drops characters a file cannot hold", () => {
  assert.equal(fontFileName({ family: "Sans/Serif", style: "Bold" }, new Uint8Array([0, 1, 0, 0])), "SansSerif-Bold.ttf");
});
