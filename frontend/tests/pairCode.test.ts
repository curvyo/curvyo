// The pairing code field (`specs/0096-own-devices-sync` ux-notes 4.2).
import assert from "node:assert/strict";
import { test } from "node:test";

import { formatPairCode, pairCodeCharacters, pairCodeComplete } from "../src/lib/pairCode.ts";

test("typed codes are upper-cased, spaces and dashes dropped, shown in two groups", () => {
  assert.equal(pairCodeCharacters(" k7q2 - 9m4x "), "K7Q29M4X");
  assert.equal(formatPairCode("k7q29"), "K7Q2-9");
  assert.equal(formatPairCode("K7Q2"), "K7Q2");
  assert.equal(formatPairCode("k7q2-9m4x-extra"), "K7Q2-9M4X");
});

test("Connect waits for eight characters", () => {
  assert.equal(pairCodeComplete("K7Q2-9M4"), false);
  assert.equal(pairCodeComplete("k7q2 9m4x"), true);
});
