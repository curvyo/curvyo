// The browser build's photo files (`specs/0112-test-grid-and-materials` criteria 27, 31, 33, ADR
// 0058 §1, §5): only `<record id>.<test id>.jpg|png` names reach the store, at most 1 MiB is kept,
// a missing photo is `not-found`. The IndexedDB store is replaced by a hand-written double.
import assert from "node:assert/strict";
import { test } from "node:test";

import {
  MAX_PHOTO_BYTES,
  type PhotoStore,
  browserPhotoCommands,
  isPhotoFileName,
} from "../src/lib/materialPhotoStore.ts";

const ID = "0b6c3f1e-2a4d-4b8e-9c7f-1d2e3f4a5b6c";

function memoryStore(): PhotoStore & { kept: Map<string, Uint8Array> } {
  const kept = new Map<string, Uint8Array>();
  return {
    kept,
    get: async (name) => kept.get(name) ?? null,
    put: async (name, bytes) => void kept.set(name, bytes),
    delete: async (name) => void kept.delete(name),
  };
}

test("only photo file names of a record id and a test id pass", () => {
  assert.ok(isPhotoFileName(`${ID}.M8W1O.png`));
  assert.ok(isPhotoFileName(`${ID}.7K2QX.jpg`));
  for (const bad of [
    `${ID}.json`,
    `${ID}.m8w1o.png`,
    `${ID}.M8W1.png`,
    `${ID}.M8W1O.gif`,
    `${ID}.M8W1O.jpeg`,
    `${ID}.M8W1O.png.json`,
    `${ID.toUpperCase()}.M8W1O.png`,
    `../${ID}.M8W1O.png`,
    "generic-laser.M8W1O.png",
    "",
  ]) {
    assert.equal(isPhotoFileName(bad), false, bad);
  }
});

test("a photo is written, read back and removed under its file name", async () => {
  const store = memoryStore();
  const commands = browserPhotoCommands(store);
  const name = `${ID}.M8W1O.png`;
  await assert.rejects(commands.read_material_photo({ name }), /not-found/);
  await commands.write_material_photo({ name, bytes: new Uint8Array([1, 2, 3]) });
  const read = await commands.read_material_photo({ name });
  assert.ok(read instanceof ArrayBuffer);
  assert.deepEqual([...new Uint8Array(read)], [1, 2, 3]);
  await commands.remove_material_photo({ name });
  await commands.remove_material_photo({ name });
  await assert.rejects(commands.read_material_photo({ name }), /not-found/);
});

test("a bad name or more than 1 MiB is refused and nothing is kept", async () => {
  const store = memoryStore();
  const commands = browserPhotoCommands(store);
  await assert.rejects(
    commands.write_material_photo({ name: "../x.png", bytes: new Uint8Array([1]) }),
    /not a photo file name/,
  );
  await assert.rejects(commands.read_material_photo({ name: `${ID}.json` }), /not a photo file name/);
  await assert.rejects(commands.remove_material_photo({ name: "x" }), /not a photo file name/);
  await assert.rejects(
    commands.write_material_photo({
      name: `${ID}.M8W1O.jpg`,
      bytes: new Uint8Array(MAX_PHOTO_BYTES + 1),
    }),
    /it is over 1 MiB/,
  );
  assert.equal(store.kept.size, 0);
});
