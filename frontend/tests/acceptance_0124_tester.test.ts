// Tester acceptance tests for specs/0124-gradient-fill, the UI and repository criteria that the
// source states: the Fill paint group's four symbol-only items with their tooltips (1), the
// Gradient block's buttons, symbols and tooltips and no slider or Opacity field (3), no rail
// tool or top bar control (1), the decoders reading the shared view fixtures (3, 6, 7), and the
// docs that state the reversal (26). Written from the specification and the UX notes.

import assert from "node:assert/strict";
import { existsSync, readFileSync, readdirSync } from "node:fs";
import test from "node:test";

const read = (p: string) => readFileSync(new URL(p, import.meta.url), "utf8");
const fill = read("../src/components/panel/FillSection.tsx") + read("../src/components/panel/styleOptions.tsx");
const block = read("../src/components/panel/GradientBlock.tsx");
const entry = read("../src/components/GradientEntryBox.tsx");
const rail = read("../src/components/ToolRail.tsx");
const design = read("../../docs/design-system.md");
const requirements = read("../../docs/requirements.md");
const svgImport = read("../../docs/svg-import.md");

test("c1 the Fill paint group has four items with their tooltips", () => {
  for (const tip of ["No fill", "Solid fill", "Pattern fill", "Gradient fill"]) {
    assert.ok(fill.includes(`"${tip}"`), `tooltip ${tip}`);
  }
});

test("c1 there is no gradient rail tool", () => {
  assert.ok(!/gradient/i.test(rail), "ToolRail names no gradient");
});

test("c3 the block has Kind Linear/Radial, Add stop, Reverse and Remove stop with their symbols", () => {
  for (const tip of ['"Linear"', '"Radial"', '"Add stop"', '"Reverse"', '"Remove stop"']) {
    assert.ok(block.includes(tip), tip);
  }
  for (const icon of ["Trash2", "Plus", "ArrowLeftRight", "LinearGradientIcon", "RadialGradientIcon"]) {
    assert.ok(block.includes(icon), icon);
  }
});

test("c3 no slider and no Opacity field in the block or the entry box", () => {
  const code = (src: string) => src.replace(/\/\*[\s\S]*?\*\//g, "").replace(/\/\/.*$/gm, "");
  for (const [name, src] of [["block", code(block)], ["entry", code(entry)]] as const) {
    assert.ok(!/type=["']range["']/.test(src), `${name}: no range input`);
    assert.ok(!/Slider/.test(src), `${name}: no Slider`);
    assert.ok(!/Opacity/.test(src), `${name}: no Opacity field`);
  }
});

test("c3 the angle field shows the degree unit inside", () => {
  assert.ok(/°/.test(block) || /deg/.test(block), "the angle unit");
});

test("c26 the docs state the reversal", () => {
  const row = requirements.split("\n").find((l) => l.includes("R-EDIT-006")) ?? "";
  assert.ok(/gradient/i.test(row) && row.includes("0124"), row);
  assert.ok(/pattern/i.test(row) && row.includes("0071"), row);
  assert.ok(design.includes("Gradient (`0124`)"), "design-system row");
  assert.ok(/linearGradient|radialGradient/.test(svgImport), "svg-import.md names the gradients");
  assert.ok(!/middle colour/.test(svgImport) || /no longer|not any more|removed/i.test(svgImport), "old sentence replaced");
});

test("c14 gradient_middle is removed from svg-core's paint.rs", () => {
  const paint = read("../../curvyo-svg-core/src/paint.rs");
  assert.ok(!paint.includes("gradient_middle"), "gradient_middle is gone");
});

test("c3 c6 c7 the shared view fixtures decode", async () => {
  const dir = new URL("../../curvyo-editor-wasm/tests/fixtures/gradient_view/", import.meta.url);
  assert.ok(existsSync(dir), "fixtures exist");
  const mod = await import("../src/lib/gradientView.ts");
  const names = readdirSync(dir).filter((n) => n.endsWith(".json"));
  assert.ok(names.length >= 4, names.join(","));
  for (const n of names) {
    const json = JSON.parse(readFileSync(new URL(n, dir), "utf8"));
    const view = mod.decodeGradientView(json.view);
    const handles = mod.decodeGradientHandles(json.handles);
    assert.ok(view !== null, `${n} view decodes`);
    assert.ok(handles !== null, `${n} handles decode`);
    // A damaged value is refused, not half read.
    assert.equal(mod.decodeGradientView({ ...json.view, stops: "x" }), null, `${n} damaged view`);
    assert.equal(mod.decodeGradientHandles({ ...json.handles, handles: [{ kind: 3 }] }), null, `${n} damaged handles`);
  }
});
