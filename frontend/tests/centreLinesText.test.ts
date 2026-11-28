import assert from "node:assert/strict";
import { describe, it } from "node:test";

import { readFileSync } from "node:fs";

import {
  CENTRE_INFO_ENTRIES,
  CENTRE_INFO_INTRO,
  centreField,
  centreFieldParse,
  centreFieldPosition,
  centreFieldStep,
  centreFieldText,
  centreFieldValueAt,
  centreLinesDimmedTooltip,
  centreEscapeCancels,
  centreLinesNotice,
  centrePreviewCounts,
  fieldFeedback,
  isContextMenuKey,
  parseOption,
} from "../src/lib/centreLinesText.ts";

const base = { kind: "applied", converted: 18, kept: 0, skipped: 0, unlinked: 0, nodes: 0 };

describe("centre lines text", () => {
  it("shows nothing after a clean conversion", () => {
    assert.equal(centreLinesNotice(base), null);
  });
  it("counts kept and skipped objects", () => {
  // 0159 S7 (customer 2026-11-27): result notices carry no counts of what came out.
    assert.equal(centreLinesNotice({ ...base, kept: 2, skipped: 3 })?.text, "Centre lines: some shapes were skipped.");
  });
  it("lists only the parts that are not zero", () => {
    // 0159 S7: shapes kept filled are seen on the canvas; no notice for them.
    assert.equal(centreLinesNotice({ ...base, converted: 1, kept: 1 }), null);
    assert.equal(centreLinesNotice({ ...base, converted: 0, kept: 2 }), null);
  });
  it("names the field of an out-of-range or non-numeric entry and is silent on a valid one", () => {
    const offset = { label: "Offset", min: -5, max: 5, suffix: "mm" };
    assert.deepEqual(fieldFeedback("9", offset), { value: 5, message: "Offset: From -5 to 5 mm" });
    assert.deepEqual(fieldFeedback("2", offset), { value: 2, message: null });
    assert.deepEqual(fieldFeedback("x", offset), { value: null, message: "Offset: Enter a number" });
    assert.deepEqual(fieldFeedback("99", { label: "Overlap", min: -50, max: 50, suffix: "%" }).message, "Overlap: From -50 to 50 %");
  });
  it("opens the context menu from Shift+F10 and the Menu key only", () => {
    assert.ok(isContextMenuKey({ key: "F10", shiftKey: true }));
    assert.ok(isContextMenuKey({ key: "ContextMenu", shiftKey: false }));
    assert.ok(!isContextMenuKey({ key: "F10", shiftKey: false }));
  });
  it("names detached clones", () => {
    assert.ok(centreLinesNotice({ ...base, unlinked: 1 })?.text.includes("Linked clones became ordinary paths."));
  });
  it("refuses with the first reason", () => {
    assert.deepEqual(centreLinesNotice({ ...base, kind: "text" }), { text: "Convert text to path first.", alert: true });
    assert.equal(centreLinesNotice({ ...base, kind: "too_large", nodes: 123456 })?.text, "Too large: 123,456 nodes. The limit is 100,000.");
  });
  it("says so when the worker fails, as the UX notes word it", () => {
    const text = centreLinesNotice({ ...base, kind: "failed" })?.text;
    assert.equal(text, "Centre lines failed. Nothing changed.");
    const notes = readFileSync(new URL("../../specs/0117-centre-lines/ux-notes.md", import.meta.url), "utf8");
    assert.ok(notes.includes(`"${text}"`));
  });
  it("writes the dimmed tooltip in lower case", () => {
    assert.equal(centreLinesDimmedTooltip("not_filled", 0), "Find centre lines: select filled shapes.");
  });
  it("clamps typed values", () => {
    assert.equal(parseOption("80", -50, 50), 50);
    assert.equal(parseOption("abc", -50, 50), null);
    assert.equal(parseOption("0,3", -5, 5), 0.3);
  });
});

