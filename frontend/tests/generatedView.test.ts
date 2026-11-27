// Run with `npm test`. The decoder and the field words of `src/lib/generatedView.ts` against
// `specs/0141-parametric-generators` (criteria 1 to 3, UX notes §3 and §4).

import assert from "node:assert/strict";
import { test } from "node:test";

import {
  canvasNoticeLook,
  decodeGeneratedView,
  decodeKinds,
  parseNumber,
  rowText,
  shownNumber,
  rowRange,
  stepRowText,
  unitOf,
  unitOfRow,
  positionOf,
  automaticWords,
  sliderEnd,
  valueAt,
  stepValue,
  dragSpan,
} from "../src/lib/generatedView.ts";

const MODULE = {
  group: "Teeth",
  param: {
    key: "module",
    label: "Module",
    kind: { kind: "length", min: 0.1, max: 25, step: 0.1, decimals: 4 },
  },
  value: { kind: "length", value: 2 },
};
const TEETH = {
  group: "Teeth",
  param: { key: "teeth", label: "Teeth", kind: { kind: "count", min: 6, max: 200 } },
  value: { kind: "count", value: 24 },
};
const WEB = {
  group: "Web",
  param: {
    key: "web",
    label: "Web",
    kind: { kind: "choice", options: [{ id: "solid", label: "Solid" }, { id: "spokes", label: "Spokes" }] },
  },
  value: { kind: "choice", value: "solid" },
};

const json = (rows: unknown[], known = true) =>
  JSON.stringify({ id: "1", label: "Gear", known, rows });

test("the block decodes its rows in order with their values and kinds", () => {
  const view = decodeGeneratedView(json([MODULE, TEETH, WEB]));
  assert.ok(view);
  assert.equal(view.label, "Gear");
  assert.deepEqual(view.rows.map((row) => row.key), ["module", "teeth", "web"]);
  assert.equal(view.rows[2].value, "solid");
  assert.equal(view.rows[1].kind.kind, "count");
});

test("an unknown generator, no selection and broken text show no block", () => {
  assert.equal(decodeGeneratedView(json([MODULE], false)), null);
  assert.equal(decodeGeneratedView("null"), null);
  assert.equal(decodeGeneratedView("not json"), null);
});

test("lengths show their decimals trimmed, counts whole, with the unit inside the field", () => {
  const view = decodeGeneratedView(json([MODULE, TEETH]));
  assert.ok(view);
  assert.equal(rowText(view.rows[0]), "2");
  assert.equal(rowText(view.rows[1]), "24");
  assert.equal(unitOf(view.rows[0].kind), "mm");
  assert.equal(unitOf(view.rows[1].kind), undefined);
  assert.equal(shownNumber(2.0833, 4), "2.0833");
  assert.equal(shownNumber(50, 2), "50");
  assert.equal(shownNumber(0.1, 2), "0.1");
});

test("a typed decimal comma is a point and anything else is not a number", () => {
  assert.equal(parseNumber("2,5"), 2.5);
  assert.equal(parseNumber(" 48 "), 48);
  assert.equal(parseNumber("abc"), null);
  assert.equal(parseNumber(""), null);
  assert.equal(parseNumber("1e3"), null);
});

test("the arrow keys step by the row's step, Shift by ten, a count by one", () => {
  const view = decodeGeneratedView(json([MODULE, TEETH]));
  assert.ok(view);
  assert.equal(stepRowText(view.rows[0], "2", "ArrowUp", false, false), "2.1");
  assert.equal(stepRowText(view.rows[0], "2", "ArrowDown", true, false), "1");
  assert.equal(stepRowText(view.rows[0], "2", "ArrowUp", false, true), "2.01");
  assert.equal(stepRowText(view.rows[1], "24", "ArrowUp", false, true), "25");
  assert.equal(stepRowText(view.rows[1], "24", "ArrowDown", true, false), "14");
  assert.equal(stepRowText(view.rows[1], "24", "a", false, false), null);
});

test("a number row reports its value and bounds as a spin button", () => {
  const view = decodeGeneratedView(json([MODULE, TEETH, WEB]));
  assert.ok(view);
  assert.equal(rowRange(view.rows[2]), null);
  assert.deepEqual(rowRange(view.rows[0]), { now: 2, min: 0.1, max: 25 });
  assert.deepEqual(rowRange(view.rows[1]), { now: 24, min: 6, max: 200 });
});

