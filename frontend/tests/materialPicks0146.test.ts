// `specs/0146-material-test-picks` in the front end: the pick tokens' contrast (criterion 6), the
// cell names and statuses, the pick pick's availability and the canvas overlay's rules
// (criterion 8, UX 6.2, 6.3), the keyboard moves of the 2 x 2 control (criterion 1).
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

import { decodeMaterialsText } from "../src/lib/materialsView.ts";
import { chipGlyphs, chipWidth, decodePickMarks, frameOffset } from "../src/lib/pickOverlay.ts";
import { PICK_KINDS, PICK_LABEL, asPickKind, moveInGrid, noPickText } from "../src/lib/picks.ts";
import { pickChoices } from "../src/lib/operationSettingsRows.ts";
import type { OperationSettings } from "../src/lib/operationSettingsView.ts";
import { cellName, kindsAt, markingText, pickStatus } from "../src/lib/testGridResult.ts";

const css = readFileSync(new URL("../src/index.css", import.meta.url), "utf8");

function token(name: string): string {
  const match = css.match(new RegExp(`--${name}:\\s*(#[0-9A-Fa-f]{6})`));
  assert.ok(match, `--${name} is a hex colour`);
  return match[1] ?? "";
}

function luminance(hex: string): number {
  const channel = (i: number) => {
    const v = parseInt(hex.slice(1 + 2 * i, 3 + 2 * i), 16) / 255;
    return v <= 0.03928 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4;
  };
  return 0.2126 * channel(0) + 0.7152 * channel(1) + 0.0722 * channel(2);
}

function contrast(a: string, b: string): number {
  const [hi, lo] = [luminance(a), luminance(b)].sort((x, y) => y - x);
  return ((hi ?? 0) + 0.05) / ((lo ?? 0) + 0.05);
}

test("every pick colour reaches 3:1 on the grounds it is drawn on and none equals accent, cut or hatch", () => {
  const grounds = ["#DCDCE0", "#C5D7FA", "#A7C2F8", "#E8E8EB", "#FFFFFF"];
  const colours = [token("pick-best"), token("pick-fastest"), token("pick-deepest")];
  for (const colour of colours) {
    for (const ground of grounds) assert.ok(contrast(colour, ground) >= 3, `${colour} on ${ground}`);
  }
  for (const taken of [token("editor-accent"), token("cut"), token("engrave-hatch")]) {
    assert.ok(!colours.includes(taken.toUpperCase()), taken);
  }
  assert.match(css, /--pick-second:\s*var\(--pick-best\)/, "Second best is Best's colour, dashed");
});

test("the four picks have their words and a stored name round trips; best is no pick set key of the view", () => {
  assert.deepEqual(PICK_KINDS.map((k) => PICK_LABEL[k]), ["Best", "Second best", "Fastest", "Deepest"]);
  assert.equal(asPickKind("fastest"), "fastest");
  assert.equal(asPickKind("cleanest"), null);
  assert.equal(noPickText("fastest"), "No Fastest set for this laser");
  assert.equal(markingText("second_best"), "Marking Second best");
});

test("arrow keys move the focus in the 2 x 2 grid without wrapping, Home and End jump", () => {
  assert.equal(moveInGrid(0, "ArrowRight"), 1);
  assert.equal(moveInGrid(1, "ArrowRight"), null);
  assert.equal(moveInGrid(1, "ArrowDown"), 3);
  assert.equal(moveInGrid(3, "ArrowUp"), 1);
  assert.equal(moveInGrid(0, "ArrowLeft"), null);
  assert.equal(moveInGrid(2, "Home"), 0);
  assert.equal(moveInGrid(0, "End"), 3);
  assert.equal(moveInGrid(0, "Enter"), null, "activation is manual: Enter is the button's own click");
});

const axis = (values: string[], unit: string) => ({ title: "t", unit, values });

