import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test } from "node:test";

import {
  type GroupFacts,
  isSwitchOff,
  parseEnded,
} from "../src/lib/endedRecord.ts";

const golden = readFileSync(
  new URL(
    "../../curvyo-document-core/tests/fixtures/share_ended_v1.json",
    import.meta.url,
  ),
);

test("criterion 20: the golden share.ended.json v1 names its server, room and why it ended", () => {
  assert.deepEqual(parseEnded(golden), {
    server: "ws://192.168.1.20:8787",
    room: "07070707070707070707070707070707",
    left: false,
  });
});

test("a newer or damaged record is no way back", () => {
  const newer = readFileSync(
    new URL(
      "../../curvyo-document-core/tests/fixtures/share_ended_v2.json",
      import.meta.url,
    ),
  );
  assert.equal(parseEnded(newer), null);
  assert.equal(parseEnded(new TextEncoder().encode("{oops")), null);
});

const group: GroupFacts = {
  listed: new Set(["w1", "w2"]),
  switchedOn: new Set(["w1"]),
  members: new Set(["aa", "a2"]),
};

test("a removal signed by one of my devices is a switch-off", () => {
  assert.equal(isSwitchOff(group, "w1", "a2", false), true);
});

test("a removal signed by another maker's admin keeps the way back (specs/0097 criterion 20)", () => {
  assert.equal(isSwitchOff(group, "w1", "cc", false), false);
});

test("without the signer the list's switch decides", () => {
  assert.equal(isSwitchOff(group, "w1", "", false), false);
  assert.equal(isSwitchOff(group, "w2", "", false), true);
});

test("a leave or a workspace outside the list is never a switch-off", () => {
  assert.equal(isSwitchOff(group, "w2", "aa", true), false);
  assert.equal(isSwitchOff(group, "w3", "aa", false), false);
});
