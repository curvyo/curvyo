// `specs/0114-image-trace-mask` in the frontend (criteria 1 and 25, UX notes §1, §2, §8): the Mask
// group sits in the Parts row of Advanced behind the trash, never in the first row, holds
// Rectangle, Lasso, Subtract, Inside | Outside and Clear mask, pressed as the session says; a
// locked image makes them unavailable; with several images the unit is not drawn; and a part's
// settings go to the session in the stored shape of ADR 0061 §5 and come back.

import assert from "node:assert/strict";
import { after, before, test } from "node:test";

type Server = {
  ssrLoadModule(url: string): Promise<Record<string, unknown>>;
  close(): Promise<void>;
};
let server: Server;

before(async () => {
  const { createServer } = await import("vite");
  server = (await createServer({
    configFile: new URL("../vite.config.ts", import.meta.url).pathname,
    server: { middlewareMode: true },
    appType: "custom",
    logLevel: "silent",
  })) as unknown as Server;
});

after(async () => {
  await server.close();
});

const noop = () => undefined;

async function barHtml(overrides: Record<string, unknown> = {}): Promise<string> {
  const react = await import("react");
  const dom = await import("react-dom/server");
  const bar = await server.ssrLoadModule("/src/components/TraceBar.tsx");
  const tooltip = await server.ssrLoadModule("/src/components/ui/tooltip.tsx");
  const text = await server.ssrLoadModule("/src/lib/traceText.ts");
  const presets = text.TRACE_PRESETS_FALLBACK as { label: string }[];
  const choice = (text.firstChoice as (p: unknown) => { values: Record<string, unknown> })(presets);
  const palette = { swatches: [], found: null, joined: 0, excluded: () => false, toggleExclude: noop, remove: noop, pick: noop, note: "", sameNote: "", refresh: noop };
  const trace = {
    active: true, available: true, selection: "images", mask: { regions: 0, outside: false }, setMaskOutside: noop,
    maskTool: "none", setMaskTool: noop, subtract: false, setSubtract: noop, maskLocked: false, multiImage: false, clearMask: noop,
    parts: [{ id: 0, regions: 0, outside: false, whole: true, muted: false, active: true, contributes: true, mask: "" }],
    selectPart: noop, addPart: noop, removePart: noop, imagesLabel: "", params: text.TRACE_PARAMS_FALLBACK, presets,
    values: { ...choice.values, threshold: 128 }, choice, presetLabel: presets[0].label, advancedOpen: true,
    status: text.NO_STATUS, countsStale: false, slot: { kind: "none" }, retrace: noop, creating: false,
    setValue: noop, pick: noop, setAdvancedOpen: noop, endDrag: noop, stop: noop, create: noop, keyDown: () => false,
    setMethod: noop, setColourTolerance: noop, setBackgroundAuto: noop, pickMode: "", setPicking: noop, removePicked: noop,
    suggestedBackground: null, analysis: { kind: "none" }, analyse: noop, announcement: "", traceBlocked: "",
    palette,
    ...overrides,
  };
  const info = { available: true, active: false, shown: false, choose: noop, toggle: noop, dismiss: noop };
  const element = react.createElement(bar.TraceBar as never, { trace, info, onImport: noop, onReturnFocus: noop } as never);
  return dom.renderToString(react.createElement(tooltip.TooltipProvider as never, {} as never, element) as never);
}

function pressed(html: string, data: string): string | undefined {
  const tag = new RegExp(`<button[^>]*${data}=""[^>]*>`).exec(html)?.[0] ?? "";
  return /aria-pressed="(true|false)"/.exec(tag)?.[1];
}

/** The first row of the bar: everything before the Advanced block. */
function firstRow(html: string): string {
  const row = /<div[^>]*data-trace-row=""[\s\S]*$/.exec(html)?.[0] ?? "";
  const advanced = row.indexOf('role="group" aria-label="Advanced"');
  return advanced < 0 ? row : row.slice(0, advanced);
}

test("criteria 1 and 25: no Mask group in the first row, open or closed; no Pick mask paths anywhere", async () => {
  for (const advancedOpen of [false, true]) {
    const html = await barHtml({ advancedOpen });
    assert.ok(!firstRow(html).includes("data-trace-mask"), `the first row has no Mask group (Advanced ${advancedOpen ? "open" : "closed"})`);
    assert.ok(!html.includes("Pick mask paths"));
  }
  assert.ok(!(await barHtml({ advancedOpen: false })).includes("data-trace-mask"), "closed: the unit is not shown");
});

test("criterion 1: the Parts row holds the chips, Add part, the trash, then Rectangle, Lasso, Subtract, Inside | Outside, Clear mask", async () => {
  const html = await barHtml();
  const unit = /<div[^>]*data-trace-unit=""[\s\S]*$/.exec(html)?.[0] ?? "";
  const order = [
    'data-trace-part="1"',
    'data-trace-add-part=""',
    'data-trace-remove-part=""',
    'aria-label="Rectangle"',
    'aria-label="Lasso"',
    'aria-label="Subtract"',
    '>Inside<',
    '>Outside<',
    'aria-label="Clear mask"',
  ].map((needle) => unit.indexOf(needle));
  assert.ok(order.every((at) => at >= 0), `every control is there: ${order}`);
  assert.deepEqual([...order].sort((a, b) => a - b), order, "in this order");
  assert.ok(unit.indexOf('aria-label="Analyse"') < 0 || unit.indexOf("data-trace-analyse") > order[8], "Analyse after the unit");
  // The Mask group's glyphs: Square for Rectangle, Lasso for Lasso (UX notes §1).
  const tag = (data: string) => new RegExp(`<button[^>]*${data}=""[\\s\\S]*?</button>`).exec(html)?.[0] ?? "";
  assert.match(tag("data-trace-mask-rect"), /lucide-square\b/);
  assert.match(tag("data-trace-mask-lasso"), /lucide-lasso\b/);
  assert.doesNotMatch(tag("data-trace-mask-lasso"), /lasso-select/);
});

