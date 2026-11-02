// `specs/0025-raster-trace`: the Trace tool's words and plain rules (criteria 1, 3, 5, 6, 9, 11,
// 13, 15, 16, 46) and criterion 35: every notice of this slice is the same string in the code and
// in the user page `docs/tracing-images.md`.

import { strict as assert } from "node:assert";
import { readFileSync } from "node:fs";
import path from "node:path";
import { test } from "node:test";

import {
  BRIGHTNESS,
  MULTICOLOUR,
  readsText,
  textLinesText,
  NO_STATUS,
  pickedCountText,
  TRACE_BAR_DESCRIPTION,
  TRACE_EXPAND_LABELS,
  changedAdvancedText,
  PREVIEW_OUT_OF_DATE,
  TOO_DETAILED,
  TRACE_EMPTY,
  TRACE_IMPORT,
  TRACE_NOT_IMAGE,
  TRACE_PARAMS_FALLBACK,
  TRACE_TOOLTIP,
  TRACING,
  clampValue,
  imagesLabel,
  parseTyped,
  CUSTOM,
  TRACE_PRESETS_FALLBACK,
  editChoice,
  firstChoice,
  pickPreset,
  presetLabel,
  readAdvancedOpen,
  readChoice,
  readSummary,
  settingsJson,
  specksHint,
  statusText,
  stepOnGrid,
  valueAt,
  lumaToPercent,
  percentToLuma,
  thresholdPercentParam,
  TRACE_INFO_ENTRIES,
  TRACE_INFO_INTRO,
  TRACE_INFO_TITLE,
  infoEntryParts,
  tracedTooltip,
  writeAdvancedOpen,
  writeChoice,
  REDUCED_COPY,
  RETRY,
  STOPPED,
  TRACE_VIEWS,
  FAILED,
  STOP,
  percentText,
  failureReason,
  failureText,
  isPeekKey,
  isCycleKey,
  previewIsReduced,
  MAX_SWATCHES,
  PICK_COLOURS,
  PROGRESS_AFTER_MS,
  PROGRESS_HOLD_MS,
  REMOVE_BACKGROUND,
  TRACE_FINISH,
  TRACE_METHODS,
  colourTolerance,
  pickSwatch,
  progressHoldLeft,
  zeroPathsLine,
} from "../src/lib/traceText.ts";

const [threshold, smooth, tolerance, specks] = TRACE_PARAMS_FALLBACK;
const keyOf = (key: string, mods: Partial<{ shiftKey: boolean; ctrlKey: boolean }> = {}) => ({
  key,
  shiftKey: false,
  ctrlKey: false,
  metaKey: false,
  ...mods,
});

test("criteria 5 and 6: steps, limits and clamping from the schema", () => {
  const percent = thresholdPercentParam(threshold);
  assert.equal(stepOnGrid(percent, 40, 1, "normal"), 41);
  assert.equal(stepOnGrid(percent, 40, -1, "coarse"), 30);
  assert.equal(stepOnGrid(percent, 40, 1, "fine"), 41, "no finer step than 1 %");
  assert.equal(stepOnGrid(smooth, 95, 1, "coarse"), 100);
  assert.equal(stepOnGrid(specks, 0.5, 1, "normal"), 0.6);
  assert.equal(stepOnGrid(specks, 0.5, 1, "coarse"), 1.5);
  assert.equal(stepOnGrid(specks, 0.5, -1, "fine"), 0.49);
  assert.equal(valueAt(percent, 0, "normal"), 0, "Home");
  assert.equal(valueAt(percent, 1, "normal"), 100, "End");
  assert.equal(valueAt(specks, 0.25, "normal"), 2.5);
  assert.equal(valueAt(smooth, 0.333, "coarse"), 30);
  assert.equal(parseTyped(percent, "300"), 100);
  assert.equal(parseTyped(percent, "40 %"), 40);
  assert.equal(parseTyped(percent, "40,5"), 41);
  assert.equal(parseTyped(specks, "-2"), 0);
  assert.equal(parseTyped(specks, "0,25 mm"), 0.25);
  assert.equal(parseTyped(smooth, "x"), null);
  assert.equal(clampValue(smooth, 33.4), 33);
  // `specs/0089` Frage 85 B: Tolerance in source pixels after Smooth.
  assert.equal(stepOnGrid(tolerance, 0.5, 1, "normal"), 0.55);
  assert.equal(parseTyped(tolerance, "0,8 px"), 0.8);
  assert.equal(parseTyped(tolerance, "9"), 3);
  assert.deepEqual(
    TRACE_PARAMS_FALLBACK.slice(0, 4).map((p) => [p.name, p.min, p.max, p.default]),
    [
      ["threshold", 0, 255, 128],
      ["smooth", 0, 100, 80],
      ["tolerance", 0.1, 3, 0.5],
      ["specks", 0, 10, 0.5],
    ],
  );
  assert.deepEqual([percent.unit, percent.min, percent.max, percent.default, percent.step, percent.big_step], ["%", 0, 100, 50, 1, 10]);
});

