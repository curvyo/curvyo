// `specs/0051-sidebar-and-no-menu/` criterion 10: the collapsed state is a boolean in the webview's
// storage under `curvyo.panel-collapsed`; a first start, a missing or unreadable store means shown.

import assert from "node:assert/strict";
import { test } from "node:test";

import {
  PANEL_COLLAPSED_KEY,
  type PanelStorage,
  readPanelCollapsed,
  writePanelCollapsed,
} from "../src/lib/panelCollapsed.ts";

function memory(): PanelStorage & { values: Map<string, string> } {
  const values = new Map<string, string>();
  return {
    values,
    getItem: (key) => values.get(key) ?? null,
    setItem: (key, value) => void values.set(key, value),
  };
}

const throwing: PanelStorage = {
  getItem: () => {
    throw new Error("blocked");
  },
  setItem: () => {
    throw new Error("blocked");
  },
};

test("the key is curvyo.panel-collapsed", () => {
  assert.equal(PANEL_COLLAPSED_KEY, "curvyo.panel-collapsed");
});

test("a first start with nothing saved shows the body", () => {
  assert.equal(readPanelCollapsed(memory()), false);
});

test("hiding and showing are read back", () => {
  const storage = memory();
  writePanelCollapsed(true, storage);
  assert.equal(readPanelCollapsed(storage), true);
  writePanelCollapsed(false, storage);
  assert.equal(readPanelCollapsed(storage), false);
});

test("an unknown stored value shows the body", () => {
  const storage = memory();
  storage.values.set(PANEL_COLLAPSED_KEY, "maybe");
  assert.equal(readPanelCollapsed(storage), false);
});

test("storage that throws shows the body and never throws", () => {
  assert.equal(readPanelCollapsed(throwing), false);
  assert.doesNotThrow(() => writePanelCollapsed(true, throwing));
});

test("no storage at all shows the body and never throws", () => {
  assert.equal(readPanelCollapsed(undefined), false);
  assert.doesNotThrow(() => writePanelCollapsed(true, undefined));
});
