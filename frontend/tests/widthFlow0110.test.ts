// Width profiles end to end in the wasm session (`specs/0110-pen-modes-variable-stroke` criteria
// 4 to 6): the outline is computed at once, so every gesture writes on release and every export
// sees it. Pointer positions are screen pixels; a drag chooses its axis after 3 px.
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test } from "node:test";
import init, { WasmSession } from "../src/wasm-bindings/curvyo_editor_wasm.js";

await init({ module_or_path: readFileSync(new URL("../src/wasm-bindings/curvyo_editor_wasm_bg.wasm", import.meta.url)) });

function click(s: WasmSession, x: number, y: number) {
  s.pointer_down(x, y, false, false, false);
  s.pointer_up(x, y, false, false, false);
}

function drag(s: WasmSession, from: [number, number], to: [number, number]) {
  s.pointer_down(from[0], from[1], false, false, false);
  s.pointer_hover(to[0], to[1], false, false, false);
  s.pointer_up(to[0], to[1], false, false, false);
}

/** A session with the path through `points` drawn by the Pen in `mode`, selected. */
function drawn(seed: bigint, points: [number, number][], mode = "bezier"): WasmSession {
  const s = new WasmSession(seed);
  assert.equal(s.restore_view_text("1 0 0"), true);
  s.set_tool("pen");
  s.set_pen_mode(mode);
  for (const [x, y] of points) click(s, x, y);
  s.key_down("Enter", false, false, false, false, false, false, 0);
  s.set_tool("select");
  click(s, 30, 10);
  return s;
}

/** Two width points at 20 and 40 mm, the second widened by a drag across. */
function profiled(s: WasmSession) {
  assert.equal(s.begin_width_edit(), true);
  click(s, 20, 10);
  click(s, 40, 10);
  drag(s, [40, 10], [40, 14]);
}

test("a width gesture writes at once and exports, undoes and converts", () => {
  const s = drawn(401n, [[10, 10], [50, 10]]);
  assert.equal(s.begin_width_edit(), true);
  const before = s.pack();
  click(s, 30, 10);
  assert.notDeepEqual(s.pack(), before, "a click on the line adds a point at once");
  assert.equal(s.take_width_edit_notice(), "");
  const start = s.width_point_mm();
  drag(s, [30, 10], [30, 14]);
  // 4 px from the press: the width there grows by twice that (UX review U3, relative drag).
  assert.ok(Math.abs(s.width_point_mm() - (start + 8 / s.view_scale())) < 1e-9, "the width grows relative to the press");
  assert.match(s.export_svg(false, false, false, false), /^svg:1:/);
  s.history_press("undo");
  s.history_press("redo");
  assert.match(s.export_svg(false, false, false, false), /^svg:1:/);
  s.end_width_edit();
  assert.equal(s.linked_clone().kind, "done");
  assert.match(s.export_svg(false, false, false, false), /^svg:2:/);
  assert.equal(s.apply_stroke_to_path().kind, "applied");
  s.history_press("undo");
  s.free();
});

test("the numeric field sets the selected point's width as one step", () => {
  const s = drawn(402n, [[10, 10], [50, 10]]);
  assert.equal(s.begin_width_edit(), true);
  assert.ok(Number.isNaN(s.width_point_mm()), "no point selected");
  click(s, 30, 10);
  assert.equal(s.set_width_point_mm(5), true);
  assert.equal(s.width_point_mm(), 5);
  assert.equal(s.set_width_point_mm(-1), false);
  s.history_press("undo");
  assert.notEqual(s.width_point_mm(), 5);
  s.free();
});

test("a closed path profile is saved, reopened and exported", () => {
  const s = drawn(403n, [[10, 10], [50, 10], [50, 50], [10, 10]]);
  assert.equal(s.selection_count(), 1);
  assert.equal(s.begin_width_edit(), true);
  click(s, 30, 10);
  assert.match(s.export_svg(false, false, false, false), /^svg:1:/);
  const restored = WasmSession.open(404n, s.pack());
  assert.match(restored.export_svg(false, false, false, false), /^svg:1:/);
  restored.free();
  s.free();
});

