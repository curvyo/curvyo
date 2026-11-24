// Run with `npm test`. The decoder and the field words of `src/lib/generatedView.ts` against
// `specs/0141-parametric-generators` (criteria 1 to 3, UX notes §3 and §4).

import assert from "node:assert/strict";
import { test } from "node:test";

import {
  decodeGeneratedView,
  decodeKinds,
  parseNumber,
  rowText,
  shownNumber,
  rowRange,
  stepRowText,
  unitOf,
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