test("criterion 9 and Terms: the Threshold in percent, round half up both ways", () => {
  for (const [p, luma] of [[0, 0], [1, 3], [40, 102], [50, 128], [100, 255]]) {
    assert.equal(percentToLuma(p), luma, `${p} %`);
  }
  for (const [luma, p] of [[0, 0], [126, 49], [127, 50], [128, 50], [255, 100]]) {
    assert.equal(lumaToPercent(luma), p, `luma ${luma}`);
  }
  const lumas = new Set<number>();
  for (let p = 0; p <= 100; p += 1) {
    assert.equal(lumaToPercent(percentToLuma(p)), p, `${p} % round trip`);
    lumas.add(percentToLuma(p));
  }
  assert.equal(lumas.size, 101, "every percent is its own luma");
  // The one-grey picture's automatic luma 128 shows 50 %; the tracer receives the luma, never a
  // percent, and an automatic 127 is traced with 127 until the field is edited.
  assert.equal(lumaToPercent(128), 50);
  assert.equal(JSON.parse(settingsJson(firstChoice().values, 127)).threshold, 127);
});

test("criterion 70: the info page's exact text", () => {
  assert.equal(TRACE_INFO_TITLE, "About Trace");
  assert.equal(
    TRACE_INFO_INTRO,
    "Trace turns the selected image into paths. The method decides which parts become shapes. Press Enter or the Trace button to create the paths.",
  );
  assert.deepEqual(TRACE_INFO_ENTRIES, [
  "Preset: a named set of values for what you do with the paths.",
  "Logo / Line art: sharp corners, clean curves.",
  "Photo: many shapes, photos trace roughly.",
  "Silhouette for cutting/carving: closed, simple, few nodes; drops shapes and holes below 2 mm.",
  "Engraving: keeps fine lines, drops shapes below 0.1 mm.",
  "View: the image with the outline, the mask, the result alone, or the image alone. Hold \\ to peek at the image; Shift+\\ steps through the views.",
  "Mask: what the tracer turns into shapes, black on white (in Multicolour each colour as found), after Ignore below and Drop border shapes.",
  "Method: how pixels become shapes.",
  "Brightness: pixels darker than the Threshold.",
  "Remove background: everything unlike the background colour.",
  "Background: the colour Remove background leaves out; Auto takes it from the image border.",
  "Pick colours: pixels like the colours picked with the pipette; Shift+click adds a colour, up to 8. The picked colours are shown and edited in Advanced.",
  "Pipette: takes a colour from the image under a click; Esc ends it.",
  "Colour tolerance: how far a colour may be from the background or a picked colour and still count as it.",
  "Threshold: pixels darker than this brightness (0 to 100 %) become shapes.",
  "Smooth: calms the pixel steps of the outline before the fit; higher is calmer.",
  "Tolerance: how far in pixels the paths may stray from the calmed outline; higher gives fewer nodes.",
  "Ignore below: drops shapes and holes smaller than this size in mm.",
  "Analysis: names the image type and what it set for a new image: the method, its background and Colour tolerance, and the Threshold; never the preset or other values. Custom after you change one of them; Analyse sets them back.",
  "Status: the result in numbers (paths, nodes and time, specks ignored, colours) and what a colour action did.",
  "Output: Single colour traces the content as one black shape; Multicolour gives one filled shape per colour, neighbours sharing their edges.",
  "Colours: how many colours Multicolour finds, 2 to 16; Auto counts them in the image. With Pick colours the picked colours are the palette.",
  "Palette: the colours of a Multicolour trace, largest area first; picked colours carry a corner mark and come first.",
  "Isolate: shows one colour and dims the others; Enter still creates every colour.",
  "Recolour: gives a colour another fill without tracing again.",
  "Merge into: traces a colour as another one, as one shape.",
  "Exclude: makes a colour background; it is not traced.",
  "Remove: takes a picked colour out of the palette.",
  "Invert: traces what the method leaves out; transparent pixels never become shapes.",
  "Drop border shapes: drops shapes that touch the image border.",
  "Recognise text: reads the text in the picture first, sets each line as text in the nearest font and leaves it out of the paths.",
  "Denoise: smooths out noise in the picture before tracing.",
  "Upscale: doubles small images before tracing; Auto does it below 1 Mpx.",
  "Corners: higher turns more bends into sharp corners.",
  ]);
  assert.deepEqual(infoEntryParts(TRACE_INFO_ENTRIES[1]), { term: "Logo / Line art", text: "sharp corners, clean curves." });
});

