// Run with `npm test`. The decoder and the row adapter of `src/lib/sketchInstanceView.ts` against
// `specs/0099-sketch-pages` (criterion 24, UX notes 12.2).

import assert from "node:assert/strict";
import { test } from "node:test";

import { positionOf, valueAt, dragSpan } from "../src/lib/generatedView.ts";
import { asGeneratedRow, decodeSketchInstance, followsWords, resetName } from "../src/lib/sketchInstanceView.ts";

const TEXT = JSON.stringify({
  sketch: "1:7",
  sketch_name: "Bracket",
  clones: 2,
  state: "mixed",
  no_solution: true,
  rows: [
    {
      key: "p:width",
      label: "width",
      unit: "mm",
      value: null,
      mixed: true,
      origin: "mixed",
      sketch_value: 60,
      min: 20,
      max: 100,
      track_min: 20,
      track_max: 100,
      track_log: false,
      chained: false,
    },
    {
      key: "d:c7",
      label: "Radius 1",
      unit: "mm",
      value: 4,
      mixed: false,
      origin: "inherited",
      sketch_value: 5,
      min: null,
      max: null,
      track_min: 0,
      track_max: 20,
      track_log: true,
      chained: true,
    },
  ],
});

test("the decoder reads the pill state, the no-solution flag and every row", () => {
  const view = decodeSketchInstance(TEXT);
  assert.ok(view);
  assert.equal(view.sketchName, "Bracket");
  assert.equal(view.state, "mixed");
  assert.equal(view.noSolution, true);
  assert.equal(view.rows.length, 2);
  assert.equal(view.rows[0]?.value, null);
  assert.equal(view.rows[1]?.min, null);
  assert.equal(decodeSketchInstance("null"), null);
  assert.equal(decodeSketchInstance("not json"), null);
});

test("a row with both limits runs linearly between them and a typed value reaches them", () => {
  const view = decodeSketchInstance(TEXT);
  const row = asGeneratedRow(view!.rows[0]!);
  assert.equal(positionOf(row, 20), 0);
  assert.equal(positionOf(row, 100), 1);
  assert.equal(valueAt(row, 0.5, "normal"), 60);
  assert.ok(dragSpan(row).limit >= 1);
});

test("a length without limits runs log from 0 and may overshoot the track", () => {
  const view = decodeSketchInstance(TEXT);
  const row = asGeneratedRow(view!.rows[1]!);
  assert.equal(row.track?.curve, "log");
  assert.ok(positionOf(row, 40) > 1, "past the track's end");
});

test("the reset name says the original's value up a chain and the follow words say where a value comes from", () => {
  const view = decodeSketchInstance(TEXT)!;
  assert.equal(resetName(view.rows[0]!), "Back to the sketch's value");
  assert.equal(resetName(view.rows[1]!), "Back to the original's value");
  assert.equal(followsWords(view.rows[1]!), ", original's value");
  assert.equal(followsWords({ ...view.rows[0]!, origin: "own" }), "");
});
