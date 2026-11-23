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