test("criterion 2 and UX notes §1: the toggles are pressed as the session says", async () => {
  const html = await barHtml();
  assert.equal(pressed(html, "data-trace-mask-rect"), "false");
  assert.equal(pressed(html, "data-trace-mask-lasso"), "false");
  assert.equal(pressed(html, "data-trace-mask-subtract"), "false");
  const lasso = await barHtml({ maskTool: "lasso", subtract: true });
  assert.equal(pressed(lasso, "data-trace-mask-rect"), "false");
  assert.equal(pressed(lasso, "data-trace-mask-lasso"), "true");
  assert.equal(pressed(lasso, "data-trace-mask-subtract"), "true");
  const rect = await barHtml({ maskTool: "rect" });
  assert.equal(pressed(rect, "data-trace-mask-rect"), "true");
  assert.equal(pressed(rect, "data-trace-mask-lasso"), "false");
});

test("UX notes §8: a locked image makes the Mask group, Add part and the trash unavailable; several images hide the unit", async () => {
  const locked = await barHtml({ maskLocked: true });
  for (const data of ["data-trace-mask-rect", "data-trace-mask-lasso", "data-trace-mask-subtract", "data-trace-clear-mask", "data-trace-add-part"]) {
    const tag = new RegExp(`<button[^>]*${data}=""[^>]*>`).exec(locked)?.[0] ?? "";
    assert.match(tag, /aria-disabled="true"/, data);
  }
  assert.match(locked, /role="radiogroup" aria-label="Mask" aria-disabled="true"/);
  const several = await barHtml({ multiImage: true });
  assert.ok(!several.includes("data-trace-unit") && !several.includes("data-trace-mask"), "no Parts and Mask unit");
  assert.ok(several.includes('data-trace-analyse=""'), "the rest of Advanced stays");
});

test("the toggles say their state politely and show no hint text", async () => {
  const text = await server.ssrLoadModule("/src/lib/traceText.ts");
  const say = text.maskToggleStatus as (name: string, on: boolean) => string;
  assert.equal(say("Rectangle", true), "Rectangle on");
  assert.equal(say("Lasso", true), "Lasso on");
  assert.equal(say("Subtract", false), "Subtract off");
  assert.equal(text.MASK_FAILED, "0 paths: the mask paths could not be combined.");
  assert.equal(text.TRACE_MIXED, undefined, "the mixed-selection row is gone");
});

test("ADR 0061 §5: the bar's settings in the stored shape", async () => {
  const text = await server.ssrLoadModule("/src/lib/traceText.ts");
  const settings = await server.ssrLoadModule("/src/lib/traceMaskSettings.ts");
  const presets = text.TRACE_PRESETS_FALLBACK as { label: string }[];
  const choice = (text.firstChoice as (p: unknown) => { preset: string; edited: boolean; values: Record<string, unknown> })(presets);
  const json = (settings.partSettingsJson as (c: unknown) => string)({
    ...choice,
    values: { ...choice.values, method: text.PICK_COLOURS, output: text.MULTICOLOUR, swatches: ["#FF0000"], background: "#ABCDEF" },
  });
  const read = JSON.parse(json) as Record<string, unknown>;
  assert.equal(read.method, "pick_colours");
  assert.equal(read.output, "multi");
  assert.deepEqual(read.swatches, ["#ff0000"]);
  assert.equal(read.background, "#abcdef");
  assert.equal(read.preset, choice.preset);
  assert.equal(Object.keys(read).length, 24);
});

test("slice 3: a part's stored settings read back to the same choice; automatic values are stored as Auto", async () => {
  const text = await server.ssrLoadModule("/src/lib/traceText.ts");
  const settings = await server.ssrLoadModule("/src/lib/traceMaskSettings.ts");
  const presets = text.TRACE_PRESETS_FALLBACK as { label: string }[];
  type Choice = { preset: string; edited: boolean; values: Record<string, unknown> };
  const choice = (text.firstChoice as (p: unknown) => Choice)(presets);
  const edited: Choice = {
    ...choice,
    edited: true,
    values: { ...choice.values, threshold: 140, smooth: 25, method: text.REMOVE_BACKGROUND, backgroundAuto: false, background: "#ABCDEF", output: text.MULTICOLOUR, colours: 5 },
  };
  const json = (settings.partSettingsJson as (c: unknown) => string)(edited);
  const back = (settings.readPartChoice as (j: string, b: unknown) => Choice | null)(json, choice);
  assert.deepEqual(back, edited, "the same choice");
  assert.equal((settings.readPartChoice as (j: string, b: unknown) => Choice | null)("", choice), null, "an untouched image stores none");
  const stored = JSON.parse((settings.storedPartSettingsJson as (c: unknown, m: boolean) => string)({ ...edited, values: { ...edited.values, backgroundAuto: true } }, false)) as Record<string, unknown>;
  assert.equal(stored.threshold, null, "a Threshold the maker did not set is Auto");
  assert.equal(stored.background, null, "Auto background is read again from the picture");
  const kept = JSON.parse((settings.storedPartSettingsJson as (c: unknown, m: boolean) => string)(edited, true)) as Record<string, unknown>;
  assert.equal(kept.threshold, 140, "the maker's Threshold stays");
});