test("criteria 41, 43 (removed): no Node limit and no Full-size preview in the schema or the values", () => {
  for (const p of TRACE_PARAMS_FALLBACK) {
    assert.ok(!/node|full/i.test(`${p.name} ${p.label}`), p.name);
  }
  assert.deepEqual(
    TRACE_PARAMS_FALLBACK.filter((p) => p.advanced).map((p) => p.name),
    ["invert", "ignore_background", "denoise", "upscale", "corners"],
  );
  assert.deepEqual(Object.keys(firstChoice().values).sort(), [
    "background",
    "backgroundAuto",
    "backgroundTolerance",
    "colours",
    "corners",
    "denoise",
    "excluded",
    "fixed",
    "ignoreBackground",
    "invert",
    "merged",
    "method",
    "output",
    "pickTolerance",
    "recogniseText",
    "smooth",
    "specks",
    "swatches",
    "threshold",
    "tolerance",
    "upscale",
  ]);
  assert.ok(!("node_limit" in JSON.parse(settingsJson(firstChoice().values, 128))));
});

function memoryStorage() {
  const store = new Map<string, string>();
  return { store, getItem: (k: string) => store.get(k) ?? null, setItem: (k: string, v: string) => void store.set(k, v) };
}

test("criteria 5 and 66: the choice is an app setting; Threshold starts automatic", () => {
  const storage = memoryStorage();
  const first = readChoice(TRACE_PARAMS_FALLBACK, TRACE_PRESETS_FALLBACK, storage);
  assert.deepEqual(first, firstChoice());
  assert.equal(first.preset, "logo");
  assert.deepEqual(first.values, {
    threshold: null,
    smooth: 80,
    tolerance: 0.5,
    specks: 0.5,
    corners: 70,
    upscale: 1,
    denoise: 0,
    invert: false,
    ignoreBackground: false,
    recogniseText: false,
    method: 0,
    backgroundTolerance: 40,
    pickTolerance: 19,
    backgroundAuto: true,
    background: null,
    swatches: [],
    output: 0,
    colours: null,
    fixed: [],
    excluded: [],
    merged: [],
  });
  const edited = editChoice(pickPreset(first, TRACE_PRESETS_FALLBACK[2]), "smooth", 90);
  writeChoice(
    { ...edited, values: { ...edited.values, threshold: 77, invert: true, method: 2, pickTolerance: 22, swatches: ["#FFF2A8"] } },
    storage,
  );
  const read = readChoice(TRACE_PARAMS_FALLBACK, TRACE_PRESETS_FALLBACK, storage);
  assert.equal(read.preset, "silhouette");
  assert.equal(read.edited, true);
  assert.equal(read.values.smooth, 90);
  assert.equal(read.values.specks, 2);
  assert.equal(read.values.tolerance, 1.5, "the tolerance is a preset value");
  assert.equal(read.values.threshold, null, "the Threshold is never remembered");
  assert.equal(read.values.invert, false, "the switches start off");
  // `specs/0089` criterion 20: the method and its tolerance are remembered, the picked colours not.
  assert.deepEqual([read.values.method, read.values.pickTolerance, read.values.backgroundTolerance], [2, 22, 40]);
  assert.deepEqual(read.values.swatches, []);
  storage.store.set("curvyo.trace-values", JSON.stringify({ smooth: 40, specks: 1.25 }));
  const slice1 = readChoice(TRACE_PARAMS_FALLBACK, TRACE_PRESETS_FALLBACK, storage);
  assert.deepEqual([slice1.preset, slice1.edited, slice1.values.smooth, slice1.values.specks], ["logo", true, 40, 1.25]);
  storage.store.set("curvyo.trace-values", "{broken");
  assert.deepEqual(readChoice(TRACE_PARAMS_FALLBACK, TRACE_PRESETS_FALLBACK, storage), firstChoice());
  assert.equal(readAdvancedOpen(storage), false);
  writeAdvancedOpen(true, storage);
  assert.equal(readAdvancedOpen(storage), true);
});

