// The zoom and pan rules of the Before and after pane (`specs/0135-image-optimisation` criterion 11).
import { test } from "node:test";
import assert from "node:assert/strict";
import { ZOOM_MAX, ZOOM_MIN, clampZoom, fittedZoom, zoomedAbout } from "../src/lib/beforeAfterZoom.ts";

test("the pane opens fitted: the picture fills most of the viewport, whichever side is tight", () => {
  const wide = fittedZoom({ width: 400, height: 400 }, { width: 800, height: 200 });
  assert.ok(Math.abs(wide - 0.48) < 1e-9, `width is tight: ${wide}`);
  const tall = fittedZoom({ width: 400, height: 400 }, { width: 200, height: 800 });
  assert.ok(Math.abs(tall - 0.48) < 1e-9, `height is tight: ${tall}`);
  assert.equal(fittedZoom({ width: 0, height: 400 }, { width: 10, height: 10 }), 1);
});

test("the zoom is held to 1 % to 3,200 %", () => {
  assert.equal(clampZoom(0), ZOOM_MIN);
  assert.equal(clampZoom(1000), ZOOM_MAX);
  assert.equal(clampZoom(Number.NaN), 1);
});

test("a zoom about a point keeps the dot under the pointer", () => {
  const pan = zoomedAbout({ x: 10, y: -4 }, { x: 50, y: 20 }, 1, 2);
  // The dot at (50, 20) of the viewport was at ((50 - 10) / 1, (20 + 4) / 1) of the picture.
  assert.deepEqual({ x: 50 - (40 * 2), y: 20 - (24 * 2) }, pan);
});

test("100 % is the document's 100 %: a 0.1 mm Interval dot is 0.1 mm at 96 px per inch", async () => {
  const { placedSizeMm, sizeAt100 } = await import("../src/lib/beforeAfterZoom.ts");
  const size = placedSizeMm([40, 0, 0, 20, 5, 5]);
  assert.deepEqual(size, { width: 40, height: 20 });
  const px = sizeAt100(size);
  assert.ok(Math.abs(px.width - (40 * 96) / 25.4) < 1e-9);
  // A rotated placement keeps its side lengths.
  const turned = placedSizeMm([0, 40, -20, 0, 0, 0]);
  assert.deepEqual(turned, { width: 40, height: 20 });
});

test("one Ctrl+wheel notch zooms by 1.25, either way, in pixels or lines", async () => {
  const { wheelZoom } = await import("../src/lib/beforeAfterZoom.ts");
  assert.ok(Math.abs(wheelZoom(1, -100, 0) - 1.25) < 1e-9);
  assert.ok(Math.abs(wheelZoom(1, 100, 0) - 0.8) < 1e-9);
  assert.ok(Math.abs(wheelZoom(1, -3, 1) - 1.25) < 1e-9);
  assert.equal(wheelZoom(32, -100, 0), 32);
});

test("smooth scaling below one device pixel per picture pixel, sharp dots above", async () => {
  const { needsSmoothing } = await import("../src/lib/beforeAfterZoom.ts");
  assert.equal(needsSmoothing(1000, 400, 1), true);
  assert.equal(needsSmoothing(1000, 500, 2), false);
  assert.equal(needsSmoothing(1000, 2000, 1), false);
});
