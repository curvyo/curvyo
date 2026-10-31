// `specs/0089-trace-quality-and-colour` slice M: the words and plain rules of Multicolour in the
// frontend (criteria 25, 26, 29; UX notes §7, §10, §12): Output and Colours as app settings, the
// settings JSON, the palette's order, names and notices, the counts and the success notice.

import { strict as assert } from "node:assert";
import { test } from "node:test";

import {
  BRIGHTNESS,
  MULTICOLOUR,
  PICK_COLOURS,
  REMOVE_BACKGROUND,
  SINGLE_COLOUR,
  TRACE_OUTPUTS,
  TRACE_PARAMS_FALLBACK,
  TRACE_PRESETS_FALLBACK,
  changedAdvancedText,
  coloursText,
  firstChoice,
  readChoice,
  readSummary,
  settingsJson,
  statusText,
  tracedNotice,
  writeChoice,
} from "../src/lib/traceText.ts";
import {
  colourName,
  excludedNote,
  isolatingNote,
  mergedIntoText,
  mergingNote,
  readPalette,
  recolouredText,
  recolourHex,
  sameAsNote,
  swatchName,
  sameColour,
  stripOrder,
} from "../src/lib/tracePaletteText.ts";

function memoryStorage() {
  const store = new Map<string, string>();
  return { store, getItem: (k: string) => store.get(k) ?? null, setItem: (k: string, v: string) => void store.set(k, v) };
}

test("criterion 25: Output is Single colour or Multicolour, an app setting; Colours too", () => {
  assert.deepEqual(TRACE_OUTPUTS, ["Single colour", "Multicolour"]);
  assert.equal(firstChoice().values.output, SINGLE_COLOUR);
  assert.equal(firstChoice().values.colours, null, "Colours starts at Auto");
  const storage = memoryStorage();
  const first = firstChoice();
  writeChoice({ ...first, values: { ...first.values, output: MULTICOLOUR, colours: 9, fixed: ["#FF0000"], excluded: ["#00FF00"] } }, storage);
  const read = readChoice(TRACE_PARAMS_FALLBACK, TRACE_PRESETS_FALLBACK, storage);
  assert.deepEqual([read.values.output, read.values.colours], [MULTICOLOUR, 9]);
  assert.deepEqual([read.values.fixed, read.values.excluded, read.values.merged], [[], [], []], "palette edits belong to the session");
  // Out of range or unreadable: clamped, or Auto.
  storage.store.set("curvyo.trace-values", JSON.stringify({ preset: "logo", values: {}, output: 1, colours: 40 }));
  assert.equal(readChoice(TRACE_PARAMS_FALLBACK, TRACE_PRESETS_FALLBACK, storage).values.colours, 16);
  storage.store.set("curvyo.trace-values", JSON.stringify({ preset: "logo", values: {}, output: 7, colours: "x" }));
  const odd = readChoice(TRACE_PARAMS_FALLBACK, TRACE_PRESETS_FALLBACK, storage);
  assert.deepEqual([odd.values.output, odd.values.colours], [SINGLE_COLOUR, null]);
  // A Multicolour output is an Advanced value that is not the default.
  assert.equal(changedAdvancedText({ ...first.values, output: MULTICOLOUR }), "Changed: Output");
});

test("criteria 26, 29: the settings JSON carries Output, Colours and the palette edits", () => {
  const v = { ...firstChoice().values, output: MULTICOLOUR, colours: 5, fixed: ["#FF0000"], excluded: ["#00FF00"], merged: [["#0000FF", "#FF0000"] as const] };
  const json = JSON.parse(settingsJson(v, 128));
  assert.deepEqual([json.output, json.colours, json.fixed, json.excluded, json.merged], [1, 5, ["#FF0000"], ["#00FF00"], [["#0000FF", "#FF0000"]]]);
  // Fixed palette colours exist for Brightness only (Pick colours takes its swatches); Single
  // colour sends no palette edits.
  assert.deepEqual(JSON.parse(settingsJson({ ...v, method: REMOVE_BACKGROUND }, 128)).fixed, []);
  assert.deepEqual(JSON.parse(settingsJson({ ...v, method: PICK_COLOURS }, 128)).fixed, []);
  const single = JSON.parse(settingsJson({ ...v, output: SINGLE_COLOUR }, 128));
  assert.deepEqual([single.output, single.excluded, single.merged, single.fixed], [0, [], [], []]);
  assert.equal(JSON.parse(settingsJson({ ...v, method: BRIGHTNESS, colours: null }, 128)).colours, null);
});