test("0089 criterion 13, Frage 85 B: remembered settings of older builds keep their meaning", () => {
  const storage = memoryStorage();
  const stored = (value: unknown) => {
    storage.store.set("curvyo.trace-values", JSON.stringify(value));
    return readChoice(TRACE_PARAMS_FALLBACK, TRACE_PRESETS_FALLBACK, storage);
  };
  // A 0025 choice (before slices D and C): no Tolerance, no method, a Node limit and a full-size
  // flag of slice 2; Smooth keeps its number (calmness), the Tolerance is the preset's.
  for (const preset of TRACE_PRESETS_FALLBACK) {
    const read = stored({ preset: preset.key, edited: false, values: { ...preset.values, tolerance: undefined, nodeLimit: 5000, fullSize: true } });
    assert.deepEqual([read.preset, read.edited], [preset.key, false], preset.key);
    assert.equal(read.values.tolerance, preset.values.tolerance, `${preset.key} starts at its own Tolerance`);
    assert.equal(read.values.smooth, preset.values.smooth);
    assert.equal(read.values.method, BRIGHTNESS, "no method reads as Brightness");
    assert.ok(!("nodeLimit" in read.values) && !("fullSize" in read.values));
  }
  // An edited 0025 Silhouette: its edited values stay, the missing Tolerance is Silhouette's.
  const custom = stored({ preset: "silhouette", edited: true, values: { smooth: 60, specks: 3, corners: 50, upscale: 0, denoise: 2 } });
  assert.deepEqual(
    [custom.preset, custom.edited, custom.values.smooth, custom.values.tolerance, custom.values.specks, custom.values.corners, custom.values.upscale, custom.values.denoise],
    ["silhouette", true, 60, 1.5, 3, 50, 0, 2],
  );
  // A stored "Ignore background" (never written by any build) does not switch Drop border shapes
  // on: the toggles start off; an unknown preset key reads as the first preset.
  const old = stored({ preset: "lineart", edited: false, values: { smooth: 70 }, ignoreBackground: true, ignore_background: true });
  assert.deepEqual([old.preset, old.values.ignoreBackground, old.values.invert, old.values.smooth, old.values.tolerance], ["logo", false, false, 70, 0.5]);
  // The settings the worker reads keep the old key: Drop border shapes is `ignore_background`.
  assert.equal(JSON.parse(settingsJson({ ...firstChoice().values, ignoreBackground: true }, 128)).ignore_background, true);
});

