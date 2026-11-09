// The picked colours beside the pipette (`specs/0089` UX notes §25.3, §25.4): the row's keys and
// focus rule, its strings, and the row rendered on the server through the vite harness with a stub
// of the tool's state (pick order, names, one Tab stop, no label, no notice when nothing is picked).

import assert from "node:assert/strict";
import { after, before, test } from "node:test";

import { focusAfterRemoval, pickedKey } from "../src/lib/tracePickedColours.ts";
import { NO_COLOUR_PICKED_LINE, pickSwatch, pickedAnnouncement, removePickedLabel, removedPickedText, statusText, readSummary, zeroPathsLine, PICK_COLOURS } from "../src/lib/traceText.ts";

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

test("§25.4: Left / Right move without wrapping, Home / End jump; Delete and Backspace remove", () => {
  assert.deepEqual(pickedKey("ArrowRight", 0, 3), { move: 1 });
  assert.deepEqual(pickedKey("ArrowRight", 2, 3), { move: 2 });
  assert.deepEqual(pickedKey("ArrowLeft", 0, 3), { move: 0 });
  assert.deepEqual(pickedKey("Home", 2, 3), { move: 0 });
  assert.deepEqual(pickedKey("End", 0, 3), { move: 2 });
  assert.deepEqual(pickedKey("Delete", 1, 3), { remove: 1 });
  assert.deepEqual(pickedKey("Backspace", 1, 3), { remove: 1 });
  // Enter and Space press the button, whose click removes it: the key itself does nothing more.
  assert.equal(pickedKey("Enter", 1, 3), null);
  assert.equal(pickedKey(" ", 1, 3), null);
  assert.equal(pickedKey("a", 1, 3), null);
});

test("§25.4: after a key removal focus moves to the next swatch, else the previous, else the pipette", () => {
  assert.equal(focusAfterRemoval(1, 3), 1, "the next one takes the place");
  assert.equal(focusAfterRemoval(2, 3), 1, "the last removed: the previous");
  assert.equal(focusAfterRemoval(0, 1), null, "none left: the pipette");
});

test("§25.4: strings: the name says what a press does, the region says what happened", () => {
  assert.equal(removePickedLabel("#3366CC"), "Remove #3366CC");
  assert.equal(removedPickedText("#3366CC"), "Removed #3366CC");
  assert.equal(pickedAnnouncement("#3366CC", false), "Removed #3366CC");
  assert.equal(pickedAnnouncement("#3366CC", true), `Removed #3366CC. ${NO_COLOUR_PICKED_LINE}`);
  assert.equal(pickedAnnouncement(null, true), NO_COLOUR_PICKED_LINE);
  assert.equal(pickedAnnouncement(null, false), "");
});

test("§25.4: new picks go to the end, a click replaces, the ninth is ignored", () => {
  const eight = ["#000001", "#000002", "#000003", "#000004", "#000005", "#000006", "#000007", "#000008"];
  assert.deepEqual(pickSwatch(["#000001"], "#000002", true), ["#000001", "#000002"]);
  assert.deepEqual(pickSwatch(eight, "#000009", true), eight);
  assert.deepEqual(pickSwatch(eight, "#000009", false), ["#000009"]);
});

test("§25.3: nothing picked has no '0 paths' line: the notice row says nothing", () => {
  assert.equal(zeroPathsLine({ method: PICK_COLOURS, swatches: [] }), "");
  const nothing = statusText(readSummary([0, 0, 0, 0, 0, 0, 0]), 0.1, 100, false, zeroPathsLine({ method: PICK_COLOURS, swatches: [] }));
  assert.deepEqual([nothing.line, nothing.message, nothing.canCreate], ["", "", false]);
});

const noop = () => undefined;

