// The browser's key records (`specs/0096-own-devices-sync` criteria 31, 34, ADR 0040 decision 9):
// the version 1 record against its golden file, the name bound to the record, and the default
// name of a browser device (criterion 33).
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test } from "node:test";

import { browserDeviceName } from "../src/lib/browserDevice.ts";
import {
  type DeviceKeyStore,
  UnreadableKeyRecord,
  openRecord,
  readDeviceKeys,
  sealRecord,
  writeRoomKeys,
} from "../src/lib/deviceKeyStore.ts";

/** A store in memory with IndexedDB's `add` rule; `slow` makes every call yield first, so two
 * first runs interleave. */
function memoryStore(kept = new Map<string, Uint8Array>()): DeviceKeyStore {
  const yieldNow = () => new Promise((resolve) => setTimeout(resolve, 0));
  return {
    names: async () => [...kept.keys()],
    load: async (name) => {
      await yieldNow();
      return kept.get(name) ?? null;
    },
    save: async (name, value) => void kept.set(name, value),
    add: async (name, value) => {
      await yieldNow();
      if (kept.has(name)) {
        return false;
      }
      kept.set(name, value);
      return true;
    },
    remove: async (name) => void kept.delete(name),
  };
}

const fixture = JSON.parse(readFileSync(new URL("./fixtures/key_record_v1.json", import.meta.url), "utf8")) as {
  key: string;
  record: { format_version: number; name: string; iv: string; ciphertext: string };
  plaintext: string;
};

const bytes = (hex: string) => Uint8Array.from(Buffer.from(hex, "hex"));

async function testKey(): Promise<CryptoKey> {
  return crypto.subtle.importKey("raw", bytes(fixture.key), "AES-GCM", false, ["encrypt", "decrypt"]);
}

test("a version 1 key record seals and opens as its golden file says", async () => {
  const key = await testKey();
  const sealed = await sealRecord(key, fixture.record.name, bytes(fixture.plaintext), bytes(fixture.record.iv));
  assert.equal(sealed.format_version, 1);
  assert.equal(Buffer.from(sealed.ciphertext).toString("hex"), fixture.record.ciphertext);
  const record = {
    format_version: fixture.record.format_version,
    name: fixture.record.name,
    iv: bytes(fixture.record.iv),
    ciphertext: bytes(fixture.record.ciphertext),
  };
  const opened = await openRecord(key, record);
  assert.equal(Buffer.from(opened ?? new Uint8Array()).toString("hex"), fixture.plaintext);
});

test("a record moved to another name, a newer format or another key does not open", async () => {
  const key = await testKey();
  const record = {
    format_version: 1,
    name: fixture.record.name,
    iv: bytes(fixture.record.iv),
    ciphertext: bytes(fixture.record.ciphertext),
  };
  assert.equal(await openRecord(key, { ...record, name: "identity" }), null);
  assert.equal(await openRecord(key, { ...record, format_version: 2 }), null);
  const other = await crypto.subtle.generateKey({ name: "AES-GCM", length: 256 }, false, ["encrypt", "decrypt"]);
  assert.equal(await openRecord(other, record), null);
});

test("the identity is made once and kept; room keys read back", async () => {
  const store = memoryStore();
  const first = await readDeviceKeys(store, "");
  assert.equal(first.status, "ok");
  if (first.status !== "ok") {
    return;
  }
  assert.equal(first.identity.length, 128);
  assert.deepEqual(first.keys, []);
  await writeRoomKeys(store, "ab", [[0, "11".repeat(32)]]);
  const again = await readDeviceKeys(store, "ab");
  assert.deepEqual(again, { status: "ok", identity: first.identity, keys: [[0, "11".repeat(32)]] });
  // A backup reads every record; a restore into an empty store keeps the same device.
  const restored = new Map<string, Uint8Array>();
  for (const name of await store.names()) {
    restored.set(name, (await store.load(name)) ?? new Uint8Array());
  }
  const empty = memoryStore(restored);
  assert.deepEqual(await readDeviceKeys(empty, "ab"), again);
});

test("an identity record that does not open is refused, never replaced", async () => {
  const writes: string[] = [];
  const store: DeviceKeyStore = {
    names: async () => ["identity"],
    load: async (name) => {
      throw new UnreadableKeyRecord(name);
    },
    save: async (name) => void writes.push(`save ${name}`),
    add: async (name) => {
      writes.push(`add ${name}`);
      return false;
    },
    remove: async (name) => void writes.push(`remove ${name}`),
  };
  const answer = await readDeviceKeys(store, "ab");
  assert.equal(answer.status, "failed");
  assert.match(answer.status === "failed" ? answer.message : "", /cannot be read/);
  assert.deepEqual(writes, []);
});

test("two first runs at once keep one identity", async () => {
  const store = memoryStore();
  const [one, two] = await Promise.all([readDeviceKeys(store, ""), readDeviceKeys(store, "")]);
  assert.equal(one.status, "ok");
  assert.deepEqual(one, two);
});

test("a browser device is named after the browser and the system", () => {
  const firefox = "Mozilla/5.0 (X11; Linux x86_64; rv:140.0) Gecko/20100101 Firefox/140.0";
  const chrome = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/140.0 Safari/537.36";
  const safari = "Mozilla/5.0 (Macintosh; Intel Mac OS X 15_0) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/18.0 Safari/605.1.15";
  assert.equal(browserDeviceName(firefox), "Firefox on Linux");
  assert.equal(browserDeviceName(chrome), "Chrome on Windows");
  assert.equal(browserDeviceName(safari), "Safari on macOS");
  assert.equal(browserDeviceName("curl/8"), "Browser");
});
