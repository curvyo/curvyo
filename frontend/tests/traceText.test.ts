// `specs/0025-raster-trace`: the Trace tool's words and plain rules (criteria 1, 3, 5, 6, 9, 11,
// 13, 15, 16, 46) and criterion 35: every notice of this slice is the same string in the code and
// in the user page `docs/tracing-images.md`.

import { strict as assert } from "node:assert";
import { readFileSync } from "node:fs";
import path from "node:path";
import { test } from "node:test";

import {
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
  tracedNotice,
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
} from "../src/lib/traceText.ts";

const [threshold, smooth, specks] = TRACE_PARAMS_FALLBACK;
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
  assert.deepEqual(
    TRACE_PARAMS_FALLBACK.slice(0, 3).map((p) => [p.name, p.min, p.max, p.default]),
    [
      ["threshold", 0, 255, 128],
      ["smooth", 0, 100, 80],
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
    "Trace turns the selected image into paths. Dark parts become shapes, light parts stay empty. Press Enter to create the paths.",
  );
  assert.deepEqual(TRACE_INFO_ENTRIES, [
    "Preset: a named set of values for what you do with the paths.",
    "Logo / Line art: sharp corners, clean curves.",
    "Photo: many shapes, photos trace roughly.",
    "Silhouette for cutting/carving: closed, simple, few nodes; drops shapes and holes below 2 mm.",
    "Engraving: keeps fine lines, drops shapes below 0.1 mm.",
    "View: the image with the outline, the result alone, or the image alone. Hold \\ to peek at the image.",
    "Threshold: pixels darker than this brightness (0 to 100 %) become shapes.",
    "Smooth: higher gives fewer nodes and a looser fit, up to 0.2 mm.",
    "Ignore below: drops shapes and holes smaller than this size in mm.",
    "Denoise: smooths out noise in the picture before tracing.",
    "Upscale: doubles small images before tracing; Auto does it below 1 Mpx.",
    "Corners: higher turns more bends into sharp corners.",
    "Invert: traces the light parts instead of the dark ones.",
    "Ignore background: drops shapes that touch the image border.",
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
  assert.deepEqual(Object.keys(firstChoice().values).sort(), ["corners", "denoise", "ignoreBackground", "invert", "smooth", "specks", "threshold", "upscale"]);
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
    specks: 0.5,
    corners: 70,
    upscale: 1,
    denoise: 0,
    invert: false,
    ignoreBackground: false,
  });
  const edited = editChoice(pickPreset(first, TRACE_PRESETS_FALLBACK[2]), "smooth", 90);
  writeChoice({ ...edited, values: { ...edited.values, threshold: 77, invert: true } }, storage);
  const read = readChoice(TRACE_PARAMS_FALLBACK, TRACE_PRESETS_FALLBACK, storage);
  assert.equal(read.preset, "silhouette");
  assert.equal(read.edited, true);
  assert.equal(read.values.smooth, 90);
  assert.equal(read.values.specks, 2);
  assert.equal(read.values.threshold, null, "the Threshold is never remembered");
  assert.equal(read.values.invert, false, "the switches start off");
  storage.store.set("curvyo.trace-values", JSON.stringify({ smooth: 40, specks: 1.25 }));
  const slice1 = readChoice(TRACE_PARAMS_FALLBACK, TRACE_PRESETS_FALLBACK, storage);
  assert.deepEqual([slice1.preset, slice1.edited, slice1.values.smooth, slice1.values.specks], ["logo", true, 40, 1.25]);
  storage.store.set("curvyo.trace-values", "{broken");
  assert.deepEqual(readChoice(TRACE_PARAMS_FALLBACK, TRACE_PRESETS_FALLBACK, storage), firstChoice());
  assert.equal(readAdvancedOpen(storage), false);
  writeAdvancedOpen(true, storage);
  assert.equal(readAdvancedOpen(storage), true);
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
    specks: 0.1,
    corners: 30,
    upscale: 1,
    denoise: 0,
    invert: true,
    ignore_background: false,
  });
});

test("criteria 11, 15, 16, 71: the counts for the Advanced block, the states for the state row", () => {
  const wire = [0, 7, 412, 38, 5000, 0, 7];
  assert.deepEqual(statusText(readSummary(wire), 0.42, 100_000), {
    line: "",
    muted: "",
    counts: "7 paths, 412 nodes, 0.4 s",
    countsMuted: "38 specks ignored",
    canCreate: true,
  });
  assert.equal(statusText(readSummary([0, 1, 4, 1, 10, 0, 1]), 0.1, 100).countsMuted, "1 speck ignored");
  assert.deepEqual(statusText(readSummary([0, 0, 0, 0, 0, 0, 0]), 0.1, 100), {
    line: "0 paths at this Threshold: too low.",
    muted: "",
    counts: "0 paths, 0 nodes, 0.1 s",
    countsMuted: "",
    canCreate: false,
  });
  assert.equal(statusText(readSummary([0, 0, 0, 5, 90, 0, 0]), 0.1, 100).line, "0 paths at this Threshold: too high.");
  assert.deepEqual(statusText(readSummary([0, 1, 4, 0, 100, 1, 1]), 0.1, 100), {
    line: "1 path covering the whole image.",
    muted: "",
    counts: "1 path, 4 nodes, 0.1 s",
    countsMuted: "",
    canCreate: true,
  });
  assert.deepEqual(statusText(readSummary([1, 0, 0, 0, 0, 0, 0]), 0.1, 100), {
    line: TOO_DETAILED,
    muted: "",
    counts: "",
    countsMuted: "",
    canCreate: false,
  });
  assert.equal(statusText(readSummary([0, 1, 1204, 0, 10, 0, 1]), 2, 100).counts, "1 path, 1,204 nodes, 2.0 s");
});

test("criteria 13 and 46: notices and labels", () => {
  assert.equal(tracedNotice(1, 7, 412), "Traced: 7 paths, 412 nodes.");
  assert.equal(tracedNotice(3, 21, 1204), "Traced 3 images: 21 paths, 1,204 nodes.");
  assert.equal(tracedNotice(1, 1, 1), "Traced: 1 path, 1 node.");
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
    tracedNotice(1, 7, 412),
    tracedNotice(3, 21, 1204),
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

test("criterion 51: the View pill's segments and the backslash keys", () => {
  assert.deepEqual(
    TRACE_VIEWS.map((v) => [v.mode, v.label, v.tooltip]),
    [
      ["overlay", "Overlay", "Overlay"],
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
    canCreate: true,
  });
  assert.deepEqual(statusText(readSummary([0, 900, 412000, 0, 10, 0, 900]), 3, 100), {
    line: "412,000 nodes. Slow to edit.",
    muted: "",
    counts: "900 paths, 412,000 nodes, 3.0 s",
    countsMuted: "",
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
  const base = { threshold: 128, smooth: 80, specks: 0.5, corners: 70, upscale: 1, denoise: 0, invert: false, ignoreBackground: false };
  assert.equal(changedAdvancedText(base), null);
  assert.equal(changedAdvancedText({ ...base, invert: true }), "Changed: Invert");
  assert.equal(changedAdvancedText({ ...base, denoise: 2, corners: 30, ignoreBackground: true }), "Changed: Ignore background, Denoise, Corners");
});
