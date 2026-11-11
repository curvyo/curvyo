import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test } from "node:test";
import init, { WasmSession } from "../src/wasm-bindings/curvyo_editor_wasm.js";

await init({ module_or_path: readFileSync(new URL("../src/wasm-bindings/curvyo_editor_wasm_bg.wasm", import.meta.url)) });


test("placed Spiro opens and exports its evaluated geometry", () => {
  const bytes = readFileSync(new URL("./fixtures/spiro-placement-0110.curvyo", import.meta.url));
  const s = WasmSession.open(732n, bytes);
  assert.equal(s.restore_view_text("1 0 0"), true);
 
  const svg = s.export_svg(false, false, false, false);
  assert.match(svg, /^svg:1:/);
  assert.match(svg, / C /);
  s.set_tool("select");
  s.key_down("a", false, true, false, false, false, false, 0);
  assert.equal(s.selection_count(), 1);
  assert.equal(s.begin_width_edit(), true);
  const before = s.pack();
  const x = (56 - s.view_origin_x()) * s.view_scale();
  const y = (19 - s.view_origin_y()) * s.view_scale();
  s.pointer_down(x, y, false, false, false);
  s.pointer_up(x, y, false, false, false);
 
  assert.notDeepEqual(s.pack(), before);
  const profiledSvg = s.export_svg(false, false, false, false);
  assert.match(profiledSvg, /^svg:1:/);
  const reopened = WasmSession.open(733n, s.pack());
  assert.equal(reopened.export_svg(false, false, false, false), profiledSvg);
  reopened.set_tool("select");
  reopened.key_down("a", false, true, false, false, false, false, 0);
  assert.equal(reopened.can_convert_selected_spiro_to_bezier(), true);
  assert.equal(reopened.convert_selected_spiro_to_bezier(), true);
  assert.equal(reopened.can_convert_selected_spiro_to_bezier(), false);
  assert.equal(reopened.export_svg(false, false, false, false), profiledSvg);
  reopened.history_press("undo");
  assert.equal(reopened.can_convert_selected_spiro_to_bezier(), true);
  assert.equal(reopened.export_svg(false, false, false, false), profiledSvg);
  reopened.free();
  s.free();
});

test("Spiro pen draws with undo and an editable width profile", () => {
  const s = new WasmSession(734n);
  assert.equal(s.restore_view_text("1 0 0"), true);
  s.set_tool("pen");
  assert.equal(s.set_pen_mode("spiro"), true);
  for (const [x, y] of [[10, 10], [30, 10], [50, 10]]) {
    s.pointer_down(x, y, false, false, false);
    s.pointer_up(x, y, false, false, false);
   
  }
  s.key_down("Enter", false, false, false, false, false, false, 0);
 
  assert.match(s.export_svg(false, false, false, false), /^svg:1:/);
  s.history_press("undo");
  assert.match(s.export_svg(false, false, false, false), /^svg:0:/);
  s.history_press("redo");
 
  s.set_tool("select");
  s.key_down("a", false, true, false, false, false, false, 0);
  assert.equal(s.begin_width_edit(), true);
  const before = s.pack();
  s.pointer_down(20, 10, false, false, false);
  s.pointer_up(20, 10, false, false, false);
 
  assert.notDeepEqual(s.pack(), before);
  assert.match(s.export_svg(false, false, false, false), /^svg:1:/);
  s.free();
});

test("placed Spiro moves, reopens and undoes before cloning and export", () => {
  const bytes = readFileSync(new URL("./fixtures/spiro-placement-0110.curvyo", import.meta.url));
  const s = WasmSession.open(735n, bytes);
  assert.equal(s.restore_view_text("1 0 0"), true);
 
  const original = s.export_svg(false, false, false, false);
  assert.match(original, /^svg:1:/);
  s.set_tool("select");
  s.key_down("a", false, true, false, false, false, false, 0);
  s.key_down("ArrowRight", false, false, false, false, false, false, 0);
 
  const moved = s.export_svg(false, false, false, false);
  assert.match(moved, /^svg:1:/);
  assert.notEqual(moved, original);
  const reopened = WasmSession.open(736n, s.pack());
  assert.equal(reopened.export_svg(false, false, false, false), moved);
  s.history_press("undo");
 
  assert.equal(s.export_svg(false, false, false, false), original);
  s.history_press("redo");
 
  assert.equal(s.export_svg(false, false, false, false), moved);
  assert.equal(s.linked_clone().kind, "done");
 
  assert.match(s.export_svg(false, false, false, false), /^svg:2:/);
  reopened.free(); s.free();
});

test("moving a profiled Spiro commits at once", () => {
  const bytes = readFileSync(new URL("./fixtures/spiro-placement-0110.curvyo", import.meta.url));
  const s = WasmSession.open(737n, bytes);
  s.restore_view_text("1 0 0");
  s.set_tool("select"); s.key_down("a", false, true, false, false, false, false, 0);
  assert.equal(s.begin_width_edit(), true);
  const x = (56 - s.view_origin_x()) * s.view_scale();
  const y = (19 - s.view_origin_y()) * s.view_scale();
  s.pointer_down(x, y, false, false, false); s.pointer_up(x, y, false, false, false);
  s.end_width_edit();
  const before = s.pack();
  s.key_down("ArrowRight", false, false, false, false, false, false, 0);
  assert.equal(Buffer.from(s.pack()).equals(Buffer.from(before)), false, "a move never waits for the outline");
  assert.match(s.export_svg(false, false, false, false), /^svg:1:/);
  s.free();
});

test("moving a profiled Spiro and its clone keeps both exportable", () => {
  const bytes = readFileSync(new URL("./fixtures/spiro-placement-0110.curvyo", import.meta.url));
  const s = WasmSession.open(738n, bytes);
  s.restore_view_text("1 0 0");
  s.set_tool("select"); s.key_down("a", false, true, false, false, false, false, 0);
  assert.equal(s.begin_width_edit(), true);
  const x = (56 - s.view_origin_x()) * s.view_scale();
  const y = (19 - s.view_origin_y()) * s.view_scale();
  s.pointer_down(x, y, false, false, false); s.pointer_up(x, y, false, false, false);
  s.end_width_edit();
  assert.equal(s.linked_clone().kind, "done");
  s.key_down("a", false, true, false, false, false, false, 0);
  assert.equal(s.selection_count(), 2);
  s.key_down("ArrowRight", false, false, false, false, false, false, 0);
 
  assert.match(s.export_svg(false, false, false, false), /^svg:2:/);
  s.free();
});
