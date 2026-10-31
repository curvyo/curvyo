import assert from "node:assert/strict";
import test from "node:test";
import { createSketchPresentation } from "../src/lib/sketchPresentation.ts";
import { EMPTY_SKETCH_VIEW } from "../src/lib/sketchView.ts";
import { placeSketchChip } from "../src/lib/readoutPlacement.ts";

test("moving annotations notify their leaf while semantic edits refresh controls", () => {
  const store = createSketchPresentation();
  let controls = 0; let labels = 0;
  store.subscribePanel(() => controls++);
  store.subscribeAnnotations(() => labels++);
  const view = { ...EMPTY_SKETCH_VIEW, workbench: "sketch" as const, active: "a" };
  store.publish(view);
  const panel = store.panel();
  store.publish({ ...view, annotations: [{ id: "d", x: 12, y: 34, text: "10 mm", dimension: true, selected: false, conflicting: false }] });
  assert.equal(store.panel(), panel);
  assert.equal(controls, 1);
  assert.equal(labels, 2);
  assert.equal(store.annotations()[0]?.x, 12);
  store.publish({ ...view, error: "Conflicting constraints", edit_constraint: "d" });
  assert.equal(controls, 2);
  assert.equal(store.panel().error, "Conflicting constraints");
  store.publish({ ...view, active: "b" });
  assert.equal(store.panel().active, "b");
  store.publish(EMPTY_SKETCH_VIEW);
  assert.equal(store.panel().workbench, "vector");
  assert.deepEqual(store.annotations(), []);
});

test("live reference dimension changes still refresh their visible values", () => {
  const store = createSketchPresentation();
  const constraint = { id: "r", kind: "Distance", entities: [], reference: true, expression: null, value: { value: 10, dimension: "Length" as const }, conflicting: false, redundant: false, selected: false };
  store.publish({ ...EMPTY_SKETCH_VIEW, constraints: [constraint] });
  const before = store.panel();
  store.publish({ ...EMPTY_SKETCH_VIEW, constraints: [{ ...constraint, value: { ...constraint.value, value: 11 } }] });
  assert.notEqual(store.panel(), before);
  assert.equal(store.panel().constraints[0]?.value?.value, 11);
});

test("moving canvas chips never refresh Properties and a vanished entry closes the box", () => {
  const store = createSketchPresentation();
  let controls = 0;
  store.subscribePanel(() => controls++);
  const view = { ...EMPTY_SKETCH_VIEW, workbench: "sketch" as const, active: "a" };
  const entry = { kind: "construction" as const, action: null, group: "Line", fields: [{ label: "Length", name: "Sketch Length", suffix: "mm", value: "20" }], x: 10, y: 20, away_x: 1, away_y: 1, pointer_x: 30, pointer_y: 20, reference: false };
  store.publish({ ...view, canvas: { mode_label: { text: "Line", x: 1, y: 2, away_x: 1, away_y: 1, hint: null }, entry } });
  store.publish({ ...view, canvas: { mode_label: { text: "Line", x: 5, y: 6, away_x: -1, away_y: -1, hint: null }, entry: { ...entry, x: 40 } } });
  assert.equal(controls, 1);
  assert.equal(store.canvas().mode_label?.x, 5);
  store.setEntryOpen(true);
  assert.equal(store.entryOpen(), true);
  store.publish({ ...view, canvas: { mode_label: null, entry: null } });
  assert.equal(store.entryOpen(), false);
});

test("sketch chips keep off the rail, the canvas edges and the live cursor point", () => {
  const chip = { width: 100, height: 30 };
  const canvas = { width: 800, height: 600 };
  assert.deepEqual(placeSketchChip({ x: 400, y: 300 }, { x: 1, y: 1 }, chip, canvas, 10, 128), { left: 410, top: 310 });
  assert.deepEqual(placeSketchChip({ x: 400, y: 300 }, { x: -1, y: -1 }, chip, canvas, 10, 128), { left: 290, top: 260 });
  assert.equal(placeSketchChip({ x: 140, y: 300 }, { x: -1, y: 1 }, chip, canvas, 10, 128).left, 150);
  assert.deepEqual(placeSketchChip({ x: 790, y: 590 }, { x: 1, y: 1 }, chip, canvas, 10, 128), { left: 680, top: 550 });
  assert.deepEqual(placeSketchChip({ x: 400, y: 300 }, { x: 1, y: 1 }, chip, canvas, 14, 128, { x: 450, y: 320 }), { left: 286, top: 256 });
});
