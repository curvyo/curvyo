// The Adjustments group's and the Before and after pane's decoders against the Rust fixtures of
// `curvyo-editor-wasm/tests/fixtures/adjustments_view/` and `before_after_view/`
// (`specs/0135-image-optimisation`). The Rust contract test
// `curvyo-editor-wasm/src/session/image_adjust/contract_tests.rs` builds the same states and
// compares them with these files, so a drift fails on both sides.
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync, readdirSync } from "node:fs";
import { decodeAdjustmentsText, decodeAdjustmentsView, PRESET_KEYS } from "../src/lib/adjustmentsView.ts";
import { decodeBeforeAfterText, decodeBeforeAfterView } from "../src/lib/beforeAfterView.ts";
import { breakEveryField } from "./contractWalk.ts";
import { barsPath, levelsMarks } from "../src/lib/levelsGraph.ts";
import { curvePath } from "../src/lib/curvePath.ts";

const ROOT = new URL("../../curvyo-editor-wasm/tests/fixtures/", import.meta.url);

function names(folder: string): string[] {
  return readdirSync(new URL(`${folder}/`, ROOT))
    .filter((file) => file.endsWith(".json"))
    .map((file) => file.slice(0, -".json".length))
    .sort();
}

function raw(folder: string, name: string): string {
  return readFileSync(new URL(`${folder}/${name}.json`, ROOT), "utf8");
}

function adjustments(name: string) {
  const view = decodeAdjustmentsText(raw("adjustments_view", name));
  assert.ok(view, `${name} decodes`);
  return view;
}

test("the Rust states decode, and nothing else is in the folders", () => {
  assert.deepEqual(names("adjustments_view"), [
    "clone_original",
    "clone_own",
    "damaged_step",
    "expert_steps",
    "memory_short",
    "no_image",
    "preset_photo",
    "slice2_steps",
    "slice3_ai_steps",
    "unknown_step",
  ]);
  assert.deepEqual(names("before_after_view"), ["no_image", "open_two_operations"]);
  for (const name of names("adjustments_view").filter((n) => n !== "no_image")) adjustments(name);
  for (const name of names("before_after_view")) assert.ok(decodeBeforeAfterText(raw("before_after_view", name)), name);
});

test("slice 2, criteria 5, 10: Crop, Resize, Channel mix and Curves rows with their bodies; Paste after a Copy", () => {
  const view = adjustments("slice2_steps");
  assert.deepEqual(view.steps.map((r) => r.kind), ["crop", "resize", "channel_mix", "curves"]);
  // Slice 3 adds the AI steps to row 2.
  assert.deepEqual(view.addGeometry.map((b) => b.kind), ["crop", "resize", "upscale", "ai_denoise", "remove_background"]);
  assert.equal(view.addTone.length, 7);
  assert.equal(view.pasteEnabled, true);
  assert.equal(view.steps[0].body, null);
  assert.deepEqual(view.steps[1].body, { kind: "resize", heightPx: 20, line: "Working image 40 × 20 px" });
  const mix = view.steps[2].body;
  assert.ok(mix && mix.kind === "channel_mix");
  assert.deepEqual(mix.presets.map((p) => p.pressed), [false, true, false, false]);
  assert.deepEqual(view.steps[3].body, { kind: "curve", points: [[0, 0], [40, 25], [100, 100]] });
  assert.equal(adjustments("expert_steps").pasteEnabled, false);
});

test("slice 3, criteria 47, 63, 65: one row of each AI step with its model state", () => {
  const view = adjustments("slice3_ai_steps");
  assert.deepEqual(view.steps.map((r) => r.kind), ["upscale", "ai_denoise", "remove_background"]);
  const [up, denoise, mask] = view.steps.map((r) => r.body);
  assert.ok(up && up.kind === "ai" && denoise && denoise.kind === "ai" && mask && mask.kind === "ai");
  assert.equal(up.model, "upscale");
  assert.equal(up.run, "Run");
  assert.equal(up.state, "Model not downloaded yet (2.4 MB).");
  assert.equal(up.sizeLine, "40 × 20 px to 80 × 40 px");
  assert.ok(denoise.consent?.startsWith("Downloads the AI denoise model (ffdnet-color, "));
  assert.equal(mask.modelReady, true);
  assert.equal(mask.stored, null);
  assert.equal(view.storedResults, null);
});

test("the Curves path starts and ends at the end points and stays inside the graph", () => {
  const path = curvePath([[0, 0], [40, 25], [100, 100]]);
  assert.ok(path.startsWith("M0.0 120.0"));
  assert.ok(path.endsWith("120.0 0.0"));
  assert.equal(curvePath([[0, 0]]), "");
});

test("no picture selected: the group is absent", () => {
  assert.equal(JSON.parse(raw("adjustments_view", "no_image")), null);
  assert.equal(decodeAdjustmentsText(raw("adjustments_view", "no_image")), null);
});

test("criteria 25, 26, 28: Photo presses its cell, shows Strength and the derived rows", () => {
  const view = adjustments("preset_photo");
  assert.deepEqual(view.presets.map((p) => p.key), [...PRESET_KEYS]);
  assert.equal(view.presets.find((p) => p.pressed)?.key, "photo");
  assert.ok(view.strength);
  assert.equal(view.strength.value, 50);
  assert.ok(view.derived);
  assert.match(view.derived.line, /^From preset Photo, Strength 50 %/);
  assert.ok(view.derived.rows.every((row) => row.index === null && row.fields.length === 0));
  assert.deepEqual(view.addTone, [], "no add toolbar while a preset is active");
});