test("criteria 40, 59, 60: presets, Custom as stored state", () => {
  assert.deepEqual(
    TRACE_PRESETS_FALLBACK.map((p) => [p.label, p.detail]),
    [
      ["Logo / Line art", "Sharp corners, clean curves."],
      ["Photo", "Many shapes. Photos trace roughly."],
      ["Silhouette for cutting/carving", "Closed, simple, few nodes."],
      ["Engraving", "Keeps fine lines."],
    ],
  );
  const mine = editChoice(editChoice(firstChoice(), "threshold", 140), "invert", true);
  assert.equal(mine.edited, false, "Threshold and the switches never cause Custom");
  assert.equal(presetLabel(mine), "Logo / Line art");
  const photo = pickPreset(mine, TRACE_PRESETS_FALLBACK[1]);
  assert.equal(presetLabel(photo), "Photo");
  assert.deepEqual([photo.values.threshold, photo.values.invert], [140, true]);
  assert.deepEqual([photo.values.smooth, photo.values.specks, photo.values.corners], [50, 0.1, 30]);
  const custom = editChoice(photo, "corners", 31);
  assert.equal(presetLabel(custom), CUSTOM);
  assert.equal(custom.preset, "photo", "the menu marks the preset the values came from");
  assert.equal(presetLabel(editChoice(custom, "corners", 30)), CUSTOM, "editing back stays Custom");
  assert.equal(presetLabel(pickPreset(custom, TRACE_PRESETS_FALLBACK[1])), "Photo", "picking again clears it");
  assert.deepEqual(JSON.parse(settingsJson(photo.values, 140)), {
    threshold: 140,
    smooth: 50,
    tolerance: 0.5,
    specks: 0.1,
    corners: 30,
    upscale: 1,
    denoise: 0,
    invert: true,
    ignore_background: false,
    method: 0,
    colour_tolerance: 19,
    background: null,
    swatches: [],
    output: 0,
    colours: null,
    fixed: [],
    excluded: [],
    merged: [],
  });
});

// Lead decision 2026-10-31: the control is named Ignore below, so the status says so (0025 criterion 16).
test("0025 criterion 16: the too-detailed status names Ignore below", () => {
  assert.equal(TOO_DETAILED, "Too much detail: raise Ignore below.");
});

test("criteria 11, 15, 16, 71: the counts for the Advanced block, the states for the state row", () => {
  const wire = [0, 7, 412, 38, 5000, 0, 7];
  assert.deepEqual(statusText(readSummary(wire), 0.42, 100_000), {
    line: "",
    muted: "",
    counts: "7 paths, 412 nodes, 0.4 s",
    countsMuted: "38 specks ignored",
    message: "",
    canCreate: true,
  });
  assert.equal(statusText(readSummary([0, 1, 4, 1, 10, 0, 1]), 0.1, 100).countsMuted, "1 speck ignored");
  assert.deepEqual(statusText(readSummary([0, 0, 0, 0, 0, 0, 0]), 0.1, 100), {
    line: "0 paths at this Threshold: too low.",
    muted: "",
    // An empty result states "0 paths" once, in the notice row (`ux-notes.md` §22.2).
    counts: "",
    countsMuted: "",
    message: "",
    canCreate: false,
  });
  assert.equal(statusText(readSummary([0, 0, 0, 5, 90, 0, 0]), 0.1, 100).line, "0 paths at this Threshold: too high.");
  assert.deepEqual(statusText(readSummary([0, 1, 4, 0, 100, 1, 1]), 0.1, 100), {
    line: "1 path covering the whole image.",
    muted: "",
    counts: "1 path, 4 nodes, 0.1 s",
    countsMuted: "",
    message: "",
    canCreate: true,
  });
  assert.deepEqual(statusText(readSummary([1, 0, 0, 0, 0, 0, 0]), 0.1, 100), {
    line: TOO_DETAILED,
    muted: "",
    counts: "",
    countsMuted: "",
    message: "",
    canCreate: false,
  });
  assert.equal(statusText(readSummary([0, 1, 1204, 0, 10, 0, 1]), 2, 100).counts, "1 path, 1,204 nodes, 2.0 s");
});

