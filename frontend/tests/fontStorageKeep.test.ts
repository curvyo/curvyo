// Frage 13 B (`specs/0075-font-library`): the first browser Add asks for persistent storage once,
// and a refusal or a missing call does not stop it.
import assert from "node:assert/strict";
import { test } from "node:test";

import { requestFontStorageKeep } from "../src/lib/fontStorageKeep.ts";

test("the first Add asks once for persistent storage, a refusal changes nothing", () => {
  let calls = 0;
  const storage = { persist: () => { calls += 1; return Promise.resolve(false); } } as unknown as StorageManager;
  requestFontStorageKeep(storage);
  requestFontStorageKeep(storage);
  assert.equal(calls, 1);
});
