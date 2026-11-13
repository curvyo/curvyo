// The material writes a session hands out, performed through a host double (`specs/0112-test-
// grid-and-materials` criteria 17, 25, 27, 31, ADR 0058 §4, §5): photo bytes are taken by handle
// and travel as bytes, the retired file is written, a record over 512 KiB is refused before the
// host, a refused photo keeps the record and names its own reason, a missing photo is "missing".
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test } from "node:test";

import {
  type HostInvoke,
  MAX_RECORD_TEXT,
  photoFailureText,
  readMaterialPhoto,
  recordTextRefusal,
  runMaterialWrites,
} from "../src/lib/materialHostWrites.ts";
import { decodeMaterialWrites } from "../src/lib/materialWrites.ts";

const DIR = new URL("../../curvyo-editor-wasm/tests/fixtures/material_writes/", import.meta.url);
const fixture = (name: string) => decodeMaterialWrites(readFileSync(new URL(`${name}.json`, DIR), "utf8"));

interface Call {
  command: string;
  args?: Record<string, unknown>;
  bytes?: Uint8Array;
}

function host(refuse: (call: Call) => string | null = () => null): { invoke: HostInvoke; calls: Call[] } {
  const calls: Call[] = [];
  const invoke: HostInvoke = (command, args, bytes) => {
    const call = { command, args, bytes };
    calls.push(call);
    const reason = refuse(call);
    return reason === null ? Promise.resolve(undefined) : Promise.reject(reason);
  };
  return { invoke, calls };
}

test("a photo write takes its bytes by handle and sends them as bytes after the record", async () => {
  const { invoke, calls } = host();
  const taken: number[] = [];
  const result = await runMaterialWrites(fixture("photo_write"), invoke, (handle) => {
    taken.push(handle);
    return new Uint8Array([9, 8, 7]);
  }, []);
  assert.deepEqual(result, { record: null, photo: null });
  assert.deepEqual(taken, [0]);
  assert.deepEqual(calls.map((call) => call.command), ["write_material", "write_material_photo"]);
  assert.deepEqual(calls[1].args, { name: "03030303-0303-4303-8303-030303030303.M8W1O.png" });
  assert.deepEqual([...(calls[1].bytes ?? [])], [9, 8, 7]);
});

test("deleting a material removes the record and the photo and writes the retired ids", async () => {
  const { invoke, calls } = host();
  const result = await runMaterialWrites(fixture("delete_material_retires"), invoke, () => new Uint8Array(), []);
  assert.deepEqual(result, { record: null, photo: null });
  assert.deepEqual(calls.map((call) => call.command), [
    "remove_material",
    "remove_material_photo",
    "write_retired_test_ids",
  ]);
  assert.deepEqual(calls[1].args, { name: "03030303-0303-4303-8303-030303030303.M8W1O.png" });
  assert.match(String(calls[2].args?.text), /"M8W1O"/);
});

test("a photo the host refuses keeps the record and gives the photo's own reason", async () => {
  const { invoke } = host((call) => (call.command === "write_material_photo" ? "the disk is full" : null));
  const result = await runMaterialWrites(fixture("photo_write"), invoke, () => new Uint8Array([1]), []);
  assert.deepEqual(result, { record: null, photo: "the disk is full" });
  assert.equal(photoFailureText("the disk is full"), "Could not save the photo: the disk is full.");
  const thrown: HostInvoke = (command) =>
    command === "write_material_photo" ? Promise.reject(new Error("it is over 1 MiB")) : Promise.resolve();
  const refused = await runMaterialWrites(fixture("photo_write"), thrown, () => new Uint8Array([1]), []);
  assert.equal(refused.photo, "it is over 1 MiB", "an Error gives its message");
});

test("photo bytes the session no longer holds are a refused photo, not an empty file", async () => {
  const { invoke, calls } = host();
  const result = await runMaterialWrites(fixture("photo_write"), invoke, () => new Uint8Array(), []);
  assert.deepEqual(result, { record: null, photo: "its bytes are gone" });
  assert.deepEqual(calls.map((call) => call.command), ["write_material"]);
});

test("a record over 512 KiB is refused before the host and named", async () => {
  const { invoke, calls } = host();
  const text = JSON.stringify({ name: "Birch", pad: "x".repeat(MAX_RECORD_TEXT) });
  assert.equal(recordTextRefusal(text), "it is over 512 KiB");
  assert.equal(recordTextRefusal("é".repeat(MAX_RECORD_TEXT / 2)), null, "exactly 512 KiB in UTF-8");
  assert.equal(recordTextRefusal("é".repeat(MAX_RECORD_TEXT / 2) + "x"), "it is over 512 KiB");
  const result = await runMaterialWrites([{ op: "write", id: "a", text }], invoke, () => new Uint8Array(), []);
  assert.deepEqual(result, { record: { reason: "it is over 512 KiB", name: "Birch" }, photo: null });
  assert.deepEqual(calls, []);
});

test("a refused record write is the record failure, a refused retired file is silent", async () => {
  const { invoke, calls } = host((call) => (call.command === "remove_photo" ? null : "no room"));
  const known = [{ name: "03030303-0303-4303-8303-030303030303.json", text: '{"name":"Birch 3 mm"}' }];
  const result = await runMaterialWrites(fixture("delete_material_retires"), invoke, () => new Uint8Array(), known);
  assert.deepEqual(result.record, { reason: "no room", name: "Birch 3 mm" });
  assert.equal(calls.length, 3, "every write is still tried");
});

test("a photo that is missing or unreadable reads as missing", async () => {
  const bytes = new Uint8Array([0x89, 0x50]).buffer;
  const found: HostInvoke = (command, args) =>
    command === "read_material_photo" && args?.name === "a.M8W1O.png"
      ? Promise.resolve(bytes)
      : Promise.reject("not-found");
  assert.deepEqual([...((await readMaterialPhoto(found, "a.M8W1O.png")) as Uint8Array)], [0x89, 0x50]);
  assert.equal(await readMaterialPhoto(found, "b.M8W1O.png"), "missing");
  const empty: HostInvoke = () => Promise.resolve(new ArrayBuffer(0));
  assert.equal(await readMaterialPhoto(empty, "a.M8W1O.png"), "missing");
  const odd: HostInvoke = () => Promise.resolve("text");
  assert.equal(await readMaterialPhoto(odd, "a.M8W1O.png"), "missing");
});
