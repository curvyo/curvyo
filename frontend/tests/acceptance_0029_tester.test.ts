// Tester acceptance cases for `specs/0029-laser-job-preview-and-output`, written from the
// specification: the UI words and symbols of criteria 1, 4, 5, 8, 37 and 38, the play keys of
// criterion 37 and the notice of criterion 27, read from the shipped modules and sources.
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test } from "node:test";

import { EXPORT_NAME, parseExportNotice } from "../src/lib/outputText.ts";
import { TOOLPATH_VIEW_NAME, toolpathToggle } from "../src/lib/pages.ts";
import { playKeyAction } from "../src/lib/playKeys.ts";

const source = (file: string) =>
  readFileSync(new URL(`../src/${file}`, import.meta.url), "utf8").replace(/\/\*[\s\S]*?\*\//g, "").replace(/\/\/.*$/gm, "");

// Amended 2026-11-17 (customer feedback on PR #371): no Output tab; the output group sits in the
// Operations tab after the Fixture jobs section.
test("criterion 1: the output group's Export button has Download and is named 'Export job file'; no strip entry", () => {
  const strip = source("components/panel/PanelStrip.tsx");
  assert.doesNotMatch(strip, /FileOutput/);
  const tab = source("components/panel/OutputGroup.tsx");
  assert.match(tab, /<Download\b/);
  assert.equal(EXPORT_NAME, "Export job file");
});

test("criterion 1: the output group holds no number field and no second slider", () => {
  for (const f of ["components/panel/OutputGroup.tsx", "components/panel/FixtureOutputPart.tsx", "components/panel/OutputChecks.tsx", "components/panel/OutputPick.tsx"]) {
    const s = source(f);
    assert.doesNotMatch(s, /type=["']number["']/, `${f} has a number field`);
    assert.doesNotMatch(s, /type=["']range["']/, `${f} has a slider`);
  }
  const bar = source("components/PlayBar.tsx");
  assert.ok((bar.match(/type=["']range["']|role=["']slider["']/g) ?? []).length <= 1, "one slider in the play bar");
});

test("criteria 4 and 5: blocks show OctagonX and warns TriangleAlert", () => {
  const checks = source("components/panel/OutputChecks.tsx");
  assert.match(checks, /<OctagonX\b/);
  assert.match(checks, /<TriangleAlert\b/);
});

test("criterion 4: Export is disabled, never hidden, while blocked", () => {
  const tab = source("components/panel/OutputGroup.tsx");
  assert.match(tab, /aria-disabled=\{exportOn \? undefined : true\}/);
});

test("criterion 8: the Toolpath view toggle uses Route next to the Operations view's ListTree", () => {
  const frames = source("components/PageFrames.tsx");
  assert.match(frames, /Glyph=\{Route\}/);
  assert.match(frames, /Glyph=\{ListTree\}/);
  assert.equal(TOOLPATH_VIEW_NAME, "Toolpath view");
  const base = { id: "p", name: "Laser 1" } as Parameters<typeof toolpathToggle>[0];
  assert.equal(toolpathToggle({ ...base, toolpathView: null }), null, "no toggle on a page of another kind");
  assert.deepEqual(toolpathToggle({ ...base, toolpathView: false }), { pressed: false });
});

test("criterion 37: one Play/Pause symbol button named Play and Pause; arrows step 1 %", () => {
  const bar = source("components/PlayBar.tsx");
  assert.match(bar, /playing \? Pause : Play/);
  assert.match(bar, /playing \? "Pause" : "Play"/);
  assert.deepEqual(playKeyAction("ArrowRight"), { kind: "step", percent: 1 });
  assert.deepEqual(playKeyAction("ArrowLeft"), { kind: "step", percent: -1 });
});

test("criterion 38: Start is one pick named 'Start' with the Crosshair symbol and no switch", () => {
  const tab = source("components/panel/OutputGroup.tsx");
  assert.match(tab, /label="Start"/);
  assert.match(tab, /Icon=\{Crosshair\}/);
  assert.doesNotMatch(tab, /Home first|PanelSwitch/);
});

test("criterion 27: the export notice keeps its second line and its time", () => {
  const one = parseExportNotice("3000:Exported Untitled - Laser 1.gcode.");
  assert.deepEqual(one, { text: "Exported Untitled - Laser 1.gcode.", ms: 3000 });
  const two = parseExportNotice("5000:Exported a.gcode.\nOutput turned off on 2 fixture jobs.");
  assert.equal(two?.ms, 5000);
  assert.ok(two?.text.includes("\n"), "one notice with two lines");
  assert.equal(parseExportNotice(""), null, "an unknown token shows nothing");
});
