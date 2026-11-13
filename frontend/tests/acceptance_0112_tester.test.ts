// Tester acceptance cases for `specs/0112-test-grid-and-materials` on the view-only criteria: the
// dimmed tool's tooltip (criterion 1), the bar's uncaptioned row 1 with the read-only test id and
// the Crosshatch toggle and no sliders (3, 4), the test row's photo glyph and state line (22), the
// test detail (23, 24), the photo scale, type and Camera rule (27), "Photo not found" (31) and
// nothing leaving the machine (33). Written from the spec and the UX notes; they use the pure
// helpers and the shared view fixtures, and read the component sources only for the attributes
// the criteria name.
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test } from "node:test";

import { decodeMaterialsText } from "../src/lib/materialsView.ts";
import { PHOTO_MAX_PX, encodeType, isPhotoFile, scaledSize } from "../src/lib/photoSize.ts";
import { decodeTestGridText } from "../src/lib/testGridView.ts";

const FIXTURES = new URL("../../curvyo-editor-wasm/tests/fixtures/", import.meta.url);
const SRC = new URL("../src/", import.meta.url);

function source(path: string): string {
  return readFileSync(new URL(path, SRC), "utf8");
}

function fixture(path: string): string {
  return readFileSync(new URL(path, FIXTURES), "utf8");
}

test("c1: the dimmed Test grid tool says Laser pages only.", () => {
  const rail = source("components/ToolRail.tsx");
  assert.match(rail, /unavailable=\{[^}]*"Laser pages only\."/);
});

test("c3: row 1 shows the test id read-only with the tooltip Test id; Crosshatch is # for Engrave", () => {
  const bar = source("components/TestGridBar.tsx");
  assert.match(bar, /title="Test id"/);
  assert.match(bar, /title="Crosshatch"/);
  assert.match(bar, /aria-label="Crosshatch"/);
  assert.match(bar, /<Hash\b/, "the toggle shows the # glyph");
  const engrave = decodeTestGridText(fixture("test_grid_view/engrave_crosshatch.json"));
  assert.ok(engrave);
  assert.equal(typeof engrave.crosshatch, "boolean");
  const cut = decodeTestGridText(fixture("test_grid_view/lasered.json"));
  assert.ok(cut);
  assert.equal(cut.crosshatch, null, "no Crosshatch on a Cut grid");
  assert.match(cut.id ?? "", /^[0-9A-Z]{5}$/);
});

test("c4: plain number fields, no sliders", () => {
  const bar = source("components/TestGridBar.tsx");
  assert.doesNotMatch(bar, /type="range"/);
  assert.doesNotMatch(bar, /Slider/);
});

test("c5: a grid that does not fit carries its cause", () => {
  const v = decodeTestGridText(fixture("test_grid_view/does_not_fit.json"));
  assert.ok(v);
  const cause = v.fit ?? v.invalid?.cause ?? "";
  assert.match(String(cause), /^Grid is \d+(\.\d+)? mm (wide|high), the bed is \d+(\.\d+)? mm\.$/);
});

test("c22: a test row has the state line and the photo glyph flag", () => {
  const v = decodeMaterialsText(fixture("materials_view/test_with_photo.json"));
  assert.ok(v && v.unfolded);
  const tests = v.unfolded.tests;
  assert.ok(tests.some((t) => t.photo));
  assert.ok(tests.some((t) => !t.photo));
  for (const t of tests) {
    assert.match(t.line, t.lasered ? /^Lasered \d{1,2} [A-Z][a-z]{2} · / : /^Not lasered · /);
  }
});

test("c23 c24: the test detail holds the axes, the best cell and the set line", () => {
  const v = decodeMaterialsText(fixture("materials_view/test_with_photo.json"));
  assert.ok(v && v.unfolded && v.unfolded.test);
  const t = v.unfolded.test;
  assert.match(t.id, /^[0-9A-Z]{5}$/);
  assert.ok(t.best);
  assert.match(t.setLine ?? "", /^Cut set of .+: [\d.]+ %, \d+ mm\/min, \d+ ×$/);
  assert.match(t.place.label, /^(Place on .+|Select on page)$/);
});

test("c27: at most 600 px on the long side, JPEG unless PNG, JPEG or PNG files only", () => {
  assert.equal(PHOTO_MAX_PX, 600);
  assert.deepEqual(scaledSize(1200, 900), { width: 600, height: 450 });
  assert.deepEqual(scaledSize(900, 1800), { width: 300, height: 600 });
  assert.deepEqual(scaledSize(400, 300), { width: 400, height: 300 }, "never scaled up");
  assert.deepEqual(scaledSize(601, 1), { width: 600, height: 1 });
  assert.equal(encodeType("image/png").mime, "image/png");
  assert.equal(encodeType("image/jpeg").mime, "image/jpeg");
  assert.equal(encodeType("").mime, "image/jpeg", "the camera frame is JPEG");
  assert.ok(isPhotoFile("image/jpeg") && isPhotoFile("image/png"));
  assert.ok(!isPhotoFile("image/gif") && !isPhotoFile("image/webp"));
});

test("c27: Camera is only offered with a camera and permission", () => {
  const photo = source("components/panel/TestPhoto.tsx");
  assert.match(photo, /cameraOffered/);
  assert.match(photo, /\{cameraOffered\s*(\?|&&)/, "the Camera button sits behind the offer");
});

test("c31: a missing or damaged photo shows Photo not found", () => {
  const photo = source("components/panel/TestPhoto.tsx");
  assert.match(photo, /Photo not found/);
});

test("c33: the photo, camera and materials code never talks to the network", () => {
  for (const path of [
    "components/panel/TestPhoto.tsx",
    "components/panel/PhotoCrop.tsx",
    "hooks/useMaterials.ts",
    "hooks/useTestGridTool.ts",
    "lib/materialPhotoStore.ts",
    "lib/materialHostWrites.ts",
  ]) {
    const text = source(path);
    assert.doesNotMatch(text, /XMLHttpRequest|WebSocket|sendBeacon|EventSource/, path);
    // fetch only of a local object URL (the crop copy of the stored photo).
    for (const m of text.matchAll(/fetch\(([^)]*)\)/g)) {
      assert.match(m[1], /^url$/, `${path}: fetch(${m[1]})`);
    }
  }
});
