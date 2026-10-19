// Tester acceptance tests for specs/0046-document-shapes, stage 1, frontend criteria 9, 10, 14, 19, 23
// and 37: the Guide button's states and fixed words, the Template button in the File tab, the
// absence of hint text, popups and Apply buttons. Pure modules are called; the components are
// checked as source text because the repo has no DOM test runner.

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

import * as text from "../src/lib/guideText.ts";

const read = (path: string) => readFileSync(new URL(path, import.meta.url), "utf8");
const guideButton = read("../src/components/GuideButton.tsx");
const fileTab = read("../src/components/panel/FileTab.tsx");
const selectBar = read("../src/components/SelectToolbar.tsx");
const css = read("../src/index.css");

test("c9 the name is exactly Guide and Template, the tooltip is the name only", () => {
  assert.equal(text.GUIDE_NAME, "Guide");
  assert.equal(text.TEMPLATE_NAME, "Template");
  assert.equal(text.TEMPLATE_LOADED_NOTICE, "Template loaded.");
});

test("c9 c10 the session's three states map to disabled, off and pressed; unknown reads greyed", () => {
  assert.equal(text.readGuideState("disabled"), "disabled");
  assert.equal(text.readGuideState("off"), "off");
  assert.equal(text.readGuideState("pressed"), "pressed");
  for (const odd of ["", "Pressed", "on", "true", "undefined"]) {
    assert.equal(text.readGuideState(odd), "disabled", odd);
  }
});

test("c19 c23 the template outcome strings give a notice or the session's message", () => {
  assert.deepEqual(text.templateOutcome("loaded:3"), { notice: "Template loaded." });
  assert.deepEqual(text.templateOutcome("loaded:0"), { notice: "Template loaded." });
  assert.deepEqual(text.templateOutcome("error:This file is damaged and can't be read."), {
    error: "This file is damaged and can't be read.",
  });
  const fallback = text.templateOutcome("garbage");
  assert.ok("error" in fallback && fallback.error.length > 0);
  const empty = text.templateOutcome("error:");
  assert.ok("error" in empty && empty.error.length > 0, "an empty message never shows an empty dialog");
});

test("c9 the Guide button is 28 px, Lucide SquareDashed 16 px at 1.5 stroke, no label", () => {
  assert.match(guideButton, /import \{ SquareDashed \} from "lucide-react"/);
  assert.match(guideButton, /size-7/);
  assert.match(guideButton, /size=\{16\}/);
  assert.match(guideButton, /strokeWidth=\{1\.5\}/);
  assert.match(guideButton, /aria-label=\{GUIDE_NAME\}/);
  assert.match(guideButton, /aria-pressed=\{state === "pressed"\}/);
  // the button's only child is the icon: no visible text next to it
  const button = guideButton.slice(guideButton.indexOf("<button"), guideButton.indexOf("</button>"));
  assert.ok(!/>\s*[A-Za-z]{3,}\s*</.test(button.replace(/<SquareDashed[^>]*\/>/, "")), "no caption");
});

test("c9 c37 the greyed button keeps its Tab stop (aria-disabled, not the disabled attribute)", () => {
  assert.match(guideButton, /aria-disabled=\{!enabled\}/);
  assert.ok(!/\sdisabled=\{/.test(guideButton), "no disabled attribute");
  assert.match(guideButton, /if \(!enabled\)\s*\{\s*return;/, "a press on the greyed button does nothing");
});

test("c37 focus returns to the canvas after a mouse press, not after Space or Enter", () => {
  assert.match(guideButton, /event\.detail > 0/);
  assert.match(guideButton, /onReturnFocus\(\)/);
  assert.match(fileTab, /event\.detail > 0/);
});

test("c9 the Guide button sits right after the Flip group, in every selection state", () => {
  const flip = selectBar.indexOf("<FlipGroup");
  const guide = selectBar.indexOf("<GuideButton");
  assert.ok(flip > 0 && guide > flip, "after the Flip group");
  const between = selectBar.slice(flip, guide);
  assert.ok(!/<Divider|<button|<Tooltip/.test(between), "nothing between them");
  const before = selectBar.slice(selectBar.lastIndexOf("{", flip - 80), guide);
  assert.ok(!/(&&|\?)\s*\n?\s*<GuideButton/.test(before), "not conditional on the selection");
});

test("c14 no hint text, popup, Apply button or caption in the Guide or Template code", () => {
  const noComments = (source: string) => source.replace(/\/\*[\s\S]*?\*\//g, "").replace(/^\s*\/\/.*$/gm, "");
  for (const [name, source] of [
    ["GuideButton", guideButton],
    ["guideText", read("../src/lib/guideText.ts")],
  ] as const) {
    assert.ok(!/\b(hint|Apply|popover)\b/i.test(noComments(source)), `${name} has hint/popup words`);
  }
  const literals = [...read("../src/lib/guideText.ts").matchAll(/"([^"]+)"/g)].map((m) => m[1]);
  assert.ok(literals.every((l) => !/\b(tip|hint|press|click|drag)\b/i.test(l)), literals.join("|"));
  const template = fileTab.slice(fileTab.indexOf("function TemplateButton"), fileTab.indexOf("function CommandButton"));
  assert.ok(!/\b(Apply|Popover|Dialog)\b/.test(template));
  assert.match(template, /content=\{TEMPLATE_NAME\}/, "the tooltip is the name only");
});

test("c19 the Template button is an outline text button, 28 px high, like its neighbours", () => {
  const template = fileTab.slice(fileTab.indexOf("function TemplateButton"), fileTab.indexOf("function CommandButton"));
  assert.match(template, /\$\{BUTTON\} \$\{OUTLINE\}/);
  assert.match(fileTab, /const BUTTON =\s*\n?\s*"h-7 /);
  assert.match(template, /\{TEMPLATE_NAME\}\s*<\/button>/);
  assert.match(template, /project\.loadTemplate\(\)/);
  // it is in the row after Close, with the five other buttons around it
  const buttons = fileTab.slice(fileTab.indexOf("function Buttons"), fileTab.indexOf("function TemplateButton"));
  assert.ok(buttons.indexOf('button("close"') > 0 && buttons.indexOf("<TemplateButton") > buttons.indexOf('button("close"'));
});

test("c19 the Template button runs no unsaved-changes check and never replaces the project", () => {
  const project = read("../src/hooks/useProject.ts");
  const start = project.indexOf("const loadTemplate");
  const body = project.slice(start, project.indexOf("const resetProject"));
  assert.ok(!/check|confirm|openBytes|newProject|resetProject|setInfo/i.test(body), body);
});

test("c8 the --guide token is a green with at least 3:1 on canvas and pasteboard", () => {
  const match = /--guide:\s*#([0-9a-fA-F]{6})/.exec(css);
  assert.ok(match, "--guide defined");
  const channel = (hex: string, i: number) => {
    const v = parseInt(hex.slice(i * 2, i * 2 + 2), 16) / 255;
    return v <= 0.03928 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4;
  };
  const lum = (hex: string) => 0.2126 * channel(hex, 0) + 0.7152 * channel(hex, 1) + 0.0722 * channel(hex, 2);
  const ratio = (a: string, b: string) => (Math.max(lum(a), lum(b)) + 0.05) / (Math.min(lum(a), lum(b)) + 0.05);
  const guide = match[1];
  assert.ok(parseInt(guide.slice(2, 4), 16) > parseInt(guide.slice(0, 2), 16), "green dominates");
  assert.ok(ratio(guide, "FFFFFF") >= 3, "on a white canvas");
  assert.ok(ratio(guide, "B8B8BE") >= 3, "on the pasteboard");
});
