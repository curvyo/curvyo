// Run with `npm test`. The link fields of the Parameters row and the link words of
// `src/lib/sketchInstanceView.ts` against `specs/0099-sketch-pages` (criteria 32 to 35, UX notes
// 12.9.1, 12.9.2 and 12.9.5).

import assert from "node:assert/strict";
import { test } from "node:test";

import { decodeKnobEntry, decodeSketchInstance, linkMarkTitle, linkWords, stopTitle } from "../src/lib/sketchInstanceView.ts";

const row = (link: string, extra: Record<string, unknown> = {}) => ({
  key: "d:c7",
  label: "Radius 1",
  unit: "mm",
  value: 40,
  mixed: false,
  origin: "own",
  sketch_value: 40,
  min: null,
  max: null,
  track_min: 0,
  track_max: 160,
  track_log: true,
  chained: false,
  group: "A",
  partners: ["Radius 2"],
  link,
  ...extra,
});

const decode = (...rows: unknown[]) =>
  decodeSketchInstance(JSON.stringify({ sketch: "1:7", sketch_name: "Bracket", clones: 1, state: "own", no_solution: false, rows }))!;

test("a linked row decodes its group, partners and link state, and says so", () => {
  const [linked, unlinked, none] = decode(row("linked"), row("unlinked"), row("none", { group: null, partners: [] })).rows;
  assert.equal(linked.group, "A");
  assert.deepEqual(linked.partners, ["Radius 2"]);
  assert.equal(linkWords(linked), ", linked with Radius 2");
  assert.equal(linkMarkTitle(linked), "Linked with Radius 2");
  assert.equal(linkWords(unlinked), ", unlinked from Radius 2");
  assert.equal(linkMarkTitle(unlinked), "Unlinked from Radius 2");
  assert.equal(none.group, null);
  assert.equal(linkWords(none), "");
  assert.equal(linkMarkTitle(none), "");
});

test("a row without link fields reads as in no group", () => {
  const [plain] = decode({ ...row("none"), group: undefined, partners: undefined, link: undefined }).rows;
  assert.equal(plain.link, "none");
  assert.equal(plain.group, null);
  assert.deepEqual(plain.partners, []);
});

test("Stop exposing a linked member names the link first", () => {
  assert.equal(
    stopTitle(3, ["Radius 2"]),
    "Leaves the link with Radius 2. Stop exposing: 3 clones go back to the sketch's value",
  );
  assert.equal(stopTitle(0), "Stop exposing: clones can no longer set this value");
});

test("a knob's entry chip decodes, and an empty answer is no chip", () => {
  const entry = decodeKnobEntry(JSON.stringify({ key: "d:c7", label: "Radius 1", unit: "mm", value: 40, glyph: "link", x: 10, y: 20 }));
  assert.deepEqual(entry, { key: "d:c7", label: "Radius 1", unit: "mm", value: 40, glyph: "link", x: 10, y: 20 });
  assert.equal(decodeKnobEntry(""), null);
});
