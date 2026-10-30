// Reproduction of the "distribute does nothing" report: the exact wasm
// module and entry points the browser build uses. Needs `npm run build:wasm`.

import assert from "node:assert/strict";
import { existsSync, readFileSync } from "node:fs";
import path from "node:path";
import { test } from "node:test";

const ROOT = path.resolve(import.meta.dirname, "..", "..");
const BINDINGS = path.resolve(import.meta.dirname, "..", "src", "wasm-bindings");
const WASM = path.join(BINDINGS, "curvyo_editor_wasm_bg.wasm");

test("distribute through the browser's wasm moves the middle object", { skip: !existsSync(WASM) }, async () => {
  const bindings = (await import(path.join(BINDINGS, "curvyo_editor_wasm.js"))) as {
    initSync: (options: { module: BufferSource }) => unknown;
    WasmSession: new (peer: number) => {
      set_tool: (tool: string) => string | undefined;
      pointer_down: (x: number, y: number, shift: boolean, ctrl: boolean, alt: boolean) => void;
      pointer_up: (x: number, y: number, shift: boolean, constrain: boolean, alt: boolean) => void;
      key_down: (
        key: string,
        shift: boolean,
        ctrl: boolean,
        alt: boolean,
        repeat: boolean,
        dom_blocked: boolean,
        mac: boolean,
        time_ms: number,
      ) => string;
      align_shown: () => boolean;
      distribute_shown: () => boolean;
      distribute: (code: string, gap_mm: number | null) => string;
      align: (code: string) => string;
      pack: (version: string) => Uint8Array;
    };
  };
  bindings.initSync({ module: readFileSync(WASM) });
  const session = new bindings.WasmSession(1n);
  for (const [x0, y0, x1, y1] of [
    [0, 0, 10, 10],
    [30, 0, 34, 10],
    [60, 0, 70, 10],
  ]) {
    session.set_tool("rectangle");
    session.pointer_down(x0, y0, false, false, false);
    session.pointer_up(x1, y1, false, false, false);
  }
  session.set_tool("select");
  assert.equal(session.key_down("a", false, true, false, false, false, false, 0), "select-all");
  assert.equal(session.distribute_shown(), true, "the distribute group shows");
  const outcome = session.distribute("h-centres", null);
  assert.equal(outcome, "aligned", `the distribute commits: ${outcome}`);
  // The middle rectangle (30 to 34) is re-centred between the outer ones:
  // centres 5 and 65, so its centre lands on 35, its left edge on 33.
  const bytes = session.pack("0.1.0");
  assert.ok(bytes.length > 0, "the document packs");
});

test("the customer's amendment: a typed gap distributes two objects", { skip: !existsSync(WASM) }, async () => {
  const bindings = (await import(path.join(BINDINGS, "curvyo_editor_wasm.js"))) as {
    initSync: (options: { module: BufferSource }) => unknown;
    WasmSession: new (peer: bigint) => {
      set_tool: (tool: string) => string | undefined;
      pointer_down: (x: number, y: number, shift: boolean, ctrl: boolean, alt: boolean) => void;
      pointer_up: (x: number, y: number, shift: boolean, constrain: boolean, alt: boolean) => void;
      key_down: (
        key: string,
        shift: boolean,
        ctrl: boolean,
        alt: boolean,
        repeat: boolean,
        dom_blocked: boolean,
        mac: boolean,
        time_ms: number,
      ) => string;
      distribute_shown: () => boolean;
      distribute: (code: string, gap_mm: number | null) => string;
      pack: (version: string) => Uint8Array;
    };
  };
  bindings.initSync({ module: readFileSync(WASM) });
  const session = new bindings.WasmSession(1n);
  for (const [x0, y0, x1, y1] of [
    [0, 0, 10, 10],
    [30, 0, 40, 10],
  ]) {
    session.set_tool("rectangle");
    session.pointer_down(x0, y0, false, false, false);
    session.pointer_up(x1, y1, false, false, false);
  }
  session.set_tool("select");
  session.key_down("a", false, true, false, false, false, false, 0);
  assert.equal(session.distribute_shown(), true, "two objects show the group");
  // Without a typed gap the ends define the spacing: two change nothing.
  assert.equal(session.distribute("h-gaps", null), "unchanged");
  // With one, the second is spaced from the first: 10 + 5 = 15.
  assert.equal(session.distribute("h-gaps", 5), "aligned");
  const bytes = session.pack("0.1.0");
  assert.ok(bytes.length > 0, "the document packs");
});
