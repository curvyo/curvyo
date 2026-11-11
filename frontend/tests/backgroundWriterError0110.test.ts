import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import path from "node:path";
import { test } from "node:test";

test("background record errors reach the worker as a readable message", async () => {
  const bindings = await import("../src/wasm-bindings/curvyo_editor_wasm.js");
  bindings.initSync({ module: readFileSync(path.resolve(import.meta.dirname, "../src/wasm-bindings/curvyo_editor_wasm_bg.wasm")) });
  const writer = new bindings.WasmBackgroundWriter(9n);
  const document = "02020202020202020202020202020202";
  writer.add(document, 7, "container", new Uint8Array([1, 2, 3]));
  assert.equal(writer.create(document, "01010101010101010101010101010101", 0), undefined);
  assert.match(writer.take_error(), /remains queued/);
  assert.equal(writer.forget(document), 7);
  writer.free();
});
