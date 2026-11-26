import assert from "node:assert/strict";
import { test } from "node:test";

import {
  BRIDGE_KEYS,
  BRIDGE_RULE,
  DEFAULT_BRIDGE_SETTINGS,
  REACH_RANGE,
  WIDTH_RANGE,
  bridgeRefusalOutlines,
  bridgeRefusalText,
  bridgeTooltipNote,
  clampField,
  readBridgeSettings,
  writeBridgeSettings,
} from "../src/lib/bridgeText.ts";
import { BOOLEAN_CARD_COMMANDS, booleanCardCommands } from "../src/lib/stackText.ts";

function memory(): { getItem(key: string): string | null; setItem(key: string, value: string): void } {
  const map = new Map<string, string>();
  return {
    getItem: (key) => map.get(key) ?? null,
    setItem: (key, value) => void map.set(key, value),
  };
}

test("criteria 1 and 2: Bridge follows Weld on a laser page and is absent elsewhere", () => {
  assert.deepEqual(booleanCardCommands(false), BOOLEAN_CARD_COMMANDS);
  const laser = booleanCardCommands(true);
  assert.equal(laser.length, BOOLEAN_CARD_COMMANDS.length + 1);
  assert.equal(laser[laser.length - 1], "bridge");
  assert.equal(laser[laser.length - 2], "weld");
});

test("criterion 1: the tooltip names the rule; the note says what is missing", () => {
  assert.ok(BRIDGE_RULE.length <= 45);
  assert.equal(bridgeTooltipNote("needs_one", true), "Select one or more objects.");
  assert.equal(
    bridgeTooltipNote("needs_one", false),
    "Select one or more objects. Use the Select tool.",
  );
  assert.equal(bridgeTooltipNote("ready", true), "Replaces the selection.");
});

test("criterion 5: typed values are clamped with the limit notice; a non-number is refused", () => {
  assert.deepEqual(clampField(WIDTH_RANGE, "Width", 50, " mm"), {
    value: 10,
    notice: "Width set to 10 mm, the maximum.",
  });
  assert.deepEqual(clampField(REACH_RANGE, "Reach", 0.1, " mm"), {
    value: 0.5,
    notice: "Reach set to 0.5 mm, the minimum.",
  });
  assert.deepEqual(clampField(WIDTH_RANGE, "Width", 2.5, " mm"), { value: 2.5, notice: null });
  assert.equal(clampField(WIDTH_RANGE, "Width", Number.NaN, " mm"), null);
});

test("criteria 4, 17 and 22: the refusals read as the spec writes them", () => {
  const refusal = (kind: string) => bridgeRefusalText({ kind, count: 0, of: 0 });
  assert.equal(refusal("one_page"), "Bridge works on one laser page. Nothing was changed.");
  assert.equal(refusal("assigned_parent"), null, "Frage 404 A: no refusal under an operation");
  assert.equal(refusal("nothing_to_join"), "Bridge: nothing to join. Nothing was changed.");
  assert.equal(
    refusal("too_many_bridges"),
    "Bridge would make more than 2000 bridges. Nothing was changed.",
  );
  assert.equal(refusal("no_islands"), "Bridge: no islands. Nothing was changed.");
  assert.equal(refusal("applied"), null);
  assert.ok(bridgeRefusalOutlines({ kind: "no_area", count: 1, of: 1 }));
  assert.ok(!bridgeRefusalOutlines({ kind: "nothing_to_join", count: 0, of: 0 }));
});

test("the values are remembered and read back clamped; unreadable values are the defaults", () => {
  const storage = memory();
  assert.deepEqual(readBridgeSettings(storage), DEFAULT_BRIDGE_SETTINGS);
  writeBridgeSettings(storage, {
    mode: "stencil",
    widthMm: 2,
    reachMm: 12,
    joinAll: true,
    bridgesPerIsland: 3,
  });
  assert.deepEqual(readBridgeSettings(storage), {
    mode: "stencil",
    widthMm: 2,
    reachMm: 12,
    joinAll: true,
    bridgesPerIsland: 3,
  });
  storage.setItem(BRIDGE_KEYS.width, "99");
  storage.setItem(BRIDGE_KEYS.reach, "abc");
  const read = readBridgeSettings(storage);
  assert.equal(read.widthMm, 10);
  assert.equal(read.reachMm, 5);
  assert.deepEqual(readBridgeSettings(undefined), DEFAULT_BRIDGE_SETTINGS);
});