test("the bar's kinds decode in registry order", () => {
  const kinds = decodeKinds(
    JSON.stringify([
      { id: "curvyo.generators/box", label: "Box", icon: "Box" },
      { id: "curvyo.generators/gear", label: "Gear", icon: "Cog" },
    ]),
  );
  assert.deepEqual(kinds.map((kind) => kind.label), ["Box", "Gear"]);
  assert.deepEqual(decodeKinds("nope"), []);
});

test("a linear track maps positions to values on the step grid and past the ends", () => {
  const row = {
    group: "Material",
    key: "thickness",
    label: "Thickness",
    kind: { kind: "length" as const, min: 0.1, max: 50, step: 0.1, decimals: 2 },
    track: { from: 0.1, to: 12, curve: "linear" as const },
    defaultValue: 3,
    value: 3,
  };
  assert.equal(positionOf(row, 0.1), 0);
  assert.equal(positionOf(row, 12), 1);
  assert.equal(valueAt(row, 0, "normal"), 0.1);
  assert.equal(valueAt(row, 1, "normal"), 12);
  assert.equal(valueAt(row, 0.5, "coarse"), 6, "the coarse grid rounds to whole millimetres");
  assert.equal(valueAt(row, 10, "normal"), 50, "a drag past the end stops at the schema maximum");
  assert.equal(stepValue(row, 3, 1, "fine"), 3.01);
  assert.deepEqual(dragSpan(row), { min: 0, limit: positionOf(row, 50) });
});

test("a log track follows Width's curve and its inverse", () => {
  const row = {
    group: "Size",
    key: "inner_width",
    label: "Inner width",
    kind: { kind: "length" as const, min: 10, max: 2000, step: 1, decimals: 2 },
    track: { from: 10, to: 500, curve: "log" as const },
    defaultValue: 100,
    value: 100,
  };
  assert.ok(Math.abs(positionOf(row, 10)) < 1e-12);
  assert.ok(Math.abs(positionOf(row, 500) - 1) < 1e-12);
  for (const value of [10, 25, 100, 321, 500]) {
    assert.equal(valueAt(row, positionOf(row, value), "normal"), value);
  }
  assert.ok(positionOf(row, 100) > 0.6 && positionOf(row, 100) < 0.7, "a fifth of the span takes two thirds of the bar");
});

test("a count steps in whole numbers and ignores the fine grid", () => {
  const row = {
    group: "Teeth",
    key: "teeth",
    label: "Teeth",
    kind: { kind: "count" as const, min: 6, max: 200 },
    track: { from: 6, to: 100, curve: "linear" as const },
    defaultValue: 24,
    value: 24,
  };
  assert.equal(valueAt(row, 0.5, "normal"), 53);
  assert.equal(stepValue(row, 24, 1, "fine"), 25);
  assert.equal(stepValue(row, 24, 1, "coarse"), 34);
  assert.equal(stepValue(row, 199, 1, "coarse"), 200);
});

test("an automatic row shows the value the generator used and says so", () => {
  const view = decodeGeneratedView(
    JSON.stringify({
      label: "Rounded box",
      known: true,
      rows: [
        {
          group: "Slits",
          param: {
            key: "slit_width",
            label: "Slit width",
            kind: { kind: "length", min: 0.2, max: 5, step: 0.05, decimals: 3, automatic: true },
            track: { from: 0.2, to: 2, curve: "linear" },
            default: { kind: "length", value: 0 },
          },
          value: { kind: "length", value: 0 },
          used: { kind: "length", value: 0.3 },
        },
        {
          group: "Lid",
          param: {
            key: "lid_thickness",
            label: "Lid thickness",
            kind: { kind: "length", min: 0.1, max: 50, step: 0.1, decimals: 2, automatic: true },
            default: { kind: "length", value: 0 },
          },
          value: { kind: "length", value: 4 },
          used: null,
        },
      ],
    }),
  );
  assert.ok(view);
  const [slit, lid] = view.rows;
  assert.equal(rowText(slit), "0.3");
  assert.equal(automaticWords(slit), ", automatic");
  assert.equal(rowText(lid), "4");
  assert.equal(automaticWords(lid), "");
  assert.equal(automaticWords({ ...slit, key: "lid_thickness" }), ", follows wall thickness");
});

