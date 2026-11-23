import assert from "node:assert/strict";
import { test } from "node:test";

import {
  type BrowserBackend,
  type RecoveryRules,
  RecoveryWriteError,
  browserRecoveryStore,
} from "../src/lib/recoveryTypes.ts";

/** An in-memory backend: copies by key, assets by "key/name", locks held by this "tab". */
function backend(others: Set<string> = new Set()) {
  const copies = new Map<string, { record: string; copy: Uint8Array }>();
  const assets = new Map<string, Uint8Array>();
  const mine = new Set<string>();
  const fake: BrowserBackend = {
    putCopy: async (key, record, copy) => void copies.set(key, { record, copy }),
    putAsset: async (key, name, bytes) => {
      if (!assets.has(`${key}/${name}`)) assets.set(`${key}/${name}`, bytes);
    },
    deleteCopy: async (key) => {
      copies.delete(key);
      for (const id of [...assets.keys()]) if (id.startsWith(`${key}/`)) assets.delete(id);
    },
    entries: async () =>
      [...copies].map(([key, { record, copy }]) => ({
        key,
        record,
        copy,
        assetBytes: [...assets].filter(([id]) => id.startsWith(`${key}/`)).reduce((n, [, b]) => n + b.byteLength, 0),
      })),
    assets: async (key) =>
      [...assets].filter(([id]) => id.startsWith(`${key}/`)).map(([id, bytes]) => ({ name: id.slice(key.length + 1), bytes })),
    hold: async (key) => {
      if (others.has(key)) return false;
      mine.add(key);
      return true;
    },
    release: (key) => void mine.delete(key),
    held: async () => new Set([...others, ...mine]),
  };
  return { fake, copies, assets, mine };
}

/** Rules double: the record text is `name|pages|step|problem`. */
const rules: RecoveryRules = {
  check: (record) => {
    const [name, pages, step, problem] = record.split("|");
    return {
      name: name === "" ? null : name,
      pages: Number(pages),
      last_step_ms: Number(step),
      problem: problem === "" ? null : (problem as "damaged" | "newer"),
    };
  },
  // The real plan is Rust's; here: the oldest unlocked beyond the newest one.
  prune: (candidates) =>
    candidates
      .filter((c) => !c.locked)
      .sort((a, b) => b.last_step_ms - a.last_step_ms)
      .slice(2)
      .map((c) => c.key),
};

const bytes = (n: number) => new Uint8Array(n);

/** The tab that wrote is closed: its locks are released, as a crash or a closed tab does. */
function gone(fake: BrowserBackend) {
  for (const key of ["a", "b", "k1", "k2", "k3", "k4"]) fake.release(key);
}

test("a copy written is listed after the tab is gone, newest first, with its page count", async () => {
  const { fake } = backend();
  const store = browserRecoveryStore(fake, rules);
  await store.writeCopy("a", "Old.curvyo|2|10|", bytes(5));
  await store.writeCopy("b", "|1|20|", bytes(5));
  gone(fake);
  const listing = await store.list();
  assert.equal(listing.path, null);
  assert.deepEqual(listing.rows.map((r) => [r.key, r.name, r.pages, r.changed]), [
    ["b", null, 1, false],
    ["a", "Old.curvyo", 2, false],
  ]);
});

test("a copy a running tab holds is not offered, and Restore of a held key is refused", async () => {
  const { fake } = backend(new Set(["live"]));
  const store = browserRecoveryStore(fake, rules);
  await store.writeCopy("live", "|1|5|", bytes(1));
  await store.writeCopy("gone", "|1|6|", bytes(1));
  fake.release("gone");
  assert.deepEqual((await store.list()).rows.map((r) => r.key), ["gone"]);
  await assert.rejects(store.begin("live"), (error: RecoveryWriteError) => error.code === "held");
  await store.begin("gone");
});

test("a damaged or newer copy is a row that cannot be restored", async () => {
  const { fake } = backend();
  const store = browserRecoveryStore(fake, rules);
  await store.writeCopy("a", "|0|1|damaged", bytes(1));
  await store.writeCopy("b", "|0|2|newer", bytes(1));
  gone(fake);
  assert.deepEqual((await store.list()).rows.map((r) => r.problem), ["newer", "damaged"]);
});

test("the oldest copies beyond the limit are pruned at start, silently", async () => {
  const { fake, copies } = backend();
  const store = browserRecoveryStore(fake, rules);
  for (const n of [1, 2, 3, 4]) await store.writeCopy(`k${n}`, `|1|${n}|`, bytes(1));
  gone(fake);
  assert.deepEqual((await store.list()).rows.map((r) => r.key), ["k4", "k3"]);
  assert.deepEqual([...copies.keys()].sort(), ["k3", "k4"]);
});

test("Save deletes the copy and its assets and keeps the lock; Discard also releases it", async () => {
  const { fake, copies, assets, mine } = backend();
  const store = browserRecoveryStore(fake, rules);
  await store.begin("a");
  await store.writeAsset("a", "x.png", bytes(3));
  await store.writeAsset("a", "x.png", bytes(9));
  assert.equal(assets.get("a/x.png")?.byteLength, 3, "an asset is written once");
  await store.writeCopy("a", "|1|1|", bytes(2));
  const read = await store.read("a");
  assert.equal(read?.copy.byteLength, 2);
  assert.deepEqual(read?.assets.map((a) => a.name), ["x.png"]);
  await store.deleteCopy("a");
  assert.equal(copies.size, 0);
  assert.equal(assets.size, 0);
  assert.ok(mine.has("a"));
  await store.end("a");
  assert.ok(!mine.has("a"));
  assert.equal(await store.read("a"), null);
});

test("a full browser store is the code the words know", async () => {
  const { fake } = backend();
  fake.putCopy = async () => {
    throw Object.assign(new Error("full"), { name: "QuotaExceededError" });
  };
  const store = browserRecoveryStore(fake, rules);
  await assert.rejects(store.writeCopy("a", "|1|1|", bytes(1)), (error: RecoveryWriteError) => error.code === "browser-full");
});