test("criteria 13 and 46: notices and labels", () => {
  assert.equal(tracedTooltip(7, 412), "Traced: 7 paths, 412 nodes.");
  assert.equal(tracedTooltip(1, 1), "Traced: 1 path, 1 node.");
  assert.equal(imagesLabel(0, 3), "Tracing 1 of 3 images");
  assert.equal(imagesLabel(0, 1), "");
  assert.equal(specksHint(0.5, 3.78), "~ 2 px");
  assert.equal(specksHint(0.1, 3.78), "< 1 px");
  assert.equal(TRACE_TOOLTIP, "Trace tool (I)");
  assert.equal(TRACE_BAR_DESCRIPTION, "Enter creates the paths");
  assert.deepEqual(TRACE_EXPAND_LABELS, { closed: "More trace options", open: "Fewer trace options" });
});

test("criterion 35: the notices are the same strings in the user page", () => {
  const page = readFileSync(path.resolve(import.meta.dirname, "..", "..", "docs", "tracing-images.md"), "utf8").replace(/\s+/g, " ");
  const quoted = [
    TRACE_EMPTY,
    TRACE_IMPORT,
    TRACE_NOT_IMAGE,
    PREVIEW_OUT_OF_DATE,
    TOO_DETAILED,
    STOP,
    statusText(readSummary([0, 7, 412, 38, 5000, 0, 7]), 0.4, 100_000).counts,
    statusText(readSummary([0, 7, 412, 38, 5000, 0, 7]), 0.4, 100_000).countsMuted,
    statusText(readSummary([0, 0, 0, 0, 0, 0, 0]), 0.1, 100).line,
    statusText(readSummary([0, 1, 4, 0, 100, 1, 1]), 0.1, 100).line,
    imagesLabel(0, 3),
    ...TRACE_PRESETS_FALLBACK.flatMap((p) => [p.label, p.detail]),
    STOPPED,
    FAILED,
    REDUCED_COPY,
    RETRY,
    statusText(readSummary([0, 900, 412000, 0, 10, 0, 900]), 3, 100).line,
    TRACE_INFO_INTRO,
    ...TRACE_INFO_ENTRIES,
    failureText("out of memory at 16 Mpx"),
    ...TRACE_VIEWS.map((v) => v.label),
  ];
  for (const text of quoted) {
    assert.ok(page.includes(text), `the page quotes "${text}"`);
  }
});

test("criterion 51 and 0089 criterion 21: the View pill's segments and the backslash keys", () => {
  assert.deepEqual(
    TRACE_VIEWS.map((v) => [v.mode, v.label, v.tooltip]),
    [
      ["overlay", "Overlay", "Overlay"],
      ["mask", "Mask", "Mask"],
      ["trace", "Trace", "Trace"],
      ["image", "Image", "Image (\\)"],
    ],
  );
  const key = (k: string, mods: Partial<KeyboardEvent> = {}) => ({ key: k, code: "", shiftKey: false, ctrlKey: false, metaKey: false, altKey: false, repeat: false, ...mods });
  assert.ok(isPeekKey(key("\\")));
  assert.ok(isPeekKey(key("\\", { ctrlKey: true, altKey: true })), "AltGr on German keyboards");
  assert.ok(!isPeekKey(key("\\", { repeat: true })));
  assert.ok(!isPeekKey(key("\\", { shiftKey: true })));
  assert.ok(isCycleKey(key("|", { shiftKey: true })));
  assert.ok(isCycleKey(key("\\", { shiftKey: true })));
  assert.ok(!isCycleKey(key("|", { shiftKey: true, repeat: true })));
  assert.ok(!isCycleKey(key("i")));
});

