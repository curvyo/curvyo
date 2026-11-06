// The Trace bar after the cleanup of 2026-11-06 (`specs/0089` UX notes §24): rendered on the server
// through the vite harness with a stub of the tool's state; the DOM order of the focusable
// controls is the Tab order.

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
  const palette = {
    swatches: [], found: null, joined: 0, selected: null, select: noop, state: () => "normal", isolated: null, merging: null,
    isolate: noop, recolour: noop, startMerge: noop, mergeInto: noop, cancelMerge: noop, toggleExclude: noop, remove: noop,
    pick: noop, pickedOnly: false, note: "", sameNote: "", refresh: noop,
  };
  const trace = {
    active: true, available: true, selection: "images", mask: { paths: 0, outside: false }, setMaskOutside: noop,
    maskPick: false, setMaskPick: noop, maskRefused: false, clearMask: noop,
    parts: [{ id: 1, paths: 0, outside: false, whole: true, muted: false, active: true, contributes: true, mask: "" }],
    selectPart: noop, addPart: noop, removePart: noop, imagesLabel: "", params: text.TRACE_PARAMS_FALLBACK, presets,
    values: { ...choice.values, threshold: 128 }, choice, presetLabel: presets[0].label, advancedOpen: false,
    status: text.NO_STATUS, countsStale: false, slot: { kind: "none" }, retrace: noop, creating: false, pxPerMm: 4,
    setValue: noop, pick: noop, setAdvancedOpen: noop, endDrag: noop, stop: noop, create: noop, keyDown: () => false,
    setMethod: noop, setColourTolerance: noop, setBackgroundAuto: noop, pickMode: "", setPicking: noop,
    suggestedBackground: null, analysis: { kind: "none" }, analyse: noop, announcement: "", traceBlocked: "",
    palette, hsvOf: () => [0, 0, 0], rgbOf: () => 0,
    ...overrides,
  };
  const info = { available: true, active: false, shown: false, choose: noop, toggle: noop, dismiss: noop };
  const element = react.createElement(bar.TraceBar as never, { trace, info, onImport: noop, onReturnFocus: noop } as never);
  return dom.renderToString(react.createElement(tooltip.TooltipProvider as never, {} as never, element) as never);
}

/** The names of the Tab stops in DOM order: buttons and radios with tabIndex 0, spin buttons. */
function stops(html: string): string[] {
  const names: string[] = [];
  for (const m of html.matchAll(/<(button|div|input)\b([^>]*)>([^<]*)/g)) {
    const attrs = m[2];
    if (/tabindex="-1"/.test(attrs) || /\shidden(=""|\s|$)/.test(attrs)) {
      continue;
    }
    const role = /role="([^"]+)"/.exec(attrs)?.[1] ?? "";
    if (m[1] === "div" && role !== "spinbutton" && role !== "slider") {
      continue;
    }
    const name = /aria-label="([^"]+)"/.exec(attrs)?.[1] ?? m[3].trim();
    names.push(role === "radio" ? `radio:${name}` : name);
  }
  return names;
}

test("§24.1, §24.4: no View pill; the Mask group is drawn with no mask path, never dimmed", async () => {
  const html = await barHtml();
  assert.doesNotMatch(html, /aria-label="View"/, "no View pill");
  for (const name of ["Overlay", "Image"]) {
    assert.doesNotMatch(html, new RegExp(`aria-label="${name}"`));
  }
  const group = /<div data-trace-mask=""[^>]*>([\s\S]*?)<span role="status"/.exec(html)?.[1] ?? "";
  assert.match(group, /aria-label="Pick mask paths"/);
  assert.match(group, /role="radiogroup" aria-label="Mask"/);
  assert.match(group, /aria-label="Clear mask"/);
  assert.doesNotMatch(group, /aria-disabled|opacity-50/, "nothing dimmed");
  assert.match(group, /role="radio" aria-checked="true" tabindex="0"[^>]*>Inside</, "Inside on entry");
  assert.match(await barHtml({ mask: { paths: 0, outside: true } }), /role="radio" aria-checked="true" tabindex="0"[^>]*>Outside</, "Outside selectable without a path");
});

test("§24.2: the bar's Tab order: i, Preset, Pick mask paths, Mask, Clear mask, Method, fields, expander, Trace", async () => {
  const names = stops(await barHtml());
  const at = (n: string) => names.indexOf(n);
  const order = ["About Trace", "Preset", "Pick mask paths", "radio:Inside", "Clear mask", "Method", "Threshold", "Smooth", "Tolerance", "Ignore below", "More trace options", "Trace"];
  const positions = order.map(at);
  assert.ok(positions.every((p) => p >= 0), `every stop is present: ${JSON.stringify(names)}`);
  assert.deepEqual([...positions].sort((a, b) => a - b), positions, `in this order: ${JSON.stringify(names)}`);
  assert.equal(names.filter((n) => n.startsWith("radio:")).length, 1, "Inside / Outside is one stop");
});

test("§24.3: in Advanced the Parts group with its label follows Corners, then Analyse last", async () => {
  const html = await barHtml({ advancedOpen: true });
  const corners = html.indexOf("Corners");
  const parts = html.indexOf('data-trace-parts=""');
  const analyse = html.indexOf('data-trace-analyse=""');
  assert.ok(corners >= 0 && corners < parts && parts < analyse, "Corners, Parts, Analyse");
  assert.match(html, /<span aria-hidden="true"[^>]*>Parts<\/span>/, "the muted label, hidden from the reader");
  assert.match(html, /role="radiogroup" aria-label="Parts"/);
  const names = stops(html);
  assert.deepEqual(names.slice(-3), ["radio:Part 1", "Add part", "Analyse"].map((n) => names.find((x) => x.startsWith(n)) ?? n), JSON.stringify(names));
});

test("§24.5: no analysis line, no status line; one hidden region, Advanced open or closed", async () => {
  for (const advancedOpen of [false, true]) {
    const html = await barHtml({
      advancedOpen,
      announcement: "Custom · Line drawing. 7 paths, 412 nodes, 0.4 s. 12 shapes set",
      analysis: { kind: "custom", analysis: {} },
      status: { line: "", muted: "", counts: "7 paths, 412 nodes, 0.4 s", countsMuted: "", message: "", canCreate: true },
    });
    assert.doesNotMatch(html, /data-trace-counts|data-trace-analysis=/);
    const hidden = [...html.matchAll(/<div data-trace-hidden-status="" role="status" aria-live="polite" class="sr-only">([^<]*)<\/div>/g)];
    assert.equal(hidden.length, 1, "one hidden region");
    assert.equal(hidden[0][1], "Custom · Line drawing. 7 paths, 412 nodes, 0.4 s. 12 shapes set");
    const visible = html.replace(hidden[0][0], "");
    assert.doesNotMatch(visible, /412 nodes|shapes set|Line drawing/, "nothing shows it");
  }
});

test("§24.5: the reasons Trace is dimmed and a failed analysis are notice-row lines", async () => {
  const blocked = await barHtml({ status: { line: "", muted: "", counts: "", countsMuted: "", message: "No colours found.", canCreate: false }, traceBlocked: "No colours found." });
  assert.match(blocked, /data-trace-reason=""[^>]*>No colours found\.</);
  const failed = await barHtml({ analysis: { kind: "failed" } });
  assert.match(failed, /data-trace-analysis-failed=""[^>]*>Analysis failed · automatic Threshold used</);
});
