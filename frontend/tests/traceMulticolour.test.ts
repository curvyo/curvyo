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
  editChoice,
  firstChoice,
  readChoice,
  readSummary,
  settingsJson,
  statusText,
  tracedTooltip,
  writeChoice,
} from "../src/lib/traceText.ts";
import {
  PALETTE_HINT,
  colourName,
  deleteFromPalette,
  excludedNote,
  paletteKey,
  paletteSwatchLabel,
  paletteTooltip,
  readPalette,
  sameAsNote,
  sameColour,
  stripOrder,
} from "../src/lib/tracePaletteText.ts";
import { viewValues } from "../src/lib/traceMaskCaches.ts";

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
  assert.deepEqual([read.values.fixed, read.values.excluded, read.values.keptPalette], [[], [], null], "palette edits belong to the session");
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
  const v = { ...firstChoice().values, output: MULTICOLOUR, colours: 5, fixed: ["#FF0000"], excluded: ["#00FF00"] };
  const json = JSON.parse(settingsJson(v, 128));
  assert.deepEqual([json.output, json.colours, json.fixed, json.excluded], [1, 5, ["#FF0000"], ["#00FF00"]]);
  assert.equal("merged" in json, false, "Merge into is gone (ux-notes §26)");
  // Fixed palette colours exist for Brightness and Remove background (the palette pipette stays
  // there, customer 2026-11-09); Pick colours takes its swatches; Single colour sends no palette
  // edits.
  assert.deepEqual(JSON.parse(settingsJson({ ...v, method: REMOVE_BACKGROUND }, 128)).fixed, ["#FF0000"]);
  assert.deepEqual(JSON.parse(settingsJson({ ...v, method: PICK_COLOURS }, 128)).fixed, []);
  const single = JSON.parse(settingsJson({ ...v, output: SINGLE_COLOUR }, 128));
  assert.deepEqual([single.output, single.excluded, single.fixed], [0, [], []]);
  assert.equal(JSON.parse(settingsJson({ ...v, method: BRIGHTNESS, colours: null }, 128)).colours, null);
});

test("ux-notes §26: a kept palette is traced as exactly its colours, with either method, never with Pick colours", () => {
  const v = { ...firstChoice().values, output: MULTICOLOUR, colours: null, fixed: ["#FF0000"], keptPalette: ["#FF0000", "#0000FF"] };
  for (const method of [BRIGHTNESS, REMOVE_BACKGROUND]) {
    const json = JSON.parse(settingsJson({ ...v, method }, 128));
    assert.deepEqual([json.fixed, json.colours], [["#FF0000", "#0000FF"], 2], `method ${method}`);
  }
  const pick = JSON.parse(settingsJson({ ...v, method: PICK_COLOURS }, 128));
  assert.deepEqual([pick.fixed, pick.colours], [[], null]);
  assert.deepEqual(JSON.parse(settingsJson({ ...v, output: SINGLE_COLOUR }, 128)).fixed, []);
  // A new method, Colours count or image ends it; another value keeps it.
  const choice = { ...firstChoice(), values: v };
  assert.equal(editChoice(choice, "method", REMOVE_BACKGROUND).values.keptPalette, null);
  assert.equal(editChoice(choice, "colours", 4).values.keptPalette, null);
  assert.deepEqual(editChoice(choice, "smooth", 40).values.keptPalette, ["#FF0000", "#0000FF"]);
  assert.equal(viewValues(v, false, undefined, undefined).keptPalette, null);
  assert.deepEqual(viewValues(v, true, undefined, undefined).keptPalette, ["#FF0000", "#0000FF"]);
});

test("ux-notes §26, §27: deleting a colour keeps exactly the others and un-excludes it, down to no colour", () => {
  const shown = ["#D0222A", "#234EB8", "#F2C20E"];
  assert.deepEqual(deleteFromPalette(shown, ["#234EB8", "#F2C20E"], "#234EB8"), { keptPalette: ["#D0222A", "#F2C20E"], excluded: ["#F2C20E"] });
  assert.equal(deleteFromPalette(shown, [], "#000000"), null, "a colour not shown");
  assert.deepEqual(deleteFromPalette(["#D0222A", "#234EB8"], [], "#234EB8"), { keptPalette: ["#D0222A"], excluded: [] });
  assert.deepEqual(deleteFromPalette(["#D0222A"], ["#D0222A"], "#D0222A"), { keptPalette: [], excluded: [] }, "the last colour goes too");
  // One kept colour is traced as it alone; none is not traced (`useTrace`), so it sends no count.
  const one = JSON.parse(settingsJson({ ...firstChoice().values, output: MULTICOLOUR, keptPalette: ["#D0222A"] }, 128));
  assert.deepEqual([one.fixed, one.colours], [["#D0222A"], 1]);
});

test("ux-notes §26: the swatch tooltip, name and keys", () => {
  assert.equal(paletteTooltip("#4B392F"), "#4B392F · Brown");
  assert.equal(PALETTE_HINT, "Shift+click: exclude");
  assert.equal(paletteSwatchLabel("#4B392F", false), "#4B392F · Brown");
  assert.equal(paletteSwatchLabel("#4B392F", true), "#4B392F · Brown, excluded");
  assert.deepEqual(paletteKey("ArrowRight", false, 1, 3), { move: 2 });
  assert.deepEqual(paletteKey("ArrowRight", false, 2, 3), { move: 2 });
  assert.deepEqual(paletteKey("ArrowLeft", false, 0, 3), { move: 0 });
  assert.deepEqual(paletteKey("Home", false, 2, 3), { move: 0 });
  assert.deepEqual(paletteKey("End", false, 0, 3), { move: 2 });
  assert.deepEqual(paletteKey("Enter", false, 1, 3), { toggle: 1 });
  assert.deepEqual(paletteKey(" ", false, 1, 3), { toggle: 1 });
  assert.deepEqual(paletteKey("Enter", true, 1, 3), { remove: 1 });
  assert.deepEqual(paletteKey("Delete", false, 1, 3), { remove: 1 });
  assert.deepEqual(paletteKey("Backspace", false, 1, 3), { remove: 1 });
  assert.equal(paletteKey("a", false, 1, 3), null);
});

test("UX notes §7: the swatch row's order, the colour names and the palette's words", () => {
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
  assert.equal(excludedNote(2), "2 colours excluded");
  assert.equal(excludedNote(1), "1 colour excluded");
  assert.equal(sameAsNote("#3366CC"), "Same as #3366CC");
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
  assert.equal(tracedTooltip(7, 412), "Traced: 7 paths, 412 nodes.");
});
