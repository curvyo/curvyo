// Tester cases for `specs/0093-laser-workbench-shell` criteria 1, 2, 11 and 13 through the wasm module the
// browser loads (needs `npm run build:wasm`), updated for `specs/0098-page-kinds` by its "Changes to accepted
// specs" table: Add manufacturing and the switcher are Add page with the Kind switch (`0098` 1, 4, 7), delete is
// Delete page (16), and the extension switch takes Laser out of the Kind switch (18 b). A dummy second kind
// registered through the same entry point appears beside the real list (18 a). Nothing here names the laser
// beyond the process id the session reports.

import assert from "node:assert/strict";
import { existsSync, readFileSync } from "node:fs";
import path from "node:path";
import { test } from "node:test";

import {
  type WorkbenchList,
  activeKind,
  contributionOf,
  kindOptions,
  registerWorkbench,
} from "../src/workbenches/registry.ts";

const BINDINGS = path.resolve(import.meta.dirname, "..", "src", "wasm-bindings");
const WASM = path.join(BINDINGS, "curvyo_editor_wasm_bg.wasm");

type Session = {
  workbenches_json: () => string;
  manufacturing_view_json: () => string;
  pages_json: () => string;
  add_page_of_kind: (kind: string, profile: string) => string;
  delete_page: (id: string) => boolean;
  activate_page: (id: string) => boolean;
  set_extension_enabled: (id: string, enabled: boolean) => string;
  add_fixture: (kind: string) => boolean;
  history_press: (direction: string) => string;
};

async function open(): Promise<Session> {
  const bindings = (await import(path.join(BINDINGS, "curvyo_editor_wasm.js"))) as {
    initSync: (options: { module: BufferSource }) => unknown;
    WasmSession: new (peer: bigint) => Session;
  };
  bindings.initSync({ module: readFileSync(WASM) });
  return new bindings.WasmSession(7n);
}

registerWorkbench({ kind: "vector", rail: () => null });
registerWorkbench({ kind: "sketch", rail: () => null });
registerWorkbench({ kind: "manufacturing", rail: () => null });

const list = (s: Session) => JSON.parse(s.workbenches_json()) as WorkbenchList;
const pages = (s: Session) => JSON.parse(s.pages_json()) as { id: string; name: string; kindWord: string }[];
const skip = { skip: !existsSync(WASM) };
/** The output kind the session offers (the only one compiled in). */
const output = (s: Session) => list(s).kinds.find((k) => k.workbench === "manufacturing")?.id ?? "";

test("AC1: a new project offers Drawing, Sketch and the output kind in the Kind switch", skip, async () => {
  const s = await open();
  const l = list(s);
  assert.deepEqual(
    kindOptions(l).map((k) => k.label),
    ["Drawing", "Sketch", "Laser"],
  );
  assert.equal(activeKind(l), "vector");
  assert.equal(l.page.word, "Drawing");
});

test("AC2: Add page with Kind Laser adds 'Laser 1', its tools follow, and undo takes it away", skip, async () => {
  const s = await open();
  const id = s.add_page_of_kind(output(s), "");
  assert.notEqual(id, "");
  const l = list(s);
  assert.equal(activeKind(l), "manufacturing");
  assert.equal(l.page.name, "Laser 1");
  assert.equal(l.page.word, "Laser");
  const view = JSON.parse(s.manufacturing_view_json());
  assert.equal(view.name, "Laser 1");
  assert.match(view.status, /Generic laser · 400\.0 × 300\.0 mm/);
  assert.ok(contributionOf(l, view.process)?.tabs[0]?.blocks.some((b) => b.kind === "machine_profile"));
  assert.deepEqual(
    pages(s).map((p) => [p.name, p.kindWord]),
    [
      ["Page 1", "Drawing"],
      ["Laser 1", "Laser"],
    ],
  );
  s.history_press("undo");
  assert.equal(pages(s).length, 1);
  assert.equal(activeKind(list(s)), "vector");
});

test("AC11a: a dummy second kind appears through the same registry beside the real list", skip, async () => {
  const s = await open();
  registerWorkbench({ kind: "tester-dummy", rail: () => null });
  const real = list(s);
  const merged: WorkbenchList = {
    ...real,
    kinds: [...real.kinds, { id: "tester.dummy", label: "Dummy", icon: "Box", workbench: "tester-dummy" }],
  };
  assert.deepEqual(
    kindOptions(merged).map((k) => k.label),
    ["Drawing", "Sketch", "Laser", "Dummy"],
  );
});

test("AC11b: switched off, Laser leaves the Kind switch and the page stays as 'Laser (off)'", skip, async () => {
  const s = await open();
  const id = s.add_page_of_kind(output(s), "");
  assert.ok(s.add_fixture("engraved_grid"));
  const process = output(s);
  s.set_extension_enabled(process, false);
  const l = list(s);
  assert.deepEqual(
    kindOptions(l).map((k) => k.label),
    ["Drawing", "Sketch"],
  );
  assert.equal(activeKind(l), "vector");
  assert.equal(l.page.word, "Laser (off)");
  assert.equal(s.add_page_of_kind(process, ""), "");
  s.set_extension_enabled(process, true);
  assert.ok(s.activate_page(id));
  assert.equal(activeKind(list(s)), "manufacturing");
});

test("AC13: Delete page removes the laser page at once and undo brings it back", skip, async () => {
  const s = await open();
  const id = s.add_page_of_kind(output(s), "");
  assert.ok(s.delete_page(id));
  assert.equal(pages(s).length, 1);
  assert.equal(activeKind(list(s)), "vector");
  s.history_press("undo");
  assert.deepEqual(
    pages(s).map((p) => p.name),
    ["Page 1", "Laser 1"],
  );
});
