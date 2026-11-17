import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test } from "node:test";

import {
  BAR_WIDTH_RANGE,
  barPlace,
  sliderMinWidth,
  checkPrefix,
  machineLineParts,
  numbersParts,
  parseExportNotice,
  readoutParts,
} from "../src/lib/outputText.ts";
import { playKeyAction } from "../src/lib/playKeys.ts";
import type { Check } from "../src/lib/outputView.ts";

const source = (file: string) =>
  readFileSync(new URL(`../src/${file}`, import.meta.url), "utf8").replace(/\/\*[\s\S]*?\*\//g, "");

test("the play slider keys: arrows 1 %, page keys 10 %, Home and End the ends", () => {
  assert.deepEqual(playKeyAction("ArrowRight"), { kind: "step", percent: 1 });
  assert.deepEqual(playKeyAction("ArrowLeft"), { kind: "step", percent: -1 });
  assert.deepEqual(playKeyAction("PageUp"), { kind: "step", percent: 10 });
  assert.deepEqual(playKeyAction("PageDown"), { kind: "step", percent: -10 });
  assert.deepEqual(playKeyAction("Home"), { kind: "set", fraction: 0 });
  assert.deepEqual(playKeyAction("End"), { kind: "set", fraction: 1 });
  assert.equal(playKeyAction("a"), null);
  assert.equal(playKeyAction(" "), null);
});

test("the readout is time only and mutes the word between its two times", () => {
  assert.deepEqual(readoutParts("6 min 6 s of 9 min 51 s"), [
    { text: "6 min 6 s", muted: false },
    { text: " of ", muted: true },
    { text: "9 min 51 s", muted: false },
  ]);
});

test("a typed number is marked, the others are not", () => {
  assert.deepEqual(numbersParts("90 %, 6,000 mm/min, 2 ×, 0.25 mm", ["power", "passes"]), [
    { text: "90 %", typed: true },
    { text: "6,000 mm/min", typed: false },
    { text: "2 ×", typed: true },
    { text: "0.25 mm", typed: false },
  ]);
});

test("check lines are read as blocks or warnings", () => {
  const check = (severity: "block" | "warn"): Check => ({
    severity,
    text: "x",
    before: "",
    button: "",
    after: "x",
    count: 1,
    target: { kind: "none" },
  });
  assert.equal(checkPrefix(check("block")), "Blocks: ");
  assert.equal(checkPrefix(check("warn")), "Warning: ");
});

test("the export notice is milliseconds, a colon and the text with its second line", () => {
  assert.deepEqual(parseExportNotice("3000:Exported a.gcode."), { text: "Exported a.gcode.", ms: 3000 });
  assert.deepEqual(parseExportNotice("5000:Exported a.gcode.\nOutput turned off on 2 fixture jobs."), {
    text: "Exported a.gcode.\nOutput turned off on 2 fixture jobs.",
    ms: 5000,
  });
  assert.equal(parseExportNotice(""), null);
  assert.equal(parseExportNotice("x:y"), null);
});

test("the machine line is led by the profile's name", () => {
  assert.deepEqual(machineLineParts("Diode 10 W · GRBL · air assist"), {
    name: "Diode 10 W",
    rest: " · GRBL · air assist",
  });
  assert.deepEqual(machineLineParts("Machine profile damaged"), { name: "Machine profile damaged", rest: "" });
});

test("the bar is as wide as the frame between 360 and 640 px and sticks above the canvas bottom", () => {
  const canvas = { width: 1000, height: 700 };
  assert.equal(barPlace({ left: 100, bottom: 200, width: 100 }, canvas, 40).width, BAR_WIDTH_RANGE.min);
  assert.equal(barPlace({ left: 100, bottom: 200, width: 900 }, canvas, 40).width, BAR_WIDTH_RANGE.max);
  // Under the frame: 10 px below its bottom edge, left-aligned with it.
  assert.deepEqual(barPlace({ left: 100, bottom: 200, width: 400 }, canvas, 40), { left: 100, top: 210, width: 400 });
  // The frame's bottom leaves the view: 12 px above the canvas bottom.
  assert.equal(barPlace({ left: 100, bottom: 900, width: 400 }, canvas, 40).top, 700 - 12 - 40);
  // Never outside the canvas on the right.
  assert.equal(barPlace({ left: 900, bottom: 200, width: 400 }, canvas, 40).left, 1000 - 400 - 8);
});

test("the bar sticks 12 px above the canvas bottom whenever the frame's bottom is out of view", () => {
  const canvas = { width: 1000, height: 700 };
  // Panned up: the frame's bottom is above the view.
  assert.equal(barPlace({ left: 200, bottom: -30, width: 400 }, canvas, 40).top, 700 - 12 - 40);
  // Just inside the view but too low for the gap: still 12 px above the bottom.
  assert.equal(barPlace({ left: 200, bottom: 690, width: 400 }, canvas, 40).top, 700 - 12 - 40);
});

test("the bar never goes under the tool columns on the left", () => {
  const canvas = { width: 1000, height: 700 };
  // Tool columns end at --rail-right 116 px; the bar keeps 12 px from them.
  const inset = 116 + 12;
  assert.equal(barPlace({ left: 20, bottom: 200, width: 400 }, canvas, 40, inset).left, inset);
  assert.equal(barPlace({ left: 300, bottom: 200, width: 400 }, canvas, 40, inset).left, 300);
  // A narrow canvas shrinks the bar to the room right of the columns.
  const narrow = barPlace({ left: 20, bottom: 200, width: 640 }, { width: 600, height: 700 }, 40, inset);
  assert.equal(narrow.left, inset);
  assert.equal(narrow.width, 600 - inset - 8);
});

test("only Export is disabled in the output group, and only by aria-disabled so it stays a Tab stop", () => {
  assert.doesNotMatch(source("components/panel/OutputChecks.tsx"), /\bdisabled\b|aria-disabled/);
  const group = source("components/panel/OutputGroup.tsx").replace(/--field-disabled-/g, "");
  assert.doesNotMatch(group.replace(/aria-disabled/g, ""), /\bdisabled\b/);
  assert.match(group, /aria-disabled=\{exportOn \? undefined : true\}/);
  // The Start pick never greys an option (`0029` P29): its options are all pickable.
  assert.match(group, /pickable: true/);
});

test("the output group, the picks, the rows and the play bar use no colour of their own", () => {
  for (const file of [
    "components/panel/OutputGroup.tsx",
    "components/panel/OutputChecks.tsx",
    "components/panel/OutputPick.tsx",
    "components/panel/FixtureOutputPart.tsx",
    "components/panel/NumbersText.tsx",
    "components/panel/OperationRow.tsx",
    "components/PlayBar.tsx",
    "components/RingLabels.tsx",
  ]) {
    assert.doesNotMatch(source(file), /#[0-9a-fA-F]{3,8}\b|rgb\(|hsl\(|\b(?:bg|text|border)-(?:red|orange|blue|green|amber)-/, file);
  }
});

test("there is no Output tab: no strip entry, no OutputTab file, the group follows the Fixture jobs section", () => {
  assert.doesNotMatch(source("components/panel/PanelStrip.tsx"), /FileOutput/);
  assert.throws(() => source("components/panel/OutputTab.tsx"));
  const section = source("components/panel/OperationsSection.tsx");
  assert.ok(section.indexOf("{fixtureJobs}") < section.indexOf("{outputGroup}"), "the output group is after the Fixture jobs section");
});

test("the pick is a white field (draft 02 .pick) and marks the picked row with the value fill and a 3 px accent bar", () => {
  const pick = source("components/panel/OutputPick.tsx");
  // UX review U8: the button and its inline list stand on white, the tooltip is the app's.
  assert.match(pick, /border-\[var\(--field-border\)\] bg-white/);
  assert.match(pick, /<Tooltip side="left"/);
  assert.doesNotMatch(pick, /title=\{/);
  assert.match(pick, /bg-\[var\(--value-fill\)\]/);
  assert.match(pick, /w-\[3px\][^"]*bg-\[var\(--editor-accent\)\]/);
  assert.match(pick, /<Check\b/, "the current option carries the check");
});

test("the Export tooltip opens above and the group has the Start pick named Start with Crosshair", () => {
  const group = source("components/panel/OutputGroup.tsx");
  assert.equal(group.match(/<Tooltip side="top"/g)?.length, 1);
  assert.match(group, /label="Start"/);
  assert.match(group, /Icon=\{Crosshair\}/);
});

test("the play bar is one line: no wrap, no metres", () => {
  const bar = source("components/PlayBar.tsx");
  assert.doesNotMatch(bar, /flex-wrap|basis-full|BAR_WRAP/);
});

test("U1: below the bar's 360 px floor the slider gives way to the readout", () => {
  assert.equal(sliderMinWidth(BAR_WIDTH_RANGE.min), 120);
  assert.equal(sliderMinWidth(640), 120);
  // A 760 px window leaves the bar about 300 px: the slider shrinks to 48 px.
  assert.equal(sliderMinWidth(300), 48);
});
