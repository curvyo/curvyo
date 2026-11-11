import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";
import init, { WasmSession } from "../src/wasm-bindings/curvyo_editor_wasm.js";

test("a stroked path can start width editing and exit cleanly", async () => {
  const bytes = readFileSync(new URL("../src/wasm-bindings/curvyo_editor_wasm_bg.wasm", import.meta.url));
  await init({ module_or_path: bytes });
  const session = new WasmSession(1n);
  session.set_tool("pen");
  session.pointer_down(10, 10, false, false, false);
  session.pointer_up(10, 10, false, false, false);
  session.pointer_down(50, 10, false, false, false);
  session.pointer_up(50, 10, false, false, false);
  session.set_tool("select");
  session.pointer_down(30, 10, false, false, false);
  session.pointer_up(30, 10, false, false, false);
  assert.equal(session.selection_count(), 1);
  assert.equal(session.begin_width_edit(), true);
  assert.equal(session.width_edit_active(), true);
  session.end_width_edit();
  assert.equal(session.width_edit_active(), false);
  session.free();
});
