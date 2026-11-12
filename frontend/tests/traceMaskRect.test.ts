// `specs/0114-image-trace-mask` slice 1 in the frontend: the Mask group holds the Rectangle and
// Subtract toggles, pressed as the session says, and the bar's settings reach the session in the
// stored shape of ADR 0061 §5.

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
    active: true, available: true, selection: "images", mask: { paths: 0, regions: 0, outside: false }, setMaskOutside: noop,
    maskTool: "none", setMaskTool: noop, subtract: false, setSubtract: noop,
    maskPick: false, setMaskPick: noop, maskRefused: false, clearMask: noop,
    parts: [{ id: 0, paths: 0, regions: 0, outside: false, whole: true, muted: false, active: true, contributes: true, mask: "" }],
    selectPart: noop, addPart: noop, removePart: noop, imagesLabel: "", params: text.TRACE_PARAMS_FALLBACK, presets,
    values: { ...choice.values, threshold: 128 }, choice, presetLabel: presets[0].label, advancedOpen: false,
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

test("criterion 2: the Mask group holds Rectangle and Subtract, named, pressed as the session says", async () => {
  const html = await barHtml();
  const group = /<div data-trace-mask=""[\s\S]*?<span role="status"/.exec(html)?.[0] ?? "";
  assert.match(group, /aria-label="Rectangle"/);
  assert.match(group, /aria-label="Subtract"/);
  assert.ok(group.indexOf('aria-label="Rectangle"') < group.indexOf('aria-label="Subtract"'));
  assert.equal(pressed(html, "data-trace-mask-rect"), "false");
  assert.equal(pressed(html, "data-trace-mask-subtract"), "false");
  const on = await barHtml({ maskTool: "rect", subtract: true });
  assert.equal(pressed(on, "data-trace-mask-rect"), "true");
  assert.equal(pressed(on, "data-trace-mask-subtract"), "true");
});

test("the toggles say their state politely and show no hint text", async () => {
  const text = await server.ssrLoadModule("/src/lib/traceText.ts");
  const say = text.maskToggleStatus as (name: string, on: boolean) => string;
  assert.equal(say("Rectangle", true), "Rectangle on");
  assert.equal(say("Subtract", false), "Subtract off");
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