describe("centre lines sliders (0117 UX notes §3.1)", () => {
  it("puts Thick parts on a log track from 0.5 to 20 mm, typed up to 100", () => {
    const thick = centreField("thickParts");
    assert.equal(centreFieldValueAt(thick, 0, "normal"), 0.5);
    assert.equal(centreFieldValueAt(thick, 1, "normal"), 20);
    assert.equal(centreFieldValueAt(thick, 0.5, "normal"), 3.2);
    assert.ok(Math.abs(centreFieldPosition(thick, 6) - Math.log(12) / Math.log(40)) < 1e-9);
    assert.equal(centreFieldPosition(thick, 100), 1);
    assert.equal(centreFieldParse(thick, "150 mm"), 100);
    assert.equal(centreFieldParse(thick, "abc"), null);
  });
  it("steps by 1 % or 0.1 mm, ten times with Shift and a tenth with Ctrl", () => {
    const overlap = centreField("overlap");
    assert.equal(centreFieldStep(overlap, 20, 1, "normal"), 21);
    assert.equal(centreFieldStep(overlap, 20, -1, "coarse"), 10);
    assert.equal(centreFieldStep(overlap, 50, 1, "normal"), 50);
    const offset = centreField("offset");
    assert.equal(centreFieldStep(offset, 0.3, 1, "normal"), 0.4);
    assert.equal(centreFieldStep(offset, 0.3, 1, "coarse"), 1.3);
    assert.equal(centreFieldStep(offset, 0.3, 1, "fine"), 0.31);
    const thick = centreField("thickParts");
    assert.equal(centreFieldStep(thick, 6, 1, "normal"), 6.1);
  });
  it("shows the values as the table says", () => {
    assert.equal(centreFieldText(centreField("overlap"), -20), "−20");
    assert.equal(centreFieldText(centreField("thickParts"), 6), "6.0");
    assert.equal(centreFieldText(centreField("thickParts"), 6.25), "6.25");
    assert.equal(centreFieldText(centreField("offset"), 0.3), "0.30");
  });
  it("counts a preview for the Apply tooltip", () => {
    assert.equal(centrePreviewCounts(24, 312, 2), "24 paths, 312 nodes, 2 kept filled");
    assert.equal(centrePreviewCounts(0, 0, 3), "No centre lines: every shape stays filled.");
  });
  it("words the info page as the UX notes' draft texts", () => {
    const notes = readFileSync(new URL("../../specs/0117-centre-lines/ux-notes.md", import.meta.url), "utf8");
    assert.ok(notes.includes(`"${CENTRE_INFO_INTRO}"`), "intro");
    for (const { term, text } of CENTRE_INFO_ENTRIES) {
      assert.ok(notes.includes(`| ${term} | "${text}"`), term);
      assert.ok(text.split(/[.!?](\s|$)/).filter((x) => x && x.trim()).length <= 2, `${term}: at most two sentences`);
    }
    assert.ok(!CENTRE_INFO_ENTRIES.some(({ text }) => /ratio/i.test(text)), "no ratio (criterion 15c)");
  });
  it("opens Join tips on and lists it after Optimise on the info page (criterion 9b)", async () => {
    const { CENTRE_LINES_DEFAULTS, CENTRE_JOIN_TIPS_TOOLTIP } = await import("../src/lib/centreLinesText.ts");
    assert.equal(CENTRE_LINES_DEFAULTS.joinTips, true);
    assert.ok(CENTRE_JOIN_TIPS_TOOLTIP.length > 0);
    const terms = CENTRE_INFO_ENTRIES.map(({ term }) => term);
    assert.deepEqual(terms, ["Overlap", "Thick parts", "Optimise", "Join tips", "Keep inner areas", "Offset"]);
  });
  it("words the Apply notice in paths, as the Apply tooltip does", () => {
    // 0159 S7: no counts after Apply.
    assert.equal(centreLinesNotice({ ...base, converted: 1, kept: 1 }, 70), null);
    assert.equal(centreLinesNotice({ ...base, converted: 1, kept: 1, skipped: 1 }, 1)?.text, "Centre lines: some shapes were skipped.");
  });
  it("lets Escape cancel the bar, also from a number entry", () => {
    const slider = { tagName: "DIV" };
    const entry = { tagName: "INPUT" };
    assert.equal(centreEscapeCancels({ key: "Escape", defaultPrevented: false, target: slider }), true);
    // A tooltip's dismiss layer already prevented the default: still the bar's Escape.
    assert.equal(centreEscapeCancels({ key: "Escape", defaultPrevented: true, target: slider }), true);
    assert.equal(centreEscapeCancels({ key: "Escape", defaultPrevented: true, target: entry }), true);
    assert.equal(centreEscapeCancels({ key: "Enter", defaultPrevented: false, target: slider }), false);
  });
});
