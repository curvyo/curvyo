// The workbench registry (`specs/0093-laser-workbench-shell` criteria 1 and 11a, ADR 0032 §3):
// the switcher lists whatever the registry draws, a dummy second manifest workbench registered
// through the same entry point appears in it, and no shell source names the laser.

import assert from "node:assert/strict";
import { readFileSync, readdirSync } from "node:fs";
import { test } from "node:test";

import {
  type WorkbenchList,
  activeKind,
  contributionOf,
  registerWorkbench,
  switcherButtons,
} from "../src/workbenches/registry.ts";

registerWorkbench({ kind: "vector", icon: () => "PenTool", rail: () => null });
registerWorkbench({ kind: "sketch", icon: () => "DraftingCompass", rail: () => null });
registerWorkbench({ kind: "manufacturing", icon: (entry) => entry.icon ?? "Factory", rail: () => null });

const dummyManifest = {
  id: "test.dummy",
  name: "Dummy workbench",
  version: "0.0.1",
  licence: "AGPL-3.0-or-later",
  contributions: [
    {
      kind: "workbench" as const,
      id: "test.dummy",
      label: "Dummy",
      icon: "Box",
      machine_name: "Dummy machine",
      tabs: [{ id: "dummy", label: "Dummy", icon: "Box", blocks: [{ kind: "fixture_list" as const, label: "Fixtures" }] }],
      toolbox_cards: [],
      fixture_kinds: [],
    },
  ],
};

function list(): WorkbenchList {
  return {
    active: "1:7",
    entries: [
      { id: "vector", kind: "vector", label: "Vector", enabled: true },
      { id: "sketch", kind: "sketch", label: "Sketch", enabled: true },
      { id: "1:3", kind: "manufacturing", process: "curvyo.laser", label: "Laser 1", icon: "Zap", enabled: false },
      { id: "1:7", kind: "manufacturing", process: "test.dummy", label: "Dummy 1", icon: "Box", enabled: true },
      { id: "1:9", kind: "unknown-kind", label: "Nobody draws me", enabled: true },
    ],
    add: [{ process: "test.dummy", label: "Dummy", icon: "Box" }],
    extensions: [],
    manifests: [dummyManifest],
    disabled: [{ id: "1:3", label: "Laser 1" }],
  };
}

test("a dummy second manifest workbench appears in the switcher through the registry", () => {
  const buttons = switcherButtons(list());
  assert.deepEqual(
    buttons.map((b) => [b.id, b.label, b.icon, b.enabled, b.pressed]),
    [
      ["vector", "Vector", "PenTool", true, false],
      ["sketch", "Sketch", "DraftingCompass", true, false],
      ["1:3", "Laser 1 (disabled)", "Zap", false, false],
      ["1:7", "Dummy 1", "Box", true, true],
    ],
  );
  assert.equal(activeKind(list()), "manufacturing");
  assert.equal(contributionOf(list(), "test.dummy")?.label, "Dummy");
  assert.equal(contributionOf(list(), "curvyo.laser"), undefined, "a switched-off extension has no manifest");
});

test("without a manufacturing the switcher shows Vector and Sketch only", () => {
  const plain: WorkbenchList = { ...list(), active: "vector", entries: list().entries.slice(0, 2) };
  assert.deepEqual(switcherButtons(plain).map((b) => b.id), ["vector", "sketch"]);
  assert.equal(activeKind(plain), "vector");
});

test("no shell source names the laser process or branches on it: it is data from the manifest", () => {
  const src = new URL("../src/", import.meta.url);
  const offenders: string[] = [];
  const walk = (dir: URL) => {
    for (const entry of readdirSync(dir, { withFileTypes: true })) {
      if (entry.name === "wasm-bindings") continue;
      const child = new URL(entry.name + (entry.isDirectory() ? "/" : ""), dir);
      if (entry.isDirectory()) walk(child);
      else if (/\.(ts|tsx)$/.test(entry.name) && /curvyo\.laser|["'`]laser["'`]/i.test(readFileSync(child, "utf8"))) {
        offenders.push(child.pathname.slice(src.pathname.length));
      }
    }
  };
  walk(src);
  assert.deepEqual(offenders, []);
});
