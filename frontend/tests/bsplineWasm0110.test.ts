// Real browser-target WASM interaction for the editable B-spline conversion.
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import path from "node:path";
import { test } from "node:test";

test("B-spline drawing survives save and converts to the same Bézier SVG with undo", async () => {
  const bindings = await import("../src/wasm-bindings/curvyo_editor_wasm.js");
  bindings.initSync({ module: readFileSync(path.resolve(import.meta.dirname, "../src/wasm-bindings/curvyo_editor_wasm_bg.wasm")) });
  const session = new bindings.WasmSession(300n);
  assert.equal(session.restore_view_text("1 0 0"), true);
  session.set_tool("pen");
  assert.equal(session.set_pen_mode("bspline"), true);
  for (const [x,y] of [[10,10],[30,10],[30,30],[10,30]]) {
    session.pointer_down(x,y,false,false,false);
    session.pointer_up(x,y,false,false,false);
  }
  session.key_down("Enter",false,false,false,false,false,false,0);
  const originalSvg = session.export_svg(false,false,false,false);
  assert.match(originalSvg,/^svg:1:/);
  assert.match(originalSvg, / C /);
  const bytes=session.pack();
  const reopened=bindings.WasmSession.open(301n,bytes);
  assert.equal(reopened.export_svg(false,false,false,false),originalSvg);
  reopened.set_tool("select");
  reopened.key_down("a",false,true,false,false,false,false,0);
  assert.equal(reopened.selection_count(),1);
  assert.equal(reopened.can_convert_selected_bspline_to_bezier(),true);
  assert.equal(reopened.convert_selected_bspline_to_bezier(),true);
  assert.equal(reopened.can_convert_selected_bspline_to_bezier(),false);
  assert.equal(reopened.export_svg(false,false,false,false),originalSvg);
  const converted=reopened.pack();
  const reopenedConverted=bindings.WasmSession.open(302n,converted);
  assert.equal(reopenedConverted.export_svg(false,false,false,false),originalSvg);
  reopened.history_press("undo");
  assert.equal(reopened.can_convert_selected_bspline_to_bezier(),true);
  assert.equal(reopened.export_svg(false,false,false,false),originalSvg);
  reopenedConverted.free(); reopened.free(); session.free();
});