test("a cell's name lists its picks and a press says what was marked or removed", () => {
  const columns = axis(["20", "40", "60"], " %");
  const rows = axis(["600", "1950"], " mm/min");
  const picks = [
    { pick: "best" as const, cell: { column: 2, row: 1 }, line: null, setFrom: null },
    { pick: "fastest" as const, cell: { column: 2, row: 1 }, line: null, setFrom: null },
    { pick: "second_best" as const, cell: { column: 0, row: 0 }, line: null, setFrom: null },
  ];
  const kinds = kindsAt(picks, { column: 2, row: 1 });
  assert.deepEqual(kinds, ["best", "fastest"]);
  assert.equal(cellName(columns, rows, { column: 2, row: 1 }, kinds), "60, 1950, Best, Fastest");
  assert.equal(cellName(columns, rows, { column: 1, row: 0 }, []), "40, 600");
  assert.equal(pickStatus(columns, rows, { column: 2, row: 1 }, "fastest", false), "Fastest: 60 %, 1950 mm/min.");
  assert.equal(pickStatus(columns, rows, { column: 2, row: 1 }, "fastest", true), "Fastest removed.");
});

function settings(pick: string, available: string[], linked = true): { settings: OperationSettings } {
  return {
    settings: {
      material: linked ? { id: "m", name: "Birch" } : null,
      pick,
      picks: PICK_KINDS.map((p) => ({ pick: p, available: available.includes(p) })),
    } as unknown as OperationSettings,
  };
}

test("the pick pick: pressed kind, mixed, availability over several operations, nothing without a link", () => {
  const one = pickChoices([settings("fastest", ["best", "fastest"])]);
  assert.equal(one?.value, "fastest");
  assert.equal(one?.available("deepest"), false);
  const mixed = pickChoices([settings("best", ["best"]), settings("deepest", ["best", "deepest"])]);
  assert.equal(mixed?.value, null);
  assert.equal(mixed?.available("deepest"), true, "dimmed only when no selected operation has the set");
  assert.equal(mixed?.available("fastest"), false);
  assert.equal(pickChoices([settings("best", [], false)]), null);
});

test("the overlay: frame offset, chip width and the size rule of UX 6.2 and 6.3", () => {
  assert.equal(frameOffset(7.6), 1);
  assert.equal(frameOffset(20), 2);
  assert.equal(frameOffset(1), 1);
  assert.deepEqual([1, 2, 3, 4].map(chipWidth), [16, 30, 44, 58]);
  // 3 mm cells, pitch 18.9 px at 100 %: one glyph from 85 % zoom, four from 307 %.
  assert.equal(chipGlyphs(4, 18.9), 1);
  assert.equal(chipGlyphs(4, 58), 4);
  assert.equal(chipGlyphs(2, 30), 2);
  assert.equal(chipGlyphs(1, 15.9), 0, "no chip below 16 px, the frames stay");
});

test("the overlay JSON decodes, and anything else draws nothing", () => {
  const good = [{ x: 1, y: 2, width: 10, height: 10, pitch: 18, gap: 7, kinds: ["best", "deepest"] }];
  assert.deepEqual(decodePickMarks(JSON.stringify(good))[0]?.kinds, ["best", "deepest"]);
  assert.deepEqual(decodePickMarks("[]"), []);
  assert.deepEqual(decodePickMarks("not json"), []);
  assert.deepEqual(decodePickMarks(JSON.stringify([{ ...good[0], kinds: ["cleanest"] }])), []);
  assert.deepEqual(decodePickMarks(JSON.stringify([{ ...good[0], kinds: [] }])), []);
});

test("the Materials view decodes picks and folded pick lines, a bad pick name decodes to nothing", () => {
  const view = decodeMaterialsText(readFileSync(new URL("../../curvyo-editor-wasm/tests/fixtures/materials_view/test_with_picks.json", import.meta.url), "utf8"));
  assert.ok(view?.unfolded?.test, "the fixture decodes");
  const detail = view.unfolded.test;
  assert.deepEqual(detail.picks.map((p) => p.pick), ["best", "second_best", "fastest", "deepest"]);
  assert.equal(detail.picks[2]?.cell.column, 3);
  assert.equal(detail.picks[2]?.cell.column, detail.picks[3]?.cell.column, "one cell with two kinds");
  assert.match(detail.picks[0]?.line ?? "", /^Best: Cut set of /);
  assert.match(detail.picks[1]?.line ?? "", /^Second best: /);
  const others = view.unfolded.lasers[0]?.sets[0]?.otherPicks ?? [];
  assert.deepEqual(others.map((o) => o.pick), ["second_best", "fastest", "deepest"]);
  const bad = readFileSync(new URL("../../curvyo-editor-wasm/tests/fixtures/materials_view/test_with_picks.json", import.meta.url), "utf8").replace('"second_best"', '"cleanest"');
  assert.equal(decodeMaterialsText(bad), null);
});