test("UX 3.10: the picture's Dither strip decodes; Photo picked Jarvis, Expert says the Dither is unchanged", () => {
  const view = adjustments("preset_photo");
  assert.equal(view.dither.dither, "jarvis");
  assert.equal(view.dither.modes.length, 9);
  assert.equal(view.dither.modeField, null);
  assert.deepEqual(view.dither.screenFields, []);
  assert.equal(view.presets.find((p) => p.key === "expert")?.tooltip, "Expert · Dither unchanged");
  assert.equal(adjustments("expert_steps").dither.dither, "floyd_steinberg", "a new picture is Floyd-Steinberg");
  const broken = JSON.parse(raw("adjustments_view", "preset_photo"));
  broken.dither.dither = "stochastic";
  assert.equal(decodeAdjustmentsView(broken), null, "an unknown mode is refused");
  delete broken.dither;
  assert.equal(decodeAdjustmentsView(broken), null, "the strip is required");
});

test("criteria 1, 2: Expert lists the manual rows with their fields; an off row says so", () => {
  const view = adjustments("expert_steps");
  assert.equal(view.strength, null);
  assert.equal(view.derived, null);
  assert.deepEqual(view.steps.map((s) => [s.index, s.kind, s.enabled]), [
    [0, "levels", true],
    [1, "sharpen", false],
  ]);
  assert.ok(view.steps[0].fields.some((f) => f.key === "black_pct" && f.unit === "%"));
  assert.match(view.steps[1].line, /^Off/);
  // Slice 2: Channel mix and Curves join the tone row.
  assert.deepEqual(view.addTone.map((b) => b.kind), [
    "channel_mix",
    "denoise",
    "levels",
    "curves",
    "local_contrast",
    "contours",
    "sharpen",
  ]);
});

test("criterion 16: a Levels row carries 256 bars of its input, other rows none", () => {
  const view = adjustments("expert_steps");
  const bars = view.steps[0].histogram;
  assert.ok(bars && bars.length === 256 && Math.max(...bars) === 255);
  assert.equal(view.steps[1].histogram, null);
  const tooTall = JSON.parse(raw("adjustments_view", "expert_steps"));
  tooTall.steps[0].histogram[0] = 256;
  assert.equal(decodeAdjustmentsView(tooTall), null, "a bar is 0 to 255");
  tooTall.steps[0].histogram = [1, 2];
  assert.equal(decodeAdjustmentsView(tooTall), null, "256 bars");
});

test("criterion 56: a run that did not fit says so under the header", () => {
  assert.match(adjustments("memory_short").memoryLine ?? "", /need more memory than is free/);
  assert.equal(adjustments("expert_steps").memoryLine, null);
});

test("criterion 16: the Levels marks sit at Black, the Gamma mid grey and White", () => {
  assert.deepEqual(levelsMarks(10, 90, 1), [0.1, 0.5, 0.9]);
  const [, lighter] = levelsMarks(0, 100, 2);
  assert.equal(lighter, 0.25, "Gamma 2 lightens: mid grey comes from a darker input");
  assert.equal(barsPath([0, 255]), "M171.00 63V1.00");
});

test("criterion 9: a clone shows the original's adjustments until its first edit makes them its own", () => {
  assert.equal(adjustments("clone_original").cloneRow, "original");
  const own = adjustments("clone_own");
  assert.equal(own.cloneRow, "own");
  assert.equal(own.strength?.value, 80);
});

test("criteria 8, 23: a damaged step and an unknown kind are kept as rows that cannot run", () => {
  const damaged = adjustments("damaged_step");
  assert.equal(damaged.steps[0].unknown, true);
  assert.equal(damaged.steps[0].kind, "unknown");
  assert.deepEqual(damaged.steps[0].fields, []);
  assert.equal(damaged.steps[1].unknown, false);
  const unknown = adjustments("unknown_step");
  assert.equal(unknown.steps[1].kind, "unknown");
  assert.match(unknown.steps[1].line, /newer Curvyo/);
});

test("criteria 11, 12: the open pane names two operations, the first shown, and the job's dots", () => {
  const pane = decodeBeforeAfterText(raw("before_after_view", "open_two_operations"));
  assert.ok(pane);
  assert.equal(pane.open, true);
  assert.equal(pane.view, "split");
  assert.deepEqual(pane.operations.map((op) => [op.name, op.pressed]), [
    ["Image 1", true],
    ["Image 2", false],
  ]);
  assert.equal(pane.intervalMm, 0.1);
  assert.ok(pane.before && pane.after);
  assert.equal(pane.after.reduced, false);
  assert.deepEqual([pane.after.image.width, pane.after.image.height], [80, 40], "8 x 4 mm at 0.10 mm");
  const empty = decodeBeforeAfterText(raw("before_after_view", "no_image"));
  assert.equal(empty?.empty, "No picture selected.");
  assert.equal(empty?.after, null);
});

test("every missing or mistyped field is refused, at every depth", () => {
  for (const name of names("adjustments_view").filter((n) => n !== "no_image")) {
    for (const broken of breakEveryField(JSON.parse(raw("adjustments_view", name)))) {
      assert.equal(decodeAdjustmentsView(broken.value), null, `${name}: ${broken.how} ${broken.path} must be refused`);
    }
  }
  for (const name of names("before_after_view")) {
    for (const broken of breakEveryField(JSON.parse(raw("before_after_view", name)))) {
      assert.equal(decodeBeforeAfterView(broken.value), null, `${name}: ${broken.how} ${broken.path} must be refused`);
    }
  }
  assert.equal(decodeAdjustmentsText("not json"), null);
  assert.equal(decodeBeforeAfterText("{"), null);
});
