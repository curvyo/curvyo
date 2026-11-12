// The host write list's decoder against the Rust fixtures of `curvyo-editor-wasm/tests/fixtures/
// material_writes/` (`specs/0028-material-test-library` adrs.md §6). The Rust contract test
// `tests/material_test_library_view_contract.rs` builds the same steps and compares them with
// these files, so a drift fails on both sides.
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync, readdirSync } from "node:fs";
import { decodeMaterialWrite, decodeMaterialWrites } from "../src/lib/materialWrites.ts";

const DIR = new URL("../../curvyo-editor-wasm/tests/fixtures/material_writes/", import.meta.url);

function text(name: string): string {
  return readFileSync(new URL(`${name}.json`, DIR), "utf8");
}

test("the Rust write lists decode entry for entry", () => {
  const names = readdirSync(DIR)
    .filter((file) => file.endsWith(".json"))
    .map((file) => file.slice(0, -".json".length))
    .sort();
  assert.deepEqual(names, ["save_as_material", "undo_save_as_material"]);
  for (const name of names) {
    const raw = JSON.parse(text(name)) as unknown[];
    assert.equal(decodeMaterialWrites(text(name)).length, raw.length, name);
  }
  const saved = decodeMaterialWrites(text("save_as_material"));
  assert.ok(saved.some((write) => write.op === "write"));
  const undone = decodeMaterialWrites(text("undo_save_as_material"));
  assert.deepEqual(
    undone.map((write) => write.op),
    ["remove"],
  );
});

test("a write with a missing field or a wrong type is refused", () => {
  assert.equal(decodeMaterialWrite({ op: "write", id: "a" }), null);
  assert.equal(decodeMaterialWrite({ op: "remove", id: 3 }), null);
  assert.equal(decodeMaterialWrite({ op: "order" }), null);
  assert.equal(decodeMaterialWrite({ op: "rename", id: "a" }), null);
  assert.deepEqual(decodeMaterialWrite({ op: "grid_choice", text: "{}" }), {
    op: "grid_choice",
    text: "{}",
  });
  assert.deepEqual(decodeMaterialWrites("not json"), []);
  assert.deepEqual(decodeMaterialWrites("{}"), []);
});

import { failedWriteName, recordName } from "../src/lib/materialWrites.ts";

test("a failed write is named by its new text, else by the kept record", () => {
  const known = [{ name: "a.json", text: JSON.stringify({ name: "Kept name" }) }];
  assert.equal(recordName('{"name": "Birch 3 mm"}'), "Birch 3 mm");
  assert.equal(recordName("not json"), null);
  assert.equal(recordName('{"name": ""}'), null);
  assert.equal(failedWriteName({ op: "write", id: "a", text: '{"name":"New"}' }, known), "New");
  assert.equal(failedWriteName({ op: "write", id: "a", text: "x" }, known), "Kept name");
  assert.equal(failedWriteName({ op: "remove", id: "a" }, known), "Kept name");
  assert.equal(failedWriteName({ op: "remove", id: "b" }, known), null);
  assert.equal(failedWriteName({ op: "order", text: "{}" }, known), null);
});
