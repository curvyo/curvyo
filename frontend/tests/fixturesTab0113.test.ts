// The Fixtures tab's wiring and UX rules read from the sources, since the frontend tests run
// without a DOM (`specs/0113-fixture-sets-and-pins` `ux-notes.md`): hidden not disabled, no popups,
// the lucide symbols of section 2, the tab in the strip and the removed block.
import { test } from "node:test";
import assert from "node:assert/strict";
import { existsSync, readFileSync } from "node:fs";

const SRC = new URL("../src/", import.meta.url);
const source = (path: string) => readFileSync(new URL(path, SRC), "utf8");
/** The code without its comments, for the rules that name words a comment may use. */
const code = (path: string) => source(path).replace(/\/\*[\s\S]*?\*\//g, "").replace(/^\s*\/\/.*$/gm, "");

const COMPONENTS = [
  "components/workbench/FixturesTab.tsx",
  "components/workbench/FixtureSetsSection.tsx",
  "components/workbench/FixtureSetRow.tsx",
  "components/workbench/FixtureRowItem.tsx",
  "components/workbench/FixtureLockButton.tsx",
  "components/workbench/FixtureAddBar.tsx",
  "components/workbench/FixtureBody.tsx",
  "components/workbench/PinScrewBody.tsx",
  "components/workbench/JobFixturesSection.tsx",
  "components/workbench/ApplyFixturesList.tsx",
  "components/FixtureLockBar.tsx",
];

test("nothing in the tab opens a popup or a dialog, and nothing is disabled: controls are hidden", () => {
  for (const file of COMPONENTS) {
    const text = code(file);
    assert.doesNotMatch(text, /createPortal|role="dialog"|popover|window\.prompt|window\.confirm/i, file);
    // The one exception (customer, 2026-11-16): a pin's or screw's Spacing is greyed out at 1.
    // UX review 2026-11-16: the Lock of a fixture fixed by its set is `aria-disabled` (muted, not pressable).
    const rest = file.endsWith("PinScrewBody.tsx")
      ? text.replace("disabled={spec.disabled === true}", "")
      : file.endsWith("FixtureLockButton.tsx")
        ? text.replace("aria-disabled={fixed || undefined}", "")
        : text;
    assert.doesNotMatch(rest, /\bdisabled\b|aria-disabled/, `${file}: hidden, not disabled`);
  }
});

test("the symbols of UX notes section 2", () => {
  assert.match(source("components/workbench/FixtureSetRow.tsx"), /ChevronRight[\s\S]*ChevronDown|ChevronDown[\s\S]*ChevronRight/);
  assert.match(source("components/workbench/FixtureSetRow.tsx"), /ChevronUp/);
  assert.match(source("components/workbench/FixtureSetsSection.tsx"), /\bPlus\b/);
  assert.match(source("components/workbench/FixtureLockButton.tsx"), /\bLock\b[\s\S]*\bLockOpen\b|\bLockOpen\b[\s\S]*\bLock\b/);
  assert.match(source("components/workbench/PinScrewBody.tsx"), /Crosshair/);
  assert.match(source("components/workbench/PinScrewBody.tsx"), /\bDiameter\b/);
  assert.match(source("components/workbench/ApplyFixturesList.tsx"), /CopyPlus/);
  assert.match(source("components/workbench/FixtureGlyphs.tsx"), /kind === "pin"/);
});

test("the tab sits in the strip with the Holder glyph, and the Document tab has no Fixtures block", () => {
  const strip = source("components/panel/PanelStrip.tsx");
  assert.match(strip, /fixtures: HolderGlyph/);
  assert.match(strip, /<FixtureGlyph kind="holder"/);
  assert.match(source("components/panel/PropertiesPanel.tsx"), /content === "fixtures"[\s\S]{0,80}<FixturesTab/);
  assert.match(source("components/workbench/ManifestTab.tsx"), /case "fixture_list":[\s\S]{0,160}return null/);
  assert.equal(existsSync(new URL("components/workbench/FixturesBlock.tsx", SRC)), false, "the old block is gone");
});

test("the add buttons are glyph-only with the kind word as name and tooltip", () => {
  const bar = source("components/workbench/FixtureAddBar.tsx");
  assert.match(bar, /aria-label=\{word\}/);
  assert.match(bar, /<Tooltip[^>]*content=\{word\}/);
  assert.match(bar, /aria-pressed=/);
  assert.match(bar, /role="toolbar"/);
});

test("the removed binding is called nowhere", () => {
  for (const file of ["hooks/useFixtures.ts", "hooks/useFixtureSets.ts", ...COMPONENTS]) {
    assert.doesNotMatch(source(file), /\badd_fixture\(/, file);
  }
  assert.match(source("hooks/useFixtureSets.ts"), /add_set_fixture/);
});

test("the fixed jobs of a pin or screw have no trash and no rename, and the Machine block counts sets", () => {
  const row = source("components/panel/FixtureJobRow.tsx");
  assert.match(row, /renaming \|\| fixed \? null : <TrashButton/);
  // ADR 0098: the count stays under the laser's Machine group in the Document tab.
  assert.match(source("components/workbench/ManifestTab.tsx"), /Fixture sets in profile: \{view\.machine\.fixture_sets_in_profile\}/);
});

// Superseded by `0113` rework: a set fixture gets the Lock bar too (criterion 32), hidden on a read only page.
test("a selected fixture, job or set, shows the Lock bar alone", () => {
  const app = source("App.tsx");
  assert.match(app, /selectedLock \? \(\s*<FixtureLockBar/);
  assert.match(app, /!fixtures\.view\?\.selected \? \(\s*<SelectToolbar/);
  assert.match(source("components/FixtureLockBar.tsx"), /aria-pressed=\{locked\}/);
});

test("no new colour: the tab uses the design tokens only", () => {
  for (const file of COMPONENTS) {
    assert.doesNotMatch(code(file), /#[0-9a-fA-F]{3,8}\b|rgb\(|hsl\(/, file);
  }
});