test("criteria 52 to 55: large images, stopped, failure, slow to edit", () => {
  assert.ok(previewIsReduced(2001, 2000));
  assert.ok(!previewIsReduced(2000, 2000));
  const wire = [0, 7, 412, 38, 5000, 0, 7];
  assert.deepEqual(statusText(readSummary(wire), 0.4, 100_000, true), {
    line: "",
    muted: REDUCED_COPY,
    counts: "7 paths, 412 nodes, 0.4 s",
    countsMuted: "38 specks ignored",
    message: "",
    canCreate: true,
  });
  assert.deepEqual(statusText(readSummary([0, 900, 412000, 0, 10, 0, 900]), 3, 100), {
    line: "412,000 nodes. Slow to edit.",
    muted: "",
    counts: "900 paths, 412,000 nodes, 3.0 s",
    countsMuted: "",
    message: "",
    canCreate: true,
  });
  assert.equal(statusText(readSummary([0, 9, 100000, 0, 10, 0, 9]), 3, 100).line, "", "100,000 nodes is no state");
  assert.equal(failureText("internal error"), "Trace failed: internal error. Your drawing is unchanged.");
  assert.equal(failureReason("RangeError: WebAssembly.Memory.grow(): Maximum memory size exceeded", 36_000_000), "out of memory at 36 Mpx");
  assert.equal(failureReason("RuntimeError: unreachable", 4_000_000), "internal error");
  assert.equal(STOPPED, "Stopped");
  assert.equal(FAILED, "Failed");
});

test("criterion 10: the progress pill's words", () => {
  assert.equal(TRACING, "Tracing");
  assert.equal(STOP, "Stop (Esc)");
  assert.equal(percentText(42), "42 %");
  assert.equal(percentText(0), "0 %");
  assert.equal(percentText(null), "...");
});

test("UX notes 19: the closed expander names the Advanced values that are not defaults", () => {
  const base = { ...firstChoice().values, threshold: 128 };
  assert.equal(changedAdvancedText(base), null);
  assert.equal(changedAdvancedText({ ...base, invert: true }), "Changed: Invert");
  assert.equal(changedAdvancedText({ ...base, denoise: 2, corners: 30, ignoreBackground: true }), "Changed: Drop border shapes, Denoise, Corners");
});

test("specs/0089 criterion 14: the methods, their fields in the settings JSON", () => {
  assert.deepEqual(
    TRACE_METHODS.map((m) => [m.label, m.detail]),
    [
      ["Brightness", "Dark and light pixels by a cut-off."],
      ["Remove background", "Everything unlike the background."],
      ["Pick colours", "Pixels like the colours you picked."],
    ],
  );
  const base = firstChoice().values;
  const removing = { ...base, method: REMOVE_BACKGROUND, background: "#3366CC", backgroundTolerance: 30 };
  assert.equal(colourTolerance(removing), 30);
  const json = JSON.parse(settingsJson(removing, 120));
  assert.deepEqual([json.method, json.colour_tolerance, json.background], [1, 30, "#3366CC"]);
  // A transparent background crosses as null.
  assert.equal(JSON.parse(settingsJson({ ...removing, background: "" }, 120)).background, null);
  const picking = { ...base, method: PICK_COLOURS, swatches: ["#FFF2A8", "#D9D9D9"] };
  assert.deepEqual(JSON.parse(settingsJson(picking, 120)).swatches, ["#FFF2A8", "#D9D9D9"]);
  // Pick colours takes the image's suggested background, the same for preview and full size.
  assert.equal(JSON.parse(settingsJson(picking, 120, "#FFFFFF")).background, "#FFFFFF");
  assert.equal(JSON.parse(settingsJson(picking, 120, "")).background, null);
  assert.equal(JSON.parse(settingsJson({ ...picking, method: 1, background: "#112233" }, 120, "#FFFFFF")).background, "#112233");
  assert.equal(colourTolerance(picking), 19);
  const full = ["#000001", "#000002", "#000003", "#000004", "#000005", "#000006", "#000007", "#000008"];
  assert.deepEqual(pickSwatch(full, "#000009", true), full);
  // The bar's pipette label states what is picked (`ux-notes.md` §22.1).
  assert.deepEqual([0, 1, 3].map(pickedCountText), ["None picked", "1 picked", "3 picked"]);
});