test("a row at a limit that is a shape says so in its words", () => {
  const view = decodeGeneratedView(
    JSON.stringify({
      label: "Rounded box",
      known: true,
      rows: [
        {
          group: "Size",
          param: {
            key: "corner_radius",
            label: "Corner radius",
            kind: { kind: "length", min: 1, max: 1000, step: 0.5, decimals: 2 },
            default: { kind: "length", value: 15 },
          },
          value: { kind: "length", value: 40 },
          state: "full round",
        },
      ],
    }),
  );
  assert.ok(view);
  assert.equal(automaticWords(view.rows[0]), ", full round");
});

test("the slider's end is the live limit when it is lower than the track's end", () => {
  const view = decodeGeneratedView(
    JSON.stringify({
      label: "Rounded box",
      known: true,
      rows: [
        {
          group: "Lid",
          param: {
            key: "hollow_width",
            label: "Hollow width",
            kind: { kind: "length", min: 5, max: 200, step: 1, decimals: 2 },
            track: { from: 5, to: 60, curve: "linear" },
            default: { kind: "length", value: 16 },
          },
          value: { kind: "length", value: 16 },
          limit: { kind: "length", value: 40 },
        },
      ],
    }),
  );
  assert.ok(view);
  assert.equal(sliderEnd(view.rows[0]), 40);
  assert.equal(sliderEnd({ ...view.rows[0], limit: null }), 60);
});

test("a refusal shows in the refusal look for 8 s, a warning as a status for 5 s (0149 UX §1.2, §6)", () => {
  assert.deepEqual(canvasNoticeLook("The outline crosses itself. Nothing was changed."), {
    kind: "alert",
    ms: 8000,
  });
  assert.deepEqual(canvasNoticeLook("Select one closed outline. Nothing was changed."), { kind: "alert", ms: 8000 });
  assert.deepEqual(canvasNoticeLook("The corner between pieces 4 and 5 is very sharp: 9.5°."), { kind: "status", ms: 5000 });
  assert.deepEqual(canvasNoticeLook("No place for the finger hollow on this lid."), { kind: "status", ms: 5000 });
});

test("the Joint spread count is a percentage, other counts have no unit", () => {
  const spread = {
    group: "Advanced",
    param: { key: "joint_spread", label: "Joint spread", kind: { kind: "count", min: 0, max: 100 } },
    value: { kind: "count", value: 25 },
  };
  const view = decodeGeneratedView(json([spread, TEETH]));
  assert.ok(view);
  assert.equal(unitOfRow(view.rows[0]), "%");
  assert.equal(unitOfRow(view.rows[1]), "");
});

test("the Parts group decodes its rows with their state and an old session has none", () => {
  const withParts = JSON.stringify({
    id: "1",
    label: "Rounded box",
    known: true,
    rows: [],
    parts: [
      { key: "strip", name: "Strip", detached: false, operation: "Cut 1", can_detach: true },
      { key: "lid", name: "Lid", detached: true, operation: "Cut 1 · Lid", can_detach: true },
    ],
  });
  const view = decodeGeneratedView(withParts);
  assert.ok(view);
  assert.deepEqual(
    view.parts.map((part) => [part.key, part.detached, part.operation, part.canDetach]),
    [
      ["strip", false, "Cut 1", true],
      ["lid", true, "Cut 1 · Lid", true],
    ],
  );
  const plain = decodeGeneratedView(json([MODULE]));
  assert.ok(plain);
  assert.deepEqual(plain.parts, []);
});

// `specs/0144-qr-and-barcodes`: the text and percent rows and the lasting reason line.
const CONTENT = {
  group: "Content",
  param: { key: "content", label: "Content", kind: { kind: "text", multiline: true } },
  value: { kind: "text", value: "two\nlines" },
  problem: "Enter a text to encode.",
};
const FILL = {
  group: "Modules",
  param: { key: "module_fill", label: "Module fill", kind: { kind: "percent", min: 50, max: 100 } },
  value: { kind: "percent", value: 90 },
};

test("a text row keeps its line breaks and its reason; a row without one has none", () => {
  const view = decodeGeneratedView(json([CONTENT, FILL]));
  assert.ok(view);
  assert.equal(view.rows[0].value, "two\nlines");
  assert.equal(view.rows[0].kind.kind, "text");
  assert.equal(view.rows[0].problem, "Enter a text to encode.");
  assert.equal(view.rows[1].problem, null);
});

