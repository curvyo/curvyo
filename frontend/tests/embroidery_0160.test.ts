// `specs/0160-embroidery` slice 1 in the front end: the embroidery panel drawn from the session's
// view (Machine, Hoop, Operations, Output with the play bar; laser rows and font controls absent),
// the Add page group listing the embroidery profiles, the play helpers, and the browser session's
// embroidery page (needs `npm run build:wasm`). Rendered on the server through the vite harness.

import assert from "node:assert/strict";
import { existsSync, readFileSync } from "node:fs";
import path from "node:path";
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

const SRC = path.resolve(import.meta.dirname, "..", "src");
const BINDINGS = path.join(SRC, "wasm-bindings");
const WASM = path.join(BINDINGS, "curvyo_editor_wasm_bg.wasm");
const skipWasm = { skip: !existsSync(WASM) };
const noop = () => undefined;

async function render(module: string, name: string, props: Record<string, unknown>): Promise<string> {
  const react = await import("react");
  const dom = await import("react-dom/server");
  const tooltip = await server.ssrLoadModule("/src/components/ui/tooltip.tsx");
  const loaded = await server.ssrLoadModule(module);
  const element = react.createElement(
    tooltip.TooltipProvider as never,
    null,
    react.createElement(loaded[name] as never, props),
  );
  return dom.renderToString(element as never);
}

const text = (html: string) => html.replace(/<[^>]+>/g, " ").replace(/\s+/g, " ");

const API = {
  pick: () => true,
  saveAs: () => "",
  setSpeed: () => "",
  setHoop: () => true,
  press: () => "",
  removeSelection: () => true,
  remove: () => true,
  setNumber: () => "",
  setThread: () => true,
  play: noop,
  hsvOf: () => [0, 0, 0] as const,
  rgbOf: () => 0,
};

type Session = {
  add_page_of_kind: (kind: string, profile: string) => string;
  embroidery_view_json: () => string;
  embroidery_profiles_json: () => string;
  set_embroidery_hoop: (key: string) => boolean;
  set_embroidery_play: (position: number) => string;
  toggle_toolpath_view: (page: string) => boolean;
  manufacturing_view_json: () => string;
};

async function open(): Promise<Session> {
  const bindings = (await import(path.join(BINDINGS, "curvyo_editor_wasm.js"))) as {
    initSync: (options: { module: BufferSource }) => unknown;
    WasmSession: new (peer: bigint) => Session;
  };
  bindings.initSync({ module: readFileSync(WASM) });
  return new bindings.WasmSession(160n);
}

const props = (view: unknown) => ({
  view,
  api: API,
  onExport: noop,
  onToggleToolpath: noop,
  onSelectItems: noop,
  onReturnFocus: noop,
});

test("criteria 1 to 3: a new embroidery page has the W6 EU7, its hoops and the 150 x 250 field", skipWasm, async () => {
  const s = await open();
  assert.notEqual(s.add_page_of_kind("curvyo.embroidery", "w6-eu7"), "");
  const view = JSON.parse(s.embroidery_view_json());
  assert.equal(view.profile.name, "W6 EU7");
  assert.equal(view.hoop.key, "150x250");
  assert.match(JSON.parse(s.manufacturing_view_json()).status, /190\.0 × 290\.0 mm/);
  assert.ok(s.set_embroidery_hoop("100x100"));
  assert.equal(JSON.parse(s.embroidery_view_json()).profile.hoop, "100 × 100 mm");
  const profiles = JSON.parse(s.embroidery_profiles_json()) as { name: string }[];
  assert.deepEqual(
    profiles.map((p) => p.name),
    ["W6 EU7"],
  );
});

test("criteria 5, 12, 13: the panel has the three kinds, the hoop and the output, no laser or font rows", skipWasm, async () => {
  const s = await open();
  const page = s.add_page_of_kind("curvyo.embroidery", "");
  assert.ok(s.toggle_toolpath_view(page));
  const html = await render("/src/components/workbench/EmbroideryPanel.tsx", "EmbroideryPanel", props(JSON.parse(s.embroidery_view_json())));
  for (const word of ["Machine", "Hoop", "150 × 250 mm", "100 × 100 mm", "Speed", "Max stitch", "12.1 mm", "DST", "Running stitch", "Tatami fill", "Satin", "Export job file", "Tajima DST", "Nothing runs"]) {
    assert.match(text(html), new RegExp(word), word);
  }
  assert.match(html, /role="radiogroup" aria-label="Hoop"/);
  assert.doesNotMatch(html, /Laser mode|Air assist|Max power|Font|Mat grid/, "laser rows and font controls absent");
});

test("criterion 13: the play bar shows the stitch readout and the speeds, 16x first chosen", async () => {
  const view = {
    id: "1@1",
    page: "p",
    profile: { id: "w6-eu7", name: "W6 EU7", output: "DST", speed: 600, maxStitch: "12.1 mm", hoop: "150 × 250 mm" },
    profiles: [],
    hoop: { key: "150x250", sizes: [] },
    operations: [],
    kinds: [],
    checks: [],
    total: { time: "About 1 min", runs: "Runs 1 operation, 1 item.", stitches: "600 stitches · 1 colour · 0 trims · thread 1.5 m" },
    machineLine: "W6 EU7 · DST",
    exportAllowed: true,
    fileType: "Tajima DST",
    toolpath: true,
    play: { position: 12, length: 640, text: "10 of 600", colour: "#C8102E", colourText: "1 of 1" },
    stitchesPerSecond: 10,
    notice: null,
  };
  const html = await render("/src/components/workbench/EmbroideryPanel.tsx", "EmbroideryPanel", props(view));
  assert.match(html, /aria-label="Run position"/);
  assert.match(text(html), /10 of 600/);
  assert.match(html, /role="radiogroup" aria-label="Speed"/);
  assert.match(html, /aria-checked="true"[^>]*>16×/);
  const lib = await server.ssrLoadModule("/src/lib/embroideryView.ts");
  const at = lib.positionAt as (p: number, s: number, x: number, sps: number, l: number) => number;
  // Sixty frames of 1/60 s at 1x: the machine's 600 stitches per minute, 10 in one second.
  let position = 0;
  for (let frame = 1; frame <= 60; frame += 1) position = at(0, frame / 60, 1, 10, 640);
  assert.equal(position, 10, "1x");
  assert.equal(at(0, 1, 4, 10, 640), 40, "4x");
  assert.equal(at(0, 1, 16, 10, 640), 160, "16x");
  assert.equal(at(600, 1, 16, 10, 640), 640, "clamped at the end");
  assert.equal((lib.hexOf as (n: number) => string)(0xc8102e), "#C8102E");
});

test("criterion 1: the Add page group lists the embroidery profiles for Embroidery", async () => {
  const kinds = [
    { id: "drawing", label: "Drawing", icon: "PenTool", workbench: "vector" },
    { id: "curvyo.embroidery", label: "Embroidery", icon: "Spool", workbench: "manufacturing" },
  ];
  const html = await render("/src/components/panel/AddPageGroup.tsx", "AddPageGroup", {
    kinds,
    profiles: [],
    familyProfiles: { "curvyo.embroidery": [{ id: "w6-eu7", name: "W6 EU7", width: 190, height: 290, builtin: true, chosen: true }] },
    onAdd: noop,
    onCancel: noop,
  });
  assert.match(text(html), /Embroidery/);
});
