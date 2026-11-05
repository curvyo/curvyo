// The page kind registry (`specs/0098-page-kinds` criteria 7, 8 and 18 a, ADR 0042 §7, amending
// `specs/0093` criterion 11a): the Kind switch offers the kinds a registered entry draws, a dummy
// second output kind registered through the same entry point appears in it, the tools follow the
// active page's workbench, and no shell source names the laser.

import assert from "node:assert/strict";
import { readFileSync, readdirSync } from "node:fs";
import { test } from "node:test";

import {
  type WorkbenchList,
  activeKind,
  contributionOf,
  kindOptions,
  registerWorkbench,
  workbenchEntry,
} from "../src/workbenches/registry.ts";

registerWorkbench({ kind: "vector", rail: () => "vector rail" });
registerWorkbench({ kind: "sketch", rail: () => "sketch rail" });
registerWorkbench({ kind: "manufacturing", rail: () => "vector rail" });
registerWorkbench({ kind: "test-dummy", rail: () => "dummy rail" });

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
      tabs: [{ id: "dummy", label: "Dummy", icon: "Box", blocks: [{ kind: "fixture_list" as const, label: "Fixtures" }] }],
      toolbox_cards: [],
      fixture_kinds: [],
    },
  ],
};

function list(): WorkbenchList {
  return {
    active: "test-dummy",
    page: { id: "p", name: "Dummy 1", kind: "test.dummy", word: "Dummy", icon: "Box", off: false },
    kinds: [
      { id: "drawing", label: "Drawing", icon: "PenTool", workbench: "vector" },
      { id: "sketch", label: "Sketch", icon: "DraftingCompass", workbench: "sketch" },
      { id: "test.dummy", label: "Dummy", icon: "Box", workbench: "test-dummy" },
      { id: "nobody.draws", label: "Nobody", icon: "Box", workbench: "unregistered" },
    ],
    extensions: [],
    manifests: [dummyManifest],
  };
}

test("a dummy second output kind appears in the Kind switch through the registry", () => {
  assert.deepEqual(
    kindOptions(list()).map((k) => [k.id, k.label, k.icon]),
    [
      ["drawing", "Drawing", "PenTool"],
      ["sketch", "Sketch", "DraftingCompass"],
      ["test.dummy", "Dummy", "Box"],
    ],
  );
  assert.equal(contributionOf(list(), "test.dummy")?.label, "Dummy");
  assert.equal(contributionOf(list(), "curvyo.laser"), undefined, "a switched-off extension has no manifest");
});

test("the tools follow the active page's workbench", () => {
  assert.equal(activeKind(list()), "test-dummy");
  assert.equal(workbenchEntry<unknown, string>(activeKind(list()))?.rail(null), "dummy rail");
  const drawing: WorkbenchList = { ...list(), active: "vector" };
  assert.equal(workbenchEntry<unknown, string>(activeKind(drawing))?.rail(null), "vector rail");
  const sketch: WorkbenchList = { ...list(), active: "sketch" };
  assert.equal(workbenchEntry<unknown, string>(activeKind(sketch))?.rail(null), "sketch rail");
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
