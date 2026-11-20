// Tester cases for `specs/0123-raster-engraving` criterion 11 in the frontend decoders, written
// from the specification: the Image settings group decodes from the session's contract fixture
// (`curvyo-editor-wasm/tests/fixtures/operations_view/options_image.json`), it is absent
// (`null`) for a row without a raster item, and a damaged group is refused rather than shown
// half (the whole view decodes to null, so nothing wrong is drawn).
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

import { decodeOperationsText } from "../src/lib/operationsView.ts";
import { DITHER_MODES, SET_KINDS } from "../src/lib/operationSettingsView.ts";
import { kindButtonName } from "../src/lib/kindButtons.ts";
import { KIND_ORDER, KIND_WORD } from "../src/lib/materialRows.ts";
import { decodeMaterialsText } from "../src/lib/materialsView.ts";
import { decodeTestGridText } from "../src/lib/testGridView.ts";

// Superseded by the Dither on the picture (2026-11-20): the operation's own Dither shows only with a gradient fill, so the
// strip's cases read `options_image_gradient.json`.
const fixture = readFileSync(
  new URL("../../curvyo-editor-wasm/tests/fixtures/operations_view/options_image_gradient.json", import.meta.url),
  "utf8",
);

function withImage(edit: (image: Record<string, unknown> | null) => unknown): string {
  const v = JSON.parse(fixture);
  v.rows[0].settings.options.image = edit(v.rows[0].settings.options.image);
  return JSON.stringify(v);
}

// Superseded by 0135 slice 1: criteria 42 to 44 add Halftone dots and Line screen after the seven
// modes of 0123, so "halftone" is a mode now and no longer damage.
test("criterion 11: the group decodes with the nine modes in order and Grey's Lowest power", () => {
  const view = decodeOperationsText(fixture);
  assert.ok(view, "the contract fixture decodes");
  const image = view.rows[0].settings?.options?.image;
  assert.ok(image, "the group is there for an Image row");
  assert.ok(image.dither, "Superseded by the Dither on the picture (2026-11-20): the gradient fill's Dither");
  assert.deepEqual(
    image.dither.modes.map((m) => m.value),
    ["threshold", "ordered", "floyd_steinberg", "jarvis", "atkinson", "stucki", "grey", "halftone", "line_screen"],
  );
  assert.deepEqual([...DITHER_MODES], image.dither.modes.map((m) => m.value));
  assert.equal(image.dither.dither, "grey");
  // Frage 284 B: Lowest power is the operation's own field, shown while an item burns Grey.
  assert.equal(image.lowestPower?.field, "lowest_power");
  assert.equal(image.lowestPower?.unit, "%");
  assert.deepEqual(
    image.fields.map((f) => f.field),
    ["brightness", "contrast", "gamma"],
  );
  assert.equal(image.fields.find((f) => f.field === "gamma")?.unit, "");
  assert.equal(image.invert, true);
});

test("criterion 11: a row without a raster item has no group", () => {
  const view = decodeOperationsText(withImage(() => null));
  assert.ok(view);
  assert.equal(view.rows[0].settings?.options?.image, null);
});

test("an unknown dither mode or a broken field is refused, never drawn half", () => {
  assert.equal(
    decodeOperationsText(
      withImage((i) => ({ ...i, dither: { ...(i?.dither as Record<string, unknown>), dither: "stochastic" } })),
    ),
    null,
  );
  assert.equal(
    decodeOperationsText(withImage((i) => ({ ...i, fields: [{ field: "sharpen", label: "S", value: 1, unit: "", range: "x" }] }))),
    null,
  );
  assert.equal(
    decodeOperationsText(withImage((i) => ({ ...i, invert: "yes" }))),
    null,
  );
});

// ---- Image kind rework (2026-11-19) ----------------------------------------------------------

function contract(dir: string, name: string): string {
  return readFileSync(
    new URL(`../../curvyo-editor-wasm/tests/fixtures/${dir}/${name}`, import.meta.url),
    "utf8",
  );
}

test("criterion 47: the sixth kind button is named Image and the bar fixture lists it last", () => {
  assert.equal(kindButtonName("image"), "Image");
  const v = decodeOperationsText(contract("operations_view", "select_bar_kinds.json"));
  assert.ok(v?.selectBarKinds);
  assert.equal(v.selectBarKinds.length, 6);
  assert.equal(v.selectBarKinds[5].kind, "image");
});

test("criterion 47: the Image row decodes with kind image in the Marking section", () => {
  const v = decodeOperationsText(fixture);
  assert.ok(v);
  assert.equal(v.rows[0].kind, "image");
  assert.equal(v.rows[0].section, "marking");
});

test("criterion 11: a damaged dither decodes as no pressed segment, the rest of the group kept", () => {
  const v = decodeOperationsText(
    withImage((i) => ({ ...i, dither: { ...(i?.dither as Record<string, unknown>), dither: null } })),
  );
  assert.ok(v, "a damaged dither is shown, not refused");
  const image = v.rows[0].settings?.options?.image;
  assert.ok(image?.dither);
  assert.equal(image.dither.dither, null);
  assert.equal(image.dither.modes.length, 9, "the strip keeps its nine segments (0135 criterion 42)");
  assert.equal(image.fields.length, 3);
});

test("criteria 2, 45: the Materials tab knows four sets per laser, Image last", () => {
  assert.deepEqual([...SET_KINDS], ["cut", "score", "engrave", "image"]);
  assert.deepEqual([...KIND_ORDER], ["cut", "score", "engrave", "image"]);
  assert.equal(KIND_WORD.image, "Image");
  const v = decodeMaterialsText(contract("materials_view", "image_set.json"));
  assert.ok(v, "the Image set fixture decodes");
  const laser = v.unfolded?.lasers[0];
  assert.ok(laser);
  assert.deepEqual(laser.sets.map((s) => s.kind), ["cut", "image"]);
  const image = laser.sets.find((s) => s.kind === "image");
  assert.equal(image?.intervalMm, 0.08, "the Image set has an interval");
  assert.deepEqual([...laser.missing], ["score", "engrave"], "the missing sets are named");
});

test("criterion 44: the Image grid bar decodes with Dither and no Fill; the Engrave grid has neither", () => {
  const image = decodeTestGridText(contract("test_grid_view", "image_ordered.json"));
  assert.ok(image);
  const raw = JSON.parse(contract("test_grid_view", "image_ordered.json"));
  assert.equal(raw.kind ?? raw.bar?.kind, "image");
  assert.ok(!("fill" in raw), "no Fill");
  const engrave = JSON.parse(contract("test_grid_view", "engrave_crosshatch.json"));
  assert.ok(decodeTestGridText(contract("test_grid_view", "engrave_crosshatch.json")));
  assert.equal(engrave.dither ?? null, null, "no Dither on an Engrave grid");
  assert.ok(!("fill" in engrave));
});