test("a width out of range is refused with a notice and changes nothing", () => {
  const s = drawn(405n, [[10, 10], [50, 10]]);
  assert.equal(s.begin_width_edit(), true);
  click(s, 20, 10);
  click(s, 40, 10);
  const before = s.pack();
  drag(s, [40, 10], [40, 1_000_000_000]);
  assert.match(s.take_width_edit_notice(), /out of range/);
  assert.deepEqual(s.pack(), before);
  s.free();
});

test("a multi-segment line takes a zero-width end and exports the edited contour", () => {
  const s = drawn(406n, [[10, 10], [50, 10], [50, 50]], "lines");
  assert.equal(s.begin_width_edit(), true);
  click(s, 10, 10);
  s.pointer_down(10, 10, false, false, false);
  s.pointer_hover(10, 14, false, false, false);
  // Back past the press shrinks it; it stops at nothing (UX review U3).
  s.pointer_hover(10, 6, false, false, false);
  s.pointer_up(10, 6, false, false, false);
  assert.equal(s.width_point_mm(), 0, "the tip tapers to nothing");
  click(s, 40, 10);
  drag(s, [40, 10], [40, 14]);
  assert.ok(s.width_point_mm() > 0);
  assert.match(s.export_svg(false, false, false, false), /^svg:1:/);
  s.end_width_edit();
  assert.equal(s.apply_stroke_to_path().kind, "applied");
  s.history_press("undo");
  assert.match(s.export_svg(false, false, false, false), /^svg:1:/);
  s.free();
});

test("a dashed variable-width line survives clone, export, conversion and undo", () => {
  const s = drawn(407n, [[10, 10], [50, 10]]);
  s.set_stroke_dash("dash");
  profiled(s);
  assert.match(s.export_svg(false, false, false, false), /^svg:1:/);
  s.end_width_edit();
  assert.equal(s.linked_clone().kind, "done");
  assert.match(s.export_svg(false, false, false, false), /^svg:2:/);
  assert.equal(s.apply_stroke_to_path().kind, "applied");
  s.history_press("undo");
  assert.match(s.export_svg(false, false, false, false), /^svg:2:/);
  s.free();
});

test("a closed dashed profile crosses the seam and converts", () => {
  const s = drawn(408n, [[10, 10], [50, 10], [50, 50], [10, 50], [10, 10]], "lines");
  s.set_stroke_dash_text("8 3");
  profiled(s);
  assert.match(s.export_svg(false, false, false, false), /^svg:1:/);
  s.end_width_edit();
  assert.equal(s.apply_stroke_to_path().kind, "applied");
  s.history_press("undo");
  assert.match(s.export_svg(false, false, false, false), /^svg:1:/);
  s.free();
});

test("round dots follow a variable width profile in SVG", () => {
  const s = drawn(409n, [[10, 10], [50, 10]]);
  s.set_stroke_cap("round");
  s.set_stroke_dash_text("0 3");
  profiled(s);
  assert.match(s.export_svg(false, false, false, false), /^svg:1:/);
  s.free();
});

test("a marker on a variable stroke applies at once and undoes", () => {
  const s = drawn(410n, [[10, 10], [50, 10]]);
  profiled(s);
  s.end_width_edit();
  const before = s.pack();
  const originalSvg = s.export_svg(false, false, false, false);
  s.set_marker_shape("end", "arrow");
  assert.notDeepEqual(s.pack(), before);
  assert.match(s.export_svg(false, false, false, false), /^svg:1:/);
  s.history_press("undo");
  assert.equal(s.export_svg(false, false, false, false), originalSvg);
  s.free();
});

test("a linked clone keeps its source marker and converts without losing it", () => {
  const s = drawn(411n, [[10, 10], [50, 10]]);
  profiled(s);
  s.end_width_edit();
  s.set_marker_shape("end", "arrow");
  assert.match(s.export_svg(false, false, false, false), /^svg:1:/);
  assert.equal(s.linked_clone().kind, "done");
  s.key_down("ArrowRight", false, false, false, false, false, false, 0);
  assert.equal(s.set_clone_style(true).kind, "done");
  const beforeOwn = s.pack();
  s.set_marker_shape("end", "dot");
  assert.notDeepEqual(s.pack(), beforeOwn);
  const cloneSvg = s.export_svg(false, false, false, false);
  assert.match(cloneSvg, /^svg:2:/);
  assert.equal(s.apply_stroke_to_path().kind, "applied");
  s.history_press("undo");
  assert.equal(s.export_svg(false, false, false, false), cloneSvg);
  s.free();
});
