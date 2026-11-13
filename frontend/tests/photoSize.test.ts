// The photo arithmetic (`specs/0112` criterion 27): 600 px long side, encoding by type, the crop
// frame and its handles.
import { test } from "node:test";
import assert from "node:assert/strict";
import {
  CROP_MIN_PX,
  arrowStep,
  encodeType,
  isPhotoFile,
  moveFrame,
  resizeFrame,
  scaledSize,
  shownHeight,
  sizeReadout,
  wholeFrame,
} from "../src/lib/photoSize.ts";
import { bestStatus, cellName, cellWidth, moveCell, sparseHeader } from "../src/lib/testGridResult.ts";
import { newMaterialRefusal } from "../src/lib/materialRows.ts";

test("a photo is scaled to 600 px on its long side and never up", () => {
  assert.deepEqual(scaledSize(4000, 3000), { width: 600, height: 450 });
  assert.deepEqual(scaledSize(3000, 4000), { width: 450, height: 600 });
  assert.deepEqual(scaledSize(600, 400), { width: 600, height: 400 });
  assert.deepEqual(scaledSize(300, 200), { width: 300, height: 200 });
  assert.deepEqual(scaledSize(10000, 1), { width: 600, height: 1 });
});

test("PNG stays PNG, anything else is JPEG at 0.85; only JPEG and PNG files are taken", () => {
  assert.deepEqual(encodeType("image/png"), { mime: "image/png", quality: undefined });
  assert.deepEqual(encodeType("image/jpeg"), { mime: "image/jpeg", quality: 0.85 });
  assert.deepEqual(encodeType(""), { mime: "image/jpeg", quality: 0.85 });
  assert.equal(isPhotoFile("image/jpeg"), true);
  assert.equal(isPhotoFile("image/png"), true);
  assert.equal(isPhotoFile("image/gif"), false);
  assert.equal(isPhotoFile("application/pdf"), false);
});

test("the crop frame moves inside the image and resizes by its handle", () => {
  const bounds = { width: 600, height: 470 };
  const whole = wholeFrame(600, 470);
  assert.equal(sizeReadout(whole.width, whole.height), "600 × 470 px");
  assert.deepEqual(moveFrame(whole, 5, 5, bounds), whole);
  const frame = { x: 100, y: 100, width: 200, height: 100 };
  assert.deepEqual(moveFrame(frame, 10, -200, bounds), { x: 110, y: 0, width: 200, height: 100 });
  assert.deepEqual(moveFrame(frame, 1000, 1000, bounds), { x: 400, y: 370, width: 200, height: 100 });
  assert.deepEqual(resizeFrame(frame, "se", 20, 10, bounds), { x: 100, y: 100, width: 220, height: 110 });
  assert.deepEqual(resizeFrame(frame, "nw", -20, -10, bounds), { x: 80, y: 90, width: 220, height: 110 });
  assert.deepEqual(resizeFrame(frame, "e", -1000, 0, bounds), { x: 100, y: 100, width: CROP_MIN_PX, height: 100 });
  assert.deepEqual(resizeFrame(frame, "w", -1000, 0, bounds), { x: 0, y: 100, width: 300, height: 100 });
  assert.deepEqual(resizeFrame(frame, "s", 0, 1000, bounds), { x: 100, y: 100, width: 200, height: 370 });
  assert.deepEqual(resizeFrame(frame, "n", 0, 1000, bounds), { x: 100, y: 184, width: 200, height: 16 });
});

test("arrow keys step 1 px, Shift 10, other keys none", () => {
  assert.deepEqual(arrowStep("ArrowLeft", false), { dx: -1, dy: 0 });
  assert.deepEqual(arrowStep("ArrowDown", true), { dx: 0, dy: 10 });
  assert.equal(arrowStep("Enter", false), null);
});

test("a shown photo is 228 wide, its height by ratio, at most 228", () => {
  assert.equal(shownHeight(600, 470), 179);
  assert.equal(shownHeight(300, 600), 228);
  assert.equal(shownHeight(0, 0), 228);
});

test("the judge table: cell width, sparse header, names, status and the roving keys", () => {
  assert.equal(cellWidth(5), 24);
  assert.equal(cellWidth(8), 24);
  assert.equal(cellWidth(12), 17);
  assert.equal(sparseHeader(["20", "40"], 24), false);
  assert.equal(sparseHeader(["100", "1950"], 17), true);
  const columns = { title: "Power %", unit: " %", values: ["20", "60"] };
  const rows = { title: "mm/min", unit: " mm/min", values: ["600", "1950"] };
  assert.equal(cellName(columns, rows, { column: 1, row: 1 }, false), "60, 1950");
  assert.equal(cellName(columns, rows, { column: 1, row: 1 }, true), "60, 1950, best cell");
  assert.equal(bestStatus(columns, rows, { column: 1, row: 0 }), "Best cell: 60 %, 600 mm/min.");
  assert.deepEqual(moveCell({ column: 0, row: 0 }, "ArrowLeft", false, 5, 5), { column: 0, row: 0 });
  assert.deepEqual(moveCell({ column: 0, row: 0 }, "ArrowRight", false, 5, 5), { column: 1, row: 0 });
  assert.deepEqual(moveCell({ column: 2, row: 2 }, "End", false, 5, 5), { column: 4, row: 2 });
  assert.deepEqual(moveCell({ column: 2, row: 2 }, "End", true, 5, 5), { column: 4, row: 4 });
  assert.deepEqual(moveCell({ column: 2, row: 2 }, "Home", true, 5, 5), { column: 0, row: 0 });
  assert.equal(moveCell({ column: 2, row: 2 }, "x", false, 5, 5), null);
});

test("the New material form refuses an empty name and a thickness outside 0.1 to 100", () => {
  assert.equal(newMaterialRefusal("Birch", "3"), null);
  assert.deepEqual(newMaterialRefusal("  ", "3"), { field: "name", text: "Enter a name." });
  assert.deepEqual(newMaterialRefusal("x".repeat(61), "3"), { field: "name", text: "Enter a name." });
  assert.deepEqual(newMaterialRefusal("Birch", ""), { field: "thickness", text: "Thickness is 0.1 to 100 mm." });
  assert.deepEqual(newMaterialRefusal("Birch", "0.05"), { field: "thickness", text: "Thickness is 0.1 to 100 mm." });
  assert.deepEqual(newMaterialRefusal("Birch", "101"), { field: "thickness", text: "Thickness is 0.1 to 100 mm." });
  assert.equal(newMaterialRefusal("Birch", "3,5"), null);
});
