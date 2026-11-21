// The Crop bar's decoder and the frame's drag arithmetic (`specs/0135-image-optimisation`
// criterion 5, `ux-notes.md` §3.9).
import { test } from "node:test";
import assert from "node:assert/strict";
import { decodeCropBarText, dragCrop, gripPoint, toScreen, toUnit, type Quad } from "../src/lib/cropBar.ts";

const field = (key: string, label: string, value: number) => ({ key, label, unit: "%", value, decimals: 0, range: `${label} is 0 to 99 %.` });
const bar = {
  row: 0,
  pct: [10, 20, 50, 60],
  quad: [200, 0, 0, 100, 40, 30],
  width: 40,
  height: 20,
  fields: [field("left_pct", "Left", 10), field("top_pct", "Top", 20), field("width_pct", "Width", 50), field("height_pct", "Height", 60)],
};

test("the bar decodes, and null or a wrong shape is no bar", () => {
  const view = decodeCropBarText(JSON.stringify(bar));
  assert.ok(view);
  assert.deepEqual(view.pct, [10, 20, 50, 60]);
  assert.equal(view.fields.length, 4);
  assert.equal(decodeCropBarText("null"), null);
  assert.equal(decodeCropBarText(JSON.stringify({ ...bar, pct: [10, 20, 50] })), null);
  assert.equal(decodeCropBarText(JSON.stringify({ ...bar, quad: [1, 2] })), null);
});

test("the frame maps the unit square both ways, under rotation too", () => {
  const turned: Quad = [0, 100, -50, 0, 300, 20];
  for (const q of [bar.quad as Quad, turned]) {
    const [x, y] = toScreen(q, 0.3, 0.7);
    const back = toUnit(q, x, y);
    assert.ok(back);
    assert.ok(Math.abs(back[0] - 0.3) < 1e-9 && Math.abs(back[1] - 0.7) < 1e-9);
  }
  assert.equal(toUnit([0, 0, 0, 0, 0, 0], 1, 1), null);
  assert.deepEqual(gripPoint([10, 20, 50, 60], "br"), [0.6, 0.8]);
  assert.deepEqual(gripPoint([10, 20, 50, 60], "t"), [0.35, 0.2]);
});

test("a corner moves two sides, a side one, the inside the frame; whole %, inside, at least 1 %", () => {
  assert.deepEqual(dragCrop([10, 20, 50, 60], "br", 0.1, 0.05), [10, 20, 60, 65]);
  assert.deepEqual(dragCrop([10, 20, 50, 60], "l", -0.2, 0.5), [0, 20, 60, 60], "stops at the edge");
  assert.deepEqual(dragCrop([10, 20, 50, 60], "r", -0.9, 0), [10, 20, 1, 60], "at least 1 %");
  assert.deepEqual(dragCrop([10, 20, 50, 60], "inside", 0.8, -0.3), [50, 0, 50, 60]);
  const kept = dragCrop([0, 0, 40, 20], "br", 0.2, 0, true);
  assert.deepEqual(kept, [0, 0, 60, 30], "Shift keeps the ratio");
});