test("UX notes §7: the strip's order, the swatch names and the palette's words", () => {
  const palette = readPalette(
    JSON.stringify({
      colours: [
        { hex: "#3366CC", fixed: false, pixels: 500 },
        { hex: "#FFF2A8", fixed: false, pixels: 20 },
        { hex: "#D0222A", fixed: true, pixels: 10 },
        { hex: "#111111", fixed: false, pixels: 500 },
      ],
      layers: ["#3366CC", "#111111", "#FFF2A8", "#D0222A"],
      joined: 2,
    }),
  );
  assert.ok(palette);
  assert.deepEqual(
    stripOrder(palette.colours).map((c) => c.hex),
    ["#D0222A", "#111111", "#3366CC", "#FFF2A8"],
    "picked first, then by area, ties by value",
  );
  assert.equal(readPalette(""), null);
  assert.equal(readPalette("{broken"), null);
  assert.deepEqual(
    ["#FFF2A8", "#3366CC", "#D0222A", "#D9D9D9", "#000000", "#FFFFFF", "#7A4A10", "#2A9A4B", "#7A3C9E"].map(colourName),
    ["Pale yellow", "Blue", "Red", "Light grey", "Black", "White", "Brown", "Green", "Purple"],
  );
  assert.equal(coloursText(null, null), "Auto");
  assert.equal(coloursText(null, 6), "Auto  6");
  assert.equal(coloursText(9, 6), "9");
  assert.equal(isolatingNote("#FFF2A8"), "Isolating #FFF2A8");
  assert.equal(mergingNote("#FFF2A8"), "Merging #FFF2A8 into...");
  assert.equal(mergedIntoText("#3366CC"), "Merged into #3366CC");
  assert.equal(excludedNote(2), "2 colours excluded");
  assert.equal(excludedNote(1), "1 colour excluded");
  assert.equal(sameAsNote("#3366CC"), "Same as #3366CC");
  assert.equal(recolouredText("#FFE000", "#FFF2A8"), "#FFE000 (was #FFF2A8)");
  // The Recolour hex field: six digits, with or without "#", any case.
  assert.equal(recolourHex("ffe000"), "#FFE000");
  assert.equal(recolourHex(" #FfE000 "), "#FFE000");
  assert.equal(recolourHex("#FE0"), null);
  assert.equal(recolourHex("#GGE000"), null);
  // A swatch is named by the colour it is drawn in; a recoloured one also by its old hex.
  assert.equal(swatchName("#FFF2A8", "#FFF2A8"), "#FFF2A8 · Pale yellow");
  assert.equal(swatchName("#FFF2A8", "#3366CC"), `#3366CC (was #FFF2A8) · ${colourName("#3366CC")}`);
  // A pick within the merge distance (0.03 of the sRGB cube, 7.65 bytes) is the same colour.
  assert.equal(sameColour(["#3366CC", "#FFF2A8"], "#3567CA"), "#3366CC");
  assert.equal(sameColour(["#D9D9D9"], "#DCD6CF"), null);
});

test("UX notes §10: the counts name the colours, the joined regions and the success notice", () => {
  const wire = [0, 7, 412, 0, 5000, 0, 7];
  const s = statusText(readSummary(wire), 0.42, 100_000, false, null, { colours: 3, joined: 2 });
  assert.equal(s.counts, "3 colours, 7 paths, 412 nodes, 0.4 s");
  assert.equal(s.countsMuted, "2 regions joined");
  const none = statusText(readSummary([0, 0, 0, 0, 0, 0, 0]), 0.1, 100, false, null, { colours: 0, joined: 0 });
  // `ux-notes.md` §22.2: the message is the Advanced status line's, the counts are hidden, once.
  assert.equal(none.message, "No colours found.");
  assert.equal(none.line, "");
  assert.equal(none.counts, "");
  assert.equal(none.canCreate, false);
  assert.equal(tracedNotice(1, 7, 412, 3), "Traced: 3 colours, 7 paths, 412 nodes.");
  assert.equal(tracedNotice(1, 7, 412), "Traced: 7 paths, 412 nodes.");
});
