import assert from "node:assert/strict";
import test from "node:test";
import { createSketchPresentation } from "../src/lib/sketchPresentation.ts";
import { EMPTY_SKETCH_VIEW } from "../src/lib/sketchView.ts";

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
