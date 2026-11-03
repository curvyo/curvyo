// Tester cases for `specs/0093-laser-workbench-shell` criteria 1, 2, 11 and 13 through the wasm module the
// browser loads (needs `npm run build:wasm`): the session's registry fed into the front end's registry, a
// dummy second workbench registered through the same entry point, the extension switch and delete with undo.
// The switcher shows only what the registry draws; nothing here names the laser beyond the process id the
// session reports.

import assert from "node:assert/strict";
import { existsSync, readFileSync } from "node:fs";
import path from "node:path";
import { test } from "node:test";

import {
  type WorkbenchList,
  activeKind,
  contributionOf,
  registerWorkbench,
  switcherButtons,
} from "../src/workbenches/registry.ts";

const BINDINGS = path.resolve(import.meta.dirname, "..", "src", "wasm-bindings");
const WASM = path.join(BINDINGS, "curvyo_editor_wasm_bg.wasm");

type Session = {
  workbenches_json: () => string;
  manufacturing_view_json: () => string;
  add_manufacturing: (process: string) => string;
  delete_manufacturing: (id: string) => boolean;
  set_workbench: (name: string) => void;
  set_extension_enabled: (id: string, enabled: boolean) => string;
  add_fixture: (kind: string) => boolean;
  history_press: (direction: string) => string;
  pack: (version: string) => Uint8Array;
};

async function open(): Promise<Session> {
  const bindings = (await import(path.join(BINDINGS, "curvyo_editor_wasm.js"))) as {
    initSync: (options: { module: BufferSource }) => unknown;
    WasmSession: new (peer: bigint) => Session;
  };
  bindings.initSync({ module: readFileSync(WASM) });
  return new bindings.WasmSession(7n);
}

registerWorkbench({ kind: "vector", icon: () => "PenTool", rail: () => null });
registerWorkbench({ kind: "sketch", icon: () => "DraftingCompass", rail: () => null });
registerWorkbench({ kind: "manufacturing", icon: (entry) => entry.icon ?? "Factory", rail: () => null });

const list = (s: Session) => JSON.parse(s.workbenches_json()) as WorkbenchList;
const skip = { skip: !existsSync(WASM) };

test("AC1: a new project shows Vector and Sketch, Laser only as an Add manufacturing choice", skip, async () => {
  const s = await open();
  const l = list(s);
  assert.deepEqual(
    switcherButtons(l).map((b) => b.label),
    ["Vector", "Sketch"],
  );
  assert.deepEqual(
    l.add.map((a) => a.label),
    ["Laser"],
  );
  assert.equal(activeKind(l), "vector");
});

test("AC2: Add manufacturing Laser appends a pressed 'Laser 1' button and undo takes it away", skip, async () => {
  const s = await open();
  const id = s.add_manufacturing(l0(s));
  assert.notEqual(id, "");
  const l = list(s);
  const buttons = switcherButtons(l);
  assert.deepEqual(
    buttons.map((b) => [b.label, b.pressed, b.enabled]),
    [
      ["Vector", false, true],
      ["Sketch", false, true],
      ["Laser 1", true, true],
    ],
  );
  assert.equal(activeKind(l), "manufacturing");
  const view = JSON.parse(s.manufacturing_view_json());
  assert.equal(view.name, "Laser 1");
  assert.match(view.status, /Generic laser · 400\.0 × 300\.0 mm/);
  const tab = contributionOf(l, view.process)?.tabs[0];
  assert.equal(tab?.label, "Laser");
  s.history_press("undo");
  assert.deepEqual(
    switcherButtons(list(s)).map((b) => b.label),
    ["Vector", "Sketch"],
  );
  assert.equal(list(s).active, "vector");
});

function l0(s: Session): string {
  return list(s).add[0].process;
}

test("AC11a: a dummy second manifest workbench appears through the same registry beside the real list", skip, async () => {
  const s = await open();
  s.add_manufacturing(l0(s));
  const real = list(s);
  registerWorkbench({ kind: "tester-dummy", icon: () => "Box", rail: () => null });
  const merged: WorkbenchList = {
    ...real,
    entries: [...real.entries, { id: "dummy-1", kind: "tester-dummy", label: "Dummy 1", enabled: true }],
  };
  const labels = switcherButtons(merged).map((b) => b.label);
  assert.deepEqual(labels, ["Vector", "Sketch", "Laser 1", "Dummy 1"]);
});

test("AC11b: switched off, Laser leaves Add manufacturing and the manufacturing stays disabled with its name", skip, async () => {
  const s = await open();
  const id = s.add_manufacturing(l0(s));
  s.add_fixture("engraved_grid");
  const ext = list(s).extensions;
  assert.equal(ext.length, 1);
  assert.equal(ext[0].name, "Laser workbench");
  assert.match(ext[0].version, /^\d+\.\d+\.\d+$/);
  const process = ext[0].id;
  s.set_extension_enabled(process, false);
  const l = list(s);
  assert.equal(l.active, "vector");
  assert.deepEqual(l.add, []);
  assert.equal(contributionOf(l, process), undefined);
  const disabled = switcherButtons(l).find((b) => b.id === id);
  assert.ok(disabled);
  assert.equal(disabled.enabled, false);
  assert.equal(disabled.label, "Laser 1 (disabled)");
  s.set_workbench(id);
  assert.equal(list(s).active, "vector", "a disabled one does not open");
  s.set_extension_enabled(process, true);
  s.set_workbench(id);
  assert.equal(list(s).active, id);
});

test("AC13: delete goes at once and undo brings the manufacturing and its workbench back", skip, async () => {
  const s = await open();
  const id = s.add_manufacturing(l0(s));
  s.add_fixture("engraved_grid");
  assert.equal(s.delete_manufacturing(id), true);
  assert.equal(list(s).active, "vector");
  assert.deepEqual(
    switcherButtons(list(s)).map((b) => b.label),
    ["Vector", "Sketch"],
  );
  s.history_press("undo");
  assert.equal(list(s).active, id);
  assert.equal(JSON.parse(s.manufacturing_view_json()).fixtures.length, 1);
});