test("a percent row is a number with a percent sign, whole steps and its own bounds", () => {
  const view = decodeGeneratedView(json([FILL]));
  assert.ok(view);
  const [row] = view.rows;
  assert.equal(unitOf(row.kind), "%");
  assert.equal(rowText(row), "90");
  assert.deepEqual(rowRange(row), { now: 90, min: 50, max: 100 });
  assert.equal(positionOf(row, 75), 0.5);
  assert.equal(valueAt(row, 1, "normal"), 100);
  assert.equal(valueAt(row, 0, "normal"), 50);
  assert.equal(stepValue(row, 90, 15, "normal"), 100, "held inside the bounds");
  assert.equal(stepRowText(row, "90", "ArrowUp", false, false), "91");
});

const ADJUST = {
  group: "Advanced",
  param: { key: "bar_adjust", label: "Bar width adjust", kind: { kind: "percent", min: -30, max: 30 } },
  value: { kind: "percent", value: 5 },
};
const WIDE = {
  group: "Size",
  param: {
    key: "wide_narrow",
    label: "Wide:narrow",
    kind: { kind: "ratio", min: 2, max: 3, step: 0.1, decimals: 1 },
  },
  value: { kind: "ratio", value: 2.5 },
};
const FONT = {
  group: "Text",
  param: { key: "font", label: "Font", kind: { kind: "font" }, default: { kind: "font", value: null } },
  value: { kind: "font", value: null },
};

test("a signed percent shows its plus and steps across zero (Bar width adjust)", () => {
  const view = decodeGeneratedView(json([ADJUST]));
  assert.ok(view);
  const [row] = view.rows;
  assert.equal(rowText(row), "+5");
  assert.equal(rowText({ ...row, value: -5 }), "-5");
  assert.equal(rowText({ ...row, value: 0 }), "0");
  assert.deepEqual(rowRange(row), { now: 5, min: -30, max: 30 });
  assert.equal(stepValue(row, 0, -3, "normal"), -3);
  assert.equal(stepValue(row, -29, -5, "normal"), -30, "held inside the bounds");
  assert.equal(valueAt(row, 0, "normal"), -30);
  assert.equal(parseNumber("-5"), -5);
  assert.equal(parseNumber("+5"), 5);
});

test("a ratio row has one decimal, its own step and a ': 1' unit", () => {
  const view = decodeGeneratedView(json([WIDE]));
  assert.ok(view);
  const [row] = view.rows;
  assert.equal(unitOf(row.kind), ": 1");
  assert.equal(rowText(row), "2.5");
  assert.equal(stepRowText(row, "2.5", "ArrowUp", false, false), "2.6");
  assert.equal(stepValue(row, 2.9, 3, "normal"), 3, "held at the maximum");
});

test("a font row shows the face name, the included sans for none", () => {
  const view = decodeGeneratedView(json([FONT]));
  assert.ok(view);
  assert.equal(view.rows[0].value, "Source Sans 3 Regular");
  const named = JSON.stringify({
    id: "1",
    label: "EAN-13",
    known: true,
    rows: [{ ...FONT, value: { kind: "font", value: { family: "Inter", style: "Bold", face_hash: "ab", face_index: 0 } } }],
    family: { label: "Symbology", current: "curvyo.codes/ean-13", kinds: [{ id: "curvyo.codes/qr", label: "QR code" }] },
    loading: true,
    font_missing: true,
  });
  const decoded = decodeGeneratedView(named);
  assert.ok(decoded);
  assert.equal(decoded.rows[0].value, "Inter Bold");
  assert.equal(decoded.family?.label, "Symbology");
  assert.equal(decoded.loading, true);
  assert.equal(decoded.fontMissing, true);
});

test("the Code segment is one entry that carries its family and its loading state", () => {
  const kinds = decodeKinds(
    JSON.stringify([{ id: "curvyo.codes/ean-13", label: "EAN-13", segment: "Code", source: false, icon: "Barcode", family: "code", loading: true }]),
  );
  assert.equal(kinds[0].segment, "Code");
  assert.equal(kinds[0].family, "code");
  assert.equal(kinds[0].loading, true);
});
