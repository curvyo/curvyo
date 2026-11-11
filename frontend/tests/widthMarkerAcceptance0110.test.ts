// A marker on a variable stroke (`specs/0110` criterion 6, ADR 0053): the edit applies at once
// and the artwork stays exportable.
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test } from "node:test";
import init, { WasmSession } from "../src/wasm-bindings/curvyo_editor_wasm.js";

await init({ module_or_path: readFileSync(new URL("../src/wasm-bindings/curvyo_editor_wasm_bg.wasm", import.meta.url)) });

test("adding an end marker to a variable stroke keeps exportable artwork", () => {
  const s = new WasmSession(809n);
  s.restore_view_text("1 0 0");
  s.set_tool("pen");
  for (const [x, y] of [[10, 10], [50, 10]]) { s.pointer_down(x, y, false, false, false); s.pointer_up(x, y, false, false, false); }
  s.set_tool("select");
  s.pointer_down(30, 10, false, false, false); s.pointer_up(30, 10, false, false, false);
  assert.equal(s.begin_width_edit(), true);
  s.pointer_down(20, 10, false, false, false); s.pointer_up(20, 10, false, false, false);
  s.pointer_down(40, 10, false, false, false); s.pointer_up(40, 10, false, false, false);
  s.pointer_down(40, 10, false, false, false);
  s.pointer_hover(40, 14, false, false, false);
  s.pointer_up(40, 14, false, false, false);
  s.end_width_edit();
  assert.match(s.export_svg(false, false, false, false), /^svg:1:/);
  const before = s.pack();
  s.set_marker_shape("end", "arrow");
  assert.notDeepEqual(s.pack(), before);
  assert.match(s.export_svg(false, false, false, false), /^svg:1:/);
  s.free();
});
