// The host write list's decoder against the Rust fixtures of `curvyo-editor-wasm/tests/fixtures/
// material_writes/` (`specs/0112-test-grid-and-materials` adrs.md Finding 3). The Rust contract
// test `tests/test_grid_materials_view_contract.rs` builds the same steps and compares them with
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
  assert.deepEqual(names, ["delete_material_retires", "judge", "photo_remove", "photo_write"]);
  const ops = (name: string) => decodeMaterialWrites(text(name)).map((write) => write.op);
  for (const name of names) {
    const raw = JSON.parse(text(name)) as unknown[];
    assert.equal(decodeMaterialWrites(text(name)).length, raw.length, name);
  }
  assert.deepEqual(ops("judge"), ["write"]);
  assert.deepEqual(ops("photo_write"), ["write", "write_photo"]);
  assert.deepEqual(ops("photo_remove"), ["write", "remove_photo"]);
  assert.deepEqual(ops("delete_material_retires"), ["remove", "remove_photo", "retired"]);
  const photo = decodeMaterialWrites(text("photo_write"))[1];
  assert.deepEqual(photo, {
    op: "write_photo",
    name: "03030303-0303-4303-8303-030303030303.M8W1O.png",
    handle: 0,
  });
});

test("a write with a missing field or a wrong type is refused", () => {
  assert.equal(decodeMaterialWrite({ op: "write", id: "a" }), null);
  assert.equal(decodeMaterialWrite({ op: "remove", id: 3 }), null);
  assert.equal(decodeMaterialWrite({ op: "order" }), null);
  assert.equal(decodeMaterialWrite({ op: "rename", id: "a" }), null);
  assert.equal(decodeMaterialWrite({ op: "retired" }), null);
  assert.equal(decodeMaterialWrite({ op: "write_photo", name: "a.M8W1O.png" }), null);
  assert.equal(decodeMaterialWrite({ op: "write_photo", name: "a", handle: "0" }), null);
  assert.equal(decodeMaterialWrite({ op: "write_photo", name: "a", handle: -1 }), null);
  assert.equal(decodeMaterialWrite({ op: "write_photo", name: "a", handle: 1.5 }), null);
  assert.equal(decodeMaterialWrite({ op: "remove_photo", name: 7 }), null);
  assert.deepEqual(decodeMaterialWrite({ op: "remove_photo", name: "a" }), { op: "remove_photo", name: "a" });
  assert.deepEqual(decodeMaterialWrite({ op: "retired", text: "{}" }), { op: "retired", text: "{}" });
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
