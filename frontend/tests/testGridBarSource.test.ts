// Structure pins of the Test grid tool's UI (`specs/0112` `ux-notes.md` 3.1, 4.1, 4.4): read from
// the sources like the other component contracts (no DOM here; Part H runs Chromium).
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

function source(file: string): string {
  return readFileSync(new URL(`../src/${file}`, import.meta.url), "utf8");
}

test("the bar is a toolbar of two rows with plain fields: no slider, no caption in row 1", () => {
  const bar = source("components/TestGridBar.tsx");
  assert.match(bar, /role="toolbar"/);
  assert.match(bar, /data-testgrid-row="1"/);
  assert.match(bar, /data-testgrid-row="2"/);
  assert.doesNotMatch(bar, /type="range"|<Slider|ui\/slider/);
  assert.match(bar, /label="Kind"/);
  assert.match(bar, /\$\{caption\} parameter/);
  assert.match(bar, /aria-label="Crosshatch"/);
});

test("the material dropdown lists New material first and filters above 8", () => {
  const menu = source("components/TestGridMaterialMenu.tsx");
  assert.match(menu, /New material/);
  assert.match(menu, /role="menuitemradio"/);
  assert.match(menu, /MATERIAL_FILTER_ABOVE/);
  assert.ok(menu.indexOf("New material") < menu.indexOf("filtered.map"));
});

test("the rail button has no letter key and is dimmed with 'Laser pages only.' off laser pages", () => {
  const rail = source("components/ToolRail.tsx");
  const at = rail.indexOf('tool="test-grid"');
  assert.ok(at > 0);
  const button = rail.slice(at, rail.indexOf("/>", at));
  assert.doesNotMatch(button, /shortcut=/);
  assert.match(button, /Laser pages only\./);
  assert.match(button, /aria-label|name="Test grid tool"/);
});

test("the hooks are the only callers of the grid and material bindings", () => {
  for (const file of ["hooks/useEditorSession.ts", "App.tsx"]) {
    assert.doesNotMatch(source(file), /set_test_grid_|test_grid_press|judge_test|set_test_photo/, file);
  }
  assert.match(source("hooks/useTestGridTool.ts"), /set_test_grid_field/);
  assert.match(source("hooks/useMaterials.ts"), /judge_test/);
});