async function barHtml(overrides: Record<string, unknown> = {}, values: Record<string, unknown> = {}): Promise<string> {
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
    pick: noop, note: "", sameNote: "", refresh: noop,
  };
  const trace = {
    active: true, available: true, selection: "images", mask: { paths: 0, outside: false }, setMaskOutside: noop,
    maskPick: false, setMaskPick: noop, maskRefused: false, clearMask: noop,
    parts: [{ id: 1, paths: 0, outside: false, whole: true, muted: false, active: true, contributes: true, mask: "" }],
    selectPart: noop, addPart: noop, removePart: noop, imagesLabel: "", params: text.TRACE_PARAMS_FALLBACK, presets,
    values: { ...choice.values, threshold: 128, method: text.PICK_COLOURS, ...values }, choice, presetLabel: presets[0].label, advancedOpen: false,
    status: text.NO_STATUS, countsStale: false, slot: { kind: "none" }, retrace: noop, creating: false,
    setValue: noop, pick: noop, setAdvancedOpen: noop, endDrag: noop, stop: noop, create: noop, keyDown: () => false,
    setMethod: noop, setColourTolerance: noop, setBackgroundAuto: noop, pickMode: "", setPicking: noop, removePicked: noop,
    suggestedBackground: null, analysis: { kind: "none" }, analyse: noop, announcement: "", traceBlocked: "",
    palette, hsvOf: () => [0, 0, 0], rgbOf: () => 0,
    ...overrides,
  };
  const info = { available: true, active: false, shown: false, choose: noop, toggle: noop, dismiss: noop };
  const element = react.createElement(bar.TraceBar as never, { trace, info, onImport: noop, onReturnFocus: noop } as never);
  return dom.renderToString(react.createElement(tooltip.TooltipProvider as never, {} as never, element) as never);
}

/** The picked colours group's markup. */
function group(html: string): string {
  const start = html.indexOf('data-trace-colours="picked"');
  assert.ok(start >= 0, "the Pick colours group is drawn");
  const open = html.lastIndexOf("<div", start);
  let depth = 0;
  const re = /<\/?div\b[^>]*>/g;
  re.lastIndex = open;
  for (let m = re.exec(html); m; m = re.exec(html)) {
    depth += m[0].startsWith("</") ? -1 : 1;
    if (depth === 0) {
      return html.slice(open, m.index + m[0].length);
    }
  }
  return html.slice(open);
}

test("§25.4: the picked colours are buttons beside the pipette, in pick order, one Tab stop", async () => {
  const html = group(await barHtml({}, { swatches: ["#3366CC", "#FF0000", "#00AA00"] }));
  assert.match(html, /role="group" aria-label="Picked colours"/);
  const names = [...html.matchAll(/aria-label="(Remove #[0-9A-F]{6})"/g)].map((m) => m[1]);
  assert.deepEqual(names, ["Remove #3366CC", "Remove #FF0000", "Remove #00AA00"]);
  const stops = [...html.matchAll(/data-trace-picked-swatch="[^"]*"[^>]*tabindex="(-?\d)"|tabindex="(-?\d)"[^>]*data-trace-picked-swatch/g)].map((m) => m[1] ?? m[2]);
  assert.deepEqual(stops, ["0", "-1", "-1"], "one Tab stop, roving");
  assert.ok(html.indexOf("data-trace-pipette") < html.indexOf("data-trace-picked-swatch"), "the pipette comes first");
  assert.doesNotMatch(html, /picked<|None picked|data-trace-picked-count/, "no count label");
  assert.doesNotMatch(html, /min-w-/, "the group is as wide as its content, nothing reserved (customer 2026-11-09)");
});

test("§25.3: with nothing picked: no swatch, no Tab stop, no label, no notice; Trace dimmed with its reason", async () => {
  const html = await barHtml({ traceBlocked: NO_COLOUR_PICKED_LINE }, { swatches: [] });
  const picked = group(html);
  assert.doesNotMatch(picked, /data-trace-picked-swatch/);
  assert.doesNotMatch(picked, /None picked/);
  assert.doesNotMatch(html, /data-trace-reason/, "no notice-row line");
  const notice = html.slice(html.indexOf("data-trace-state"), html.indexOf("data-trace-hidden-status"));
  assert.doesNotMatch(notice, /no colour picked/, "the notice row does not say it");
  assert.match(html, /data-trace-slot="none" aria-disabled="true" aria-describedby="[^"]+"/);
  assert.match(html, new RegExp(`<span id="[^"]+" hidden="">${NO_COLOUR_PICKED_LINE.replace(".", "\\.")}</span>`));
});

test("§25.1: Ignore below has no pixel readout", async () => {
  const html = await barHtml({}, { method: 0 });
  assert.doesNotMatch(html, /&lt; 1 px|~ \d+ px/);
});