test("specs/0089 criterion 17, Frage 89 A: a click replaces the picked colours, Shift adds up to 8", () => {
  assert.deepEqual(pickSwatch(["#111111"], "#222222", false), ["#222222"]);
  assert.deepEqual(pickSwatch(["#111111"], "#222222", true), ["#111111", "#222222"]);
  assert.deepEqual(pickSwatch(["#111111"], "#111111", true), ["#111111"], "no twin");
  const full = Array.from({ length: MAX_SWATCHES }, (_, i) => `#00000${i}`);
  assert.deepEqual(pickSwatch(full, "#FFFFFF", true), full, "a ninth is not added");
  assert.equal(zeroPathsLine({ method: PICK_COLOURS, swatches: [] }), "0 paths: no colour picked.");
  assert.equal(zeroPathsLine({ method: PICK_COLOURS, swatches: ["#FFFFFF"] }), "0 paths at this Colour tolerance: too low.");
  assert.equal(zeroPathsLine({ method: REMOVE_BACKGROUND, swatches: [] }), "0 paths at this Colour tolerance: too high.");
  assert.equal(zeroPathsLine({ method: 0, swatches: [] }), null);
  // Nothing picked is fixed in Advanced: the status line says it, the notice row does not (`ux-notes.md` §22.2).
  const nothing = statusText(readSummary([0, 0, 0, 0, 0, 0, 0]), 0.1, 100, false, "0 paths: no colour picked.");
  assert.deepEqual([nothing.message, nothing.line, nothing.canCreate], ["0 paths: no colour picked.", "", false]);
});

test("customer 2026-10-30: the finish button reads Trace; its progress shows after 400 ms and holds 300 ms", () => {
  assert.equal(TRACE_FINISH, "Trace");
  assert.equal(PROGRESS_AFTER_MS, 400);
  assert.equal(PROGRESS_HOLD_MS, 300);
  assert.equal(progressHoldLeft(null, 1000), 0, "never shown: nothing to hold");
  assert.equal(progressHoldLeft(1000, 1050), 250, "shown 50 ms ago: 250 ms more");
  assert.equal(progressHoldLeft(1000, 1300), 0);
  assert.equal(progressHoldLeft(1000, 2000), 0);
});

test("specs/0091 criterion 17 (Frage 116 B a): Recognise text is off by default, remembered, and on is a change", () => {
  const storage = memoryStorage();
  assert.equal(firstChoice().values.recogniseText, false);
  const on = editChoice(firstChoice(), "recogniseText", true);
  assert.equal(on.edited, false, "a switch, not a preset value");
  writeChoice(on, storage);
  assert.equal(readChoice(TRACE_PARAMS_FALLBACK, TRACE_PRESETS_FALLBACK, storage).values.recogniseText, true);
  assert.equal(changedAdvancedText(on.values), "Changed: Recognise text");
  assert.equal(changedAdvancedText(firstChoice().values), null);
  assert.equal(readsText({ ...on.values, output: MULTICOLOUR }), false);
  // The "on" an earlier build kept as its default does not turn it on.
  storage.setItem("curvyo.trace-values", JSON.stringify({ recogniseText: true }));
  assert.equal(readChoice(TRACE_PARAMS_FALLBACK, TRACE_PRESETS_FALLBACK, storage).values.recogniseText, false);
});

test("specs/0091 UX notes §4: the text-line count", () => {
  assert.equal(textLinesText(0, 0), "");
  assert.equal(textLinesText(1, 1), "1 text line set");
  assert.equal(textLinesText(3, 5), "3 of 5 text lines set");
  assert.equal(textLinesText(0, 2), "0 of 2 text lines set");
});
